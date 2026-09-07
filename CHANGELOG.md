# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed
- **The repository's default branch is now `master`** (renamed from `main`).
  The CI workflow's push/PR triggers, the pointer rule in `AGENTS.md`, and the
  superproject's `.gitmodules` referent and submodule-pointer assertions all
  follow the new name. No API or behaviour change for consumers; git-URL
  consumers that pin `branch = "main"` should switch to `branch = "master"`.

### Documentation
- README, `AGENTS.md` and the pending-release wording corrected against the
  review of the split changes: the standalone-clone contract, the own-tag
  release contract, and the CI job set now describe what actually exists.

## [1.0.0] - 2026-09-06

First release of `subx-core` — the subtitle processing engine behind
[`subx-cli`](https://github.com/jim60105/subx-cli), extracted into a
standalone library so programs — like the SubX GUI at
[`jim60105/subx`](https://github.com/jim60105/subx) — can embed subtitle
matching, format conversion, timeline synchronisation, translation and
configuration directly. A pure library: no binary, no terminal presentation
code.

This crate publishes to crates.io from **this repository's own release tag**;
until that tag is cut and its publication run completes, the crate is
consumable from its git URL (`subx-core = { git =
"https://github.com/jim60105/subx-core", version = "1.0" }`). The public
surface described below is frozen for the 1.0 release line.

### Added
- **The public modules `config`, `core`, `error` and `services`**, carrying
  the sources migrated from `subx-cli` at their identical relative paths —
  `subx_core::core::matcher::MatchEngine` was
  `subx_cli::core::matcher::MatchEngine`, including the redundant `core::`
  segment — so a consumer migrating from the CLI crate's re-exports performs
  a pure crate-name substitution.
- **The engine set:** subtitle parsing and conversion (SRT, ASS, VTT, SUB,
  with encoding detection), AI-powered subtitle matching, timeline
  synchronisation (voice-activity-driven and manual offset), AI-assisted
  translation, archive extraction, input collection, parallel batch
  processing, file organisation, and the configuration system with its
  dependency-injected `ConfigService` — all constructible through plain
  dependency injection or the `ComponentFactory`.
- **A transport-agnostic reporting seam** (`core::report`): attach a
  `Reporter` to any engine, factory, file manager, worker pool or AI
  provider client with its `with_reporter` builder and receive diagnostics,
  warnings, AI token usage and structured progress events
  (`ProgressEvent::{Started, Advanced, Finished}`) instead of terminal
  output. `NoopReporter` is the default sink. `Reporter::cancelled()` lets
  an attached application stop a batch cooperatively: a cancelled batch ends
  cleanly, reports which operations completed, and never surfaces
  cancellation as an error.
- **The error taxonomy** (`SubXError` with `category()`, `machine_code()`,
  `hint()`) that the CLI's JSON mode renders — embedding applications get
  the same machine-readable error contract.
- **Thread-safe format handling:** `SubtitleFormat` declares `Send + Sync`
  supertraits, making the format manager, converter and translation engine
  safe to share across threads (asserted at compile time).
- **`core::sync::shift_subtitle_timing`** — the manual-offset timing
  transform as a free function, reachable without constructing the sync
  engine (whose construction otherwise requires a voice-activity detector).
- **Factory and collection queries for embedders:**
  `ComponentFactory::match_config()` and
  `ComponentFactory::create_match_engine_with(config)` (an engine from a
  caller-modified match configuration), and `CollectedFiles`'
  `default_output_dir()` / `default_output_path()` (the archive-aware output
  location rules) as public queries.
- **Feature gates:** `archive-rar` (optional RAR extraction support — the
  real gate; `subx-cli`'s flag is a pass-through), `slow-tests` (long
  round-trip format tests), and `test-support`, which exposes
  `subx_core::test_support::*` — the shared test workspace builder,
  configuration builder and mock AI-provider helpers — for consumers' test
  suites. `test-support` is never a shipping default and stays out of the
  published documentation site.
- The twelve configuration test macros (`subx_core::test_with_config!` and
  friends) are reachable at the crate root for consumer test suites.
- **Preserved provenance:** the four module trees were migrated out of
  `subx-cli` with their per-file history (`git filter-repo` +
  `--allow-unrelated-histories` merge), so `git log` / `git blame` cross the
  repository boundary. The crate is standalone-buildable — no workspace
  inheritance, no `[workspace]` table — so a plain `git clone` followed by
  `cargo build` always works.

### Documentation
- `README.md` and `AGENTS.md` written for standalone library consumers: the
  two consumers of the crate, the supported standalone-clone workflow, the
  feature flags, and the module guide including the reporting seam.

[Unreleased]: https://github.com/jim60105/subx-core/compare/v1.0.0...HEAD
[1.0.0]: https://github.com/jim60105/subx-core/releases/tag/v1.0.0
