# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- OpenSpec root: `openspec init` plus the thirteen core capabilities imported from [`subx-cli`](https://github.com/jim60105/subx-cli) — 96 requirements / 254 scenarios reproduced verbatim (see `openspec/changes/archive/2026-09-06-import-core-specs/`), their `openspec/specs/` file history carried by an unrelated-histories merge of `subx-cli`'s filtered spec history, and thirteen hand-written `## Purpose` paragraphs (twelve carried, `core-reporting`'s new). Spec-only: no code, test, API or dependency change.
- `ComponentFactory::match_config()` (the `MatchConfig` the factory's loaded `Config` implies, with the four caller-controlled fields documented for mutation) and `ComponentFactory::create_match_engine_with(config)` (engine from a caller-chosen config under the same propagation terms as `create_match_engine`). Minor-version additions.
- `CollectedFiles::default_output_dir()` and `CollectedFiles::default_output_path()` — the archive-aware output-location resolution rules (convert's and translate's, verbatim) as public queries. Minor-version additions.
- `subx_core::core::matcher::engine::apply_archive_origin_relocation(operations, files)` — the archive-extracted-subtitle forced-copy rewrite as a named core behaviour (MUST run before `apply_unique_target_paths`). Minor-version addition.
- `ProgressEvent::{Started, Advanced, Finished}` — structured batch-stream variants (`done: u64`, `Advanced::item: Option<&str>` for the subtitle the unit just completed). `ProgressEvent` stays `#[non_exhaustive]`, which is precisely why adding variants is a minor-version change: downstream wildcard arms are mandatory and keep compiling.
- `Reporter::cancelled()` — provided-method cooperative-cancellation query (default `false`), so a UI or CLI attached to an engine can stop a batch between operations. Minor-version addition.
- `subx_core::core::sync::shift_subtitle_timing(subtitle, offset_seconds)` — the manual-offset timing transform as a free function, reachable without constructing a `SyncEngine` (whose `new` requires a VAD detector even for manual-offset-only callers). It deliberately does not enforce `sync.max_offset_seconds`. `SyncEngine::apply_manual_offset` now delegates to it after its own guard, so exactly one implementation of the shift exists.

### Changed
- `MatchEngine::execute_operations_audit` now polls the attached reporter's `cancelled()` between operations: on cancellation it stops, pads the remaining `OperationOutcome` slots with `{ applied: false, error: None }` (the one-outcome-per-operation arity contract holds), closes its progress stream with `Finished { done, total }` where `done < total`, and returns `Ok` — cancellation never surfaces as `Err`. Both execution loops additionally emit `Started`/`Advanced`/`Finished` progress events (dry runs emit nothing). With no reporter attached, outcomes are unchanged.
- Handoff record from the C2a import, for the capabilities' next editors: `local-llm-provider`'s three references to `configuration-management`/`error-handling` are qualified "in `subx-cli`" and become `subx-core`'s when proposal C2b splits those capabilities; `subtitle-parser-hardening`'s `fuzz/` and quality-script citations name `subx-core` paths deliberately; and the `spec-governance` capability (which stays in `subx-cli`) is the rule set every future move is checked against. The drift check `spec-governance` specifies is not implemented here; C3 gives it a home in the quality scripts, and `subx-core/AGENTS.md` (not yet existing) inherits the two-root rules C3 authors.
- Open question for the next API review: whether `MatchConfig` should become `#[non_exhaustive]` (mirroring `ProgressEvent`) so future field additions stop breaking the exhaustive literals callers like `cache_command.rs` still write. Left open deliberately in this change.
- `SubtitleFormat` declares `Send + Sync` supertraits, which makes `Box<dyn SubtitleFormat>` thread-safe and, in turn, `FormatManager`, `FormatConverter` and `TranslationEngine` `Send + Sync`; the guarantee is asserted at compile time by the `thread_safety` module in `src/core/mod.rs`. In semver terms this is a `trait_added_supertrait` **major** change; it lands inside the 1.0.0 surface rather than against it — the release below has not been cut yet (no `v*` tag, `cargo publish` runs later from the tag), so when 1.0.0 ships this entry folds into it and the break has no audience.

## [1.0.0] - 2026-09-06

First publication of `subx-core`: the core library extracted from [`subx-cli`](https://github.com/jim60105/subx-cli) — the `config`, `core`, `error` and `services` trees migrated at their identical relative paths with `git filter-repo` history, so `git log`/`git blame` cross the repository boundary. The crate is a Cargo workspace member of `subx-cli`, mounted there as a git submodule, and standalone-buildable (no workspace inheritance, no `[workspace]`/`[profile]` tables). Public surface: `subx_core::{config, core, error, services}` plus the twelve `#[macro_export]` configuration test macros, deliberately identical to the pre-split `subx_cli::` paths with only the crate name swapped. `test-support` gates `src/test_support/` so no release artifact compiles it. Licence: GPL-3.0-or-later. Published to crates.io from `subx-cli`'s release workflow via `cargo publish --workspace` — not from this repository.

### Added
- Own CI: `.github/workflows/build-test-audit-coverage.yml` — three-OS `test` job (`scripts/quality_check.sh ci` through bash), a `security` job auditing this repository's own `Cargo.lock` (not the superproject's workspace union), and an ubuntu `coverage` job uploading LCOV with the same `--ignore-filename-regex` exclusion set as `subx-cli`'s script. The coverage job deliberately enforces no threshold: workspace-derived floors do not apply to a standalone run (design.md Decision 9).
- `scripts/quality_check.sh` — the deliberately small local gate (fmt, `check --all-features`, lib-only clippy `-D warnings`, `cargo doc`, doctests, `nextest --profile "${1:-default}" --features slow-tests`); the superproject's script remains the authority for paired changes.
- Standalone coverage baseline recorded so a future standalone floor can be derived from a measurement: **90.62%** line coverage (19,028/20,998, single instrumented `ci`-profile run with the reporting exclusions applied), 1,332 tests green.


### Added
- `tests/input_handler_tests.rs` and `tests/parallel_integration_tests.rs`: two never-compiled test files revived from `subx-cli`'s `tests/cli/` and `tests/parallel/` subdirectories (they had no `#[path]` shim and were therefore built by no target). The input-handler tests compare collected file lists through `CollectedFiles::into_paths()`; the parallel tests name `WorkerPool` through its home module (`parallel::worker`), and their `ConvertFormat` leg was dropped — see Removed.

### Removed
- `tests/sync/integration_tests.rs` (moved in from `subx-cli`'s orphan tree, then deleted under the revive-or-delete rule): every API it drives is gone — the `core::sync::dialogue::DialogueDetector` module it imports does not exist, `SyncEngine` exposes `detect_sync_offset`/`apply_manual_offset` where the test calls `sync_subtitle`/`apply_sync_offset`, `SyncConfig` no longer has the four-field shape the test constructs, the free function `services::audio::generate_dialogue_audio` was never migrated (the `AudioMockGenerator::generate_dialogue_audio` method is a different API), and `TestConfigBuilder` has no `with_dialogue_detection`/`with_sync_threshold`. Reviving it would mean authoring that production surface, so task 4.7 deletes it. What it asserted (dialogue-detection sync workflows and sync-quality assessment over hand-built `Subtitle` trees) is recorded here so its coverage is not silently lost.
- The `ConvertFormat` leg of `tests/parallel_integration_tests.rs`: `FileProcessingTask::convert_format` is a stub that returns the input path unchanged without writing the requested output, so the leg's "output file exists" assertion cannot pass without a production change — and pinning the stub would invert the test into a guard for the defect. Restore the leg when the stub becomes a real conversion.

### Changed
- The `subx-cli` back-compatibility re-exports (`subx_cli::config`/`core`/`error`/`services`/`Result` and the twelve test macros) survive this change and now have zero in-repository consumers; their removal is a future major-version deletion, not a rewrite. Twelve never-compiled test files, the runtime binary-name lookups, and the per-crate coverage floors are handed to `harden-split-test-suite` (B4).

### Added
- `src/test_support/` behind the `test-support` feature (workspace builder, file managers, mock OpenAI / Azure OpenAI helpers, response generators), shared with `subx-cli`'s test suite through a dev-dependency feature so no release artifact sees it. Own `tests/` (40 core-bound integration tests relocated from `subx-cli`), `benches/` with their `[[bench]]` tables, `tests/fixtures/formats/` (22 byte-identical parser fixtures), and `assets/` media (mp4/mp3 move, srt copy).

- The crate now contains the configuration (`config/`), core-engine (`core/`),
  error (`error.rs`) and services (`services/`) modules, migrated from
  `subx-cli` with their per-file history: the `subx-cli` history of those four
  paths was extracted with `git filter-repo` and merged into this branch with
  `--allow-unrelated-histories`, so `git log`/`git blame` work across the
  repository boundary (this branch consequently has two roots).
- Public paths are deliberately identical to the pre-split `subx_cli::` paths
  with only the crate name swapped — including the redundant `core::` segment
  (`subx_core::core::matcher::MatchEngine`) — so a downstream consumer's
  migration is a pure crate-name substitution. The decision is recorded in
  the crate-level rustdoc.
- Feature gates that were inert while the sources lived in `subx-cli` are now
  real here: `archive-rar = ["dep:unrar"]` gates the RAR extraction path, and
  `slow-tests` gates the long-running format round-trip tests.
- The twelve `#[macro_export]` configuration test macros defined in
  `src/config/test_macros.rs` are reachable at this crate's root
  (`subx_core::test_with_config!` and friends), as `#[macro_export]` places
  macros at the defining crate's root.
- `config::service::read_config_value_from` is public: its other caller is
  the `subx-cli` `config get` command, which now crosses the crate boundary.

### Fixed
- `[profile.default.junit] path` no longer doubles the store prefix (was `target/nextest/junit.xml`, resolving to `target/nextest/default/target/nextest/junit.xml`); now `junit.xml` with the table beside `[profile.default]`, and every comment in English (closing the wording divergence B1 deliberately left with the parent).
- `.gitignore`: dropped the `target/nextest/*/junit.xml` pattern subsumed by `junit.xml` and `/target/`.

