# Subtitle Matching

## Purpose

Define the core match engine: AI-based file pairing over `AnalysisRequest`, confidence-threshold filtering, dry-run versus live execution, the `FileRelocationMode` relocation model, backup-before-move, per-scan UUIDv7 file identifiers, AI-driven language and globally-unique target naming with the archive-origin relocation function, and the structured JSON operation payload. The `subx-cli` repository carries the same-named `subtitle-matching` capability holding the command-surface half: the `match` flag surface and preconditions, mutual-exclusion validation, and the call-ordering obligation between archive-origin relocation and uniqueness allocation. Implemented in `src/core/matcher/`.

## Requirements

### Requirement: Optional Backup Before Move

The system SHALL create a backup of the source subtitle file before moving it only when the relocation mode is `Move` and backups are enabled (enabled by `subx-cli`'s `--backup` flag or by `general.backup_enabled` in configuration). The system SHALL NOT create backups in rename-in-place or `Copy` modes.

#### Scenario: Backup before Move
- **GIVEN** the user runs `subx-cli`'s `subx match --move --backup <path>` and a matching subtitle is identified
- **WHEN** the engine executes the operation
- **THEN** a backup of the source subtitle SHALL be created before the file is moved to the video's directory

#### Scenario: No backup when only renaming in place
- **GIVEN** the user runs `subx-cli`'s `subx match --backup <path>` without `--copy` or `--move`
- **WHEN** the engine executes the rename-in-place operation
- **THEN** no backup task SHALL be scheduled

### Requirement: File Relocation Modes

The system SHALL model relocation as the `FileRelocationMode` values copy, move, and rename-in-place: copy and move relocate the matched subtitle alongside its paired video; the rename mode renames the subtitle in place. The mutually exclusive `--copy` (`-c`) / `--move` (`-m`) flag surface is `subx-cli`'s, specified by the `subtitle-matching` capability's *Match Command Argument Surface and Input Preconditions* requirement in `subx-cli`.

#### Scenario: Copy relocates matched subtitle
- **GIVEN** a video in directory `A/` and its matched subtitle in directory `B/`, and the user passes `--copy`
- **WHEN** the engine executes operations
- **THEN** the subtitle SHALL be copied into directory `A/` with a name derived from the video's base name, and the original subtitle in `B/` SHALL remain untouched

### Requirement: Confidence Threshold Enforcement

The system SHALL take a 0.0-1.0 confidence threshold supplied by the caller and discard any AI-proposed pair whose score falls below that threshold. The 0-100 percentage surface (`--confidence`, default 80, validated by `clap`) is `subx-cli`'s, specified by the `subtitle-matching` capability's *Match Command Argument Surface and Input Preconditions* requirement in `subx-cli`.

#### Scenario: Low-confidence pairs are filtered out
- **GIVEN** the caller supplies a 0.9 threshold (`subx-cli`'s `--confidence 90`) and the AI provider returns a candidate match with confidence 0.75
- **WHEN** the engine processes the AI response
- **THEN** the engine SHALL omit that candidate from the generated operations list

### Requirement: AI-Based File Pairing

The system SHALL use a configured AI provider to analyze video and subtitle file names (and optional subtitle content samples) and return candidate video-subtitle pairings with a per-pair confidence score.

#### Scenario: Successful match with sufficient confidence
- **GIVEN** a directory containing at least one video file and one subtitle file, and an AI provider is configured
- **WHEN** the user runs `subx-cli`'s `subx match <path>`
- **THEN** the engine SHALL collect eligible files via `InputPathHandler`, send an `AnalysisRequest` to the AI provider, and generate rename operations for every pair whose confidence is greater than or equal to the configured threshold

The `No files found to process` empty-input precondition is a `subx-cli` obligation, specified by the `subtitle-matching` capability's *Match Command Argument Surface and Input Preconditions* requirement in `subx-cli`.

### Requirement: Per-Scan Unique UUIDv7 File Identifiers for Matching

The system SHALL assign each discovered media file a per-scan unique identifier of the form `file_<uuid-v7-hyphenated>` (total length 41) generated through the shared `crate::core::uuidv7::Uuidv7Generator` with strict 1ms spacing. Identifiers SHALL be unique within a single discovery scan but SHALL NOT be guaranteed to remain stable across separate invocations of the binary, because UUIDv7 IDs are intrinsically time-based. The match pipeline SHALL reference video and subtitle files by these identifiers (rather than by filename) when sending requests to the AI provider and when correlating the AI response back to disk paths within the same invocation. Cross-invocation correlation (e.g., between the match cache and a later `cache apply` invocation of `subx-cli`) SHALL use canonical filesystem paths rather than identifiers. Implemented in `src/core/matcher/discovery.rs` (`generate_file_id`, `Uuidv7Generator` integration) and exercised by `tests/match_engine_id_integration_tests.rs`.

#### Scenario: All discovered files receive unique IDs

- **GIVEN** a directory containing several video and subtitle files, including entries with complex non-ASCII filenames
- **WHEN** `FileDiscovery::scan_directory` runs
- **THEN** every returned file SHALL have a non-empty `id` beginning with `file_` and of length 41, the embedded UUIDv7 version nibble SHALL equal `7`, and the full set of IDs SHALL be unique

#### Scenario: AI response is correlated via IDs within the same invocation

- **GIVEN** an AI provider returns `MatchResult.matches` entries referencing `video_file_id` and `subtitle_file_id` shaped as `file_<uuid-v7>`
- **WHEN** `MatchEngine::match_file_list` processes the response
- **THEN** the generated `MatchOperation` set SHALL resolve each ID back to the corresponding `MediaFile` and SHALL produce operations whose `video_file.id` and `subtitle_file.id` match the AI-supplied identifiers

#### Scenario: IDs are not stable across invocations

- **GIVEN** the same directory is scanned in two separate invocations of the binary
- **WHEN** `FileDiscovery::scan_directory` runs in each invocation
- **THEN** the returned `MediaFile.id` values for the same on-disk file MAY differ between the two invocations, AND any cross-invocation correlation SHALL be performed by canonical path rather than by ID

### Requirement: AI-Driven Language and Globally-Unique Target Naming

The system SHALL derive each renamed subtitle's filename by consuming the AI's optional `FileMatch.target_filename_suffix` and `FileMatch.language` fields when present, and SHALL guarantee that the operations list emitted for a single match batch contains no two operations whose final target path (parent directory plus filename, after any archive-origin or `--copy`/`--move` relocation) is identical — across the entire batch, not only within a single video.

The naming precedence in `MatchEngine::generate_subtitle_name` SHALL be:

1. If `FileMatch.target_filename_suffix` is `Some(value)` after sanitization (keep `[A-Za-z0-9_-]`, truncate to 16 characters, drop if empty) and language-code normalization, use `<video_base>.<value>.<subtitle_extension>`.
2. Else if `FileMatch.language` is `Some(value)` after the same sanitization and normalization, use `<video_base>.<value>.<subtitle_extension>`. The literal value `und` (case-insensitive) SHALL collapse to "no language tag".
3. Else if `LanguageDetector::get_primary_language(&subtitle.path)` returns `Some(code)`, use `<video_base>.<code>.<subtitle_extension>`.
4. Else use `<video_base>.<subtitle_extension>`.

Language-code normalization SHALL pass the AI value through `LanguageDetector`'s code map so that synonyms (`english`/`eng`/`EN` → `en`; `cht`/`繁中`/`traditional-chinese` → `tc`; `chs`/`简中` → `sc`) collapse to the canonical short code; unrecognized but otherwise valid `[A-Za-z0-9_-]{1,16}` tokens SHALL be passed through verbatim after lower-casing.

The archive-origin forced relocation SHALL be a named public function of `subx-core`, not inline command logic:

```rust
// src/core/matcher/engine.rs
pub fn apply_archive_origin_relocation(
    operations: &mut [MatchOperation],
    collected: &CollectedFiles,
);
```

For each operation whose `subtitle_file.path` has an `archive_origin` in `collected` and whose `requires_relocation` is still `false`, it SHALL set `relocation_target_path` to the matched video's parent directory joined with `new_subtitle_name`, set `requires_relocation` to `true`, and set `relocation_mode` to `FileRelocationMode::Copy`. An operation whose subtitle did not come from an archive, or which already requires relocation, SHALL be left untouched, and an operation whose video path has no parent SHALL be left untouched. It SHALL live beside `apply_unique_target_paths` in the same module, and each function's rustdoc SHALL name the other, because the two are only correct when called in order.

Once every operation already carries its final relocation target — a **precondition on the allocator's input**, since a function cannot require its caller to have done something first — the engine SHALL run a deterministic global uniqueness allocator. The obligation that `subx-cli`'s match command completes every archive-origin forced rewrite before invoking the allocator exactly once is specified by the `subtitle-matching` capability's *Match Command Applies Archive-Origin Relocation Before Uniqueness Allocation* requirement in `subx-cli`.

1. Sort operations ascending by `(target_directory, subtitle_file.relative_path)` for stability across reruns.
2. Maintain a `claimed: HashSet<PathBuf>` of already-allocated final target paths.
3. For each operation, take its candidate final path; while the candidate is in `claimed`, replace the filename's pre-extension portion with `<base>.<n>.<ext>` (or `<base>.<lang>.<n>.<ext>` if a language segment is present), where `n` is the smallest integer ≥ 2 not yet used for that base.
4. Insert the resolved path into `claimed` before processing the next operation.

The allocator SHALL produce stable results across reruns of the same input set and SHALL probe past pre-existing files-on-disk patterns (e.g. an input that already contains `movie.2.srt`) when those filenames appear among the candidates.

Neither function SHALL be folded into the other, and `apply_unique_target_paths`' signature SHALL NOT gain a `CollectedFiles` parameter: the allocator is meaningful for operations that never touched an archive, and its current arity is part of the published library surface.

#### Scenario: AI-supplied suffix wins over filename heuristic
- **GIVEN** a `FileMatch` whose `target_filename_suffix` is `Some("tc")` and whose subtitle path is `subs/movie.srt` (no language tag detectable from the path)
- **WHEN** `MatchEngine::generate_subtitle_name` runs against video `movie.mkv`
- **THEN** the produced name SHALL be `movie.tc.srt`

#### Scenario: AI-supplied language used when no suffix is present
- **GIVEN** a `FileMatch` whose `target_filename_suffix` is `None`, whose `language` is `Some("ja")`, and whose subtitle path has no detectable language tag
- **WHEN** `MatchEngine::generate_subtitle_name` runs against video `movie.mkv`
- **THEN** the produced name SHALL be `movie.ja.srt`

#### Scenario: AI language synonyms are normalized
- **GIVEN** a `FileMatch` whose `language` is `Some("english")` (or `"eng"`, or `"EN"`) and whose subtitle path has no detectable language tag
- **WHEN** `MatchEngine::generate_subtitle_name` runs against video `movie.mkv`
- **THEN** the produced name SHALL be `movie.en.srt` for every variant

#### Scenario: AI value `und` is treated as no language
- **GIVEN** a `FileMatch` whose `language` is `Some("und")` and whose `target_filename_suffix` is `None`, and whose subtitle path has no detectable language tag
- **WHEN** `MatchEngine::generate_subtitle_name` runs against video `movie.mkv`
- **THEN** the produced name SHALL be `movie.srt` (no language segment appended)

#### Scenario: Falls back to LanguageDetector when AI omits both fields
- **GIVEN** a `FileMatch` whose `target_filename_suffix` and `language` are both `None`, and whose subtitle path is `subs/movie.tc.srt`
- **WHEN** `MatchEngine::generate_subtitle_name` runs against video `movie.mkv`
- **THEN** the produced name SHALL be `movie.tc.srt`

#### Scenario: Sanitization rejects unsafe AI-supplied suffix
- **GIVEN** a `FileMatch` whose `target_filename_suffix` is `Some("../etc")` and whose subtitle path has no detectable language tag
- **WHEN** `MatchEngine::generate_subtitle_name` runs against video `movie.mkv`
- **THEN** the suffix SHALL be discarded by sanitization and the produced name SHALL fall through to the next precedence rule, yielding `movie.srt`

#### Scenario: Two duplicate-target operations get globally unique names
- **GIVEN** an AI response whose two `FileMatch` entries both pair the same video with subtitles that produce candidate target filename `movie.srt` (same parent directory)
- **WHEN** the engine assembles the operations list and runs the global uniqueness allocator
- **THEN** sorted by canonical relative subtitle path, the first SHALL keep target filename `movie.srt` and the second SHALL be renamed to `movie.2.srt`

#### Scenario: Three-way duplicates and a pre-existing numeric suffix
- **GIVEN** candidate target filenames `movie.srt`, `movie.srt`, and `movie.2.srt` for the same target directory (the third one is a pre-existing distinct candidate)
- **WHEN** the engine runs the global uniqueness allocator
- **THEN** the resolved names SHALL be `movie.srt`, `movie.3.srt`, and `movie.2.srt` (sorted by canonical path, the second clone probes past the already-claimed `movie.2.srt`)

#### Scenario: Language-disambiguated pairs do not need numeric suffixes
- **GIVEN** an AI response that pairs two subtitles with the same video and supplies distinct `language` values `"tc"` and `"sc"`
- **WHEN** the engine assembles the operations list
- **THEN** the produced names SHALL be `movie.tc.srt` and `movie.sc.srt` and the allocator SHALL NOT modify them

#### Scenario: Cross-video duplicates in the same target directory are disambiguated
- **GIVEN** two different videos `a.mkv` and `b.mkv` whose AI-paired subtitles both relocate — via `subx-cli`'s `--copy` handling — into a shared target directory and both happen to produce the same candidate filename `subs.srt` after step 1-4
- **WHEN** the engine runs the global uniqueness allocator over the operation set as rewritten by `subx-cli`'s match command
- **THEN** the two final target paths SHALL differ — sorted by canonical subtitle path, the first keeps `subs.srt` and the second becomes `subs.2.srt`

#### Scenario: Archive-extracted subtitle is forced to copy beside its video
- **GIVEN** a `CollectedFiles` in which the matched subtitle `/tmp/subx-XXXX/movie.srt` has `archive_origin` `/data/subs.zip`, an operation pairing it with `/media/movie.mp4`, and `requires_relocation == false`
- **WHEN** `apply_archive_origin_relocation(&mut operations, &collected)` is called
- **THEN** that operation SHALL have `requires_relocation == true`, `relocation_mode == FileRelocationMode::Copy`, and `relocation_target_path == Some(PathBuf::from("/media/movie.srt"))` — the video's parent directory, not the temporary extraction directory and not the archive's parent directory

#### Scenario: Directly supplied subtitle is left alone
- **GIVEN** a `CollectedFiles` in which the matched subtitle was supplied directly and has no `archive_origin`, and an operation with `requires_relocation == false`
- **WHEN** `apply_archive_origin_relocation(&mut operations, &collected)` is called
- **THEN** that operation SHALL be unchanged in `requires_relocation`, `relocation_mode` and `relocation_target_path`

