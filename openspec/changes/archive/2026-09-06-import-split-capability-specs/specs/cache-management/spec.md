## ADDED Requirements

### Requirement: Match Cache Location

The system SHALL persist the match cache as a JSON file at `$CONFIG_DIR/subx/match_cache.json`, where `$CONFIG_DIR` is resolved via the standard platform configuration directory (`dirs::config_dir()`).

The `subx-cli` half of this capability holds the agreement obligation for this location: its *Cache Clear Subcommand* requirement is the holder of the rule that the CLI's independent resolver must keep producing the identical path, including honouring the `XDG_CONFIG_HOME` override.

#### Scenario: Cache persisted after a match run
- **GIVEN** `subx match` executes successfully and produces at least one match operation
- **WHEN** the run finishes
- **THEN** a file at `$CONFIG_DIR/subx/match_cache.json` SHALL contain the serialized `CacheData` including file snapshot, match operations, and a configuration hash

### Requirement: Configuration-Aware Invalidation

The cache SHALL record the configuration hash and relocation settings under which its entries were generated so that changes in matching-relevant configuration invalidate prior results.

#### Scenario: Config hash stored with cache
- **GIVEN** a cache is written by `save_file_list_cache`
- **WHEN** the JSON file is inspected
- **THEN** it SHALL contain a `config_hash` field, an `original_relocation_mode` field, and an `original_backup_enabled` field reflecting the settings in effect during the producing run

### Requirement: Cache Reuse Preserves Relocation Mode

When a cached match result is reused on a subsequent `subx match` run with the same relocation mode, the system SHALL apply operations using the relocation mode and target directories that were in effect when the cache was produced, without issuing a new AI request. Implemented in `src/core/matcher/engine.rs` and exercised by `subx-cli:tests/match_cache_reuse_tests.rs` and `subx-cli:tests/match_cache_target_directory_tests.rs`.

#### Scenario: Copy mode cache reused without a second AI call
- **GIVEN** `subx match --copy <dir>` has populated the cache for a given set of files
- **WHEN** the user runs `subx match --copy <dir>` again on the same files and configuration
- **THEN** the cached match operations SHALL be reused, no additional AI provider call SHALL be made, and the subtitle SHALL be copied to the video's directory preserving the original subtitle at its source

#### Scenario: Target directory consistency between dry-run and actual execution
- **GIVEN** `subx match --copy --dry-run <dir>` has populated the cache for a given file set
- **WHEN** the user runs `subx match --copy <dir>` (actual execution) on the same files and configuration
- **THEN** the files written to disk SHALL land in the same target directories that the dry-run preview reported

### Requirement: Dry-Run Cache Reuse Without AI Calls

When a `subx match` invocation (including `--dry-run`) has populated the match cache for a given set of files, subsequent invocations with the same input files, relocation mode, backup setting, and configuration hash SHALL reuse the cached match operations and SHALL NOT call the AI provider again. Implemented in `src/core/matcher/engine.rs` (via `check_file_list_cache` / `save_file_list_cache`) and exercised by `subx-cli:tests/match_cache_reuse_tests.rs`.

#### Scenario: Repeated dry-run does not call the AI provider
- **GIVEN** the user has run `subx match --copy --dry-run <dir>` once against a mock AI provider and the cache has been populated
- **WHEN** the user runs the same `subx match --copy --dry-run <dir>` command a second time with no file or configuration changes
- **THEN** the mock AI provider SHALL have received exactly one chat-completion request in total across both invocations

### Requirement: Cache Invalidation On Relocation-Affecting Config Change

The `CacheData.config_hash` SHALL be derived from the configuration values that affect file-operation planning — specifically `relocation_mode` and `backup_enabled` — such that a change to either of those values on a subsequent run SHALL cause the cache entry to be treated as stale and a fresh AI call SHALL be performed. Implemented in `src/core/matcher/engine.rs::calculate_config_hash`.

#### Scenario: Relocation mode change invalidates cache
- **GIVEN** the cache was produced by a run with `relocation_mode = None`
- **WHEN** the user re-runs `subx match --copy <dir>` on the same files (relocation mode `Copy`)
- **THEN** the cached operations SHALL NOT be reused verbatim for the new relocation mode and the run SHALL either recompute or re-derive operations consistent with the new mode

### Requirement: Cache Scoped To File-List Directory Key

The cache SHALL be keyed on the sorted set of canonicalized input file paths (with size and modification time) combined with the configuration hash, so that cache reuse applies only when the same target file set is supplied on a subsequent run. Implemented by the `filelist_<hash>` key computed in `src/core/matcher/engine.rs` and exercised by `subx-cli:tests/match_cache_target_directory_tests.rs`.

#### Scenario: Different input file set misses the cache
- **GIVEN** a cache produced for file set `{A.mp4, A.srt}` in directory `videos/`
- **WHEN** the user runs `subx match` on a different file set `{B.mp4, B.srt}`
- **THEN** the new run SHALL compute a different cache key and SHALL NOT reuse the cached operations from the first file set

