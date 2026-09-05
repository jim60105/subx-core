# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
