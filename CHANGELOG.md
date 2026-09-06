# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `subx_core::core::sync::shift_subtitle_timing(subtitle, offset_seconds)` — the manual-offset timing transform as a free function, reachable without constructing a `SyncEngine` (whose `new` requires a VAD detector even for manual-offset-only callers). It deliberately does not enforce `sync.max_offset_seconds`. `SyncEngine::apply_manual_offset` now delegates to it after its own guard, so exactly one implementation of the shift exists.

### Changed
- `SubtitleFormat` declares `Send + Sync` supertraits, which makes `Box<dyn SubtitleFormat>` thread-safe and, in turn, `FormatManager`, `FormatConverter` and `TranslationEngine` `Send + Sync`; the guarantee is asserted at compile time by the `thread_safety` module in `src/core/mod.rs`. In semver terms this is a `trait_added_supertrait` **major** change; it lands inside the 1.0.0 surface rather than against it (the crate is not yet published, so the break has no audience).

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

