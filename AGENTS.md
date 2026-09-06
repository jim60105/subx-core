# AGENTS.md

Instructions for AI coding agents working on **subx-core**.

## Test-suite split (mirrored rules)

- **Which crate does a test belong to?** It spawns the CLI binary or names
  `subx_cli::cli` / `subx_cli::commands` / `subx_cli::App` → it lives in the
  `subx-cli` repository. It names only library segments → `subx-core/tests/`.
- **`test-support` feature:** shared fixtures live in
  `subx_core::test_support`, gated by the `test-support` feature. This crate's
  own tests reach it through a path-only self dev-dependency; `subx-cli`
  enables it through its `[dev-dependencies]`. Never pass `--features
  test-support` on the command line, and never add the feature to a shipping
  default — `cargo build --release` must not compile the module.
- **`subx-core/tests/` is flat** (Cargo auto-discovers `tests/*.rs` only), so
  it has no `#[path]` shims; shared code comes from `test_support`, not a
  `tests/common/`. The `every_subdirectory_test_file_has_exactly_one_harness_shim`
  guard in the `subx-cli` repository (`subx-cli/tests/core_cli_boundary.rs`)
  walks this crate's `tests/` tree too and fails the build if a subdirectory
  ever holds a `.rs` file with zero or more than one declaring shim — never
  park a test file in a `tests/` subdirectory.
- **Revive or delete — never leave an orphan:** revive a dead test file when
  its imports already resolve (after rewriting to `subx_core::…` paths),
  delete it when reviving would require authoring production code that does
  not exist (record the deletion in `CHANGELOG.md`). When a revived test's
  assertion fails because behaviour legitimately changed, update the
  **assertion** to characterise current behaviour — never change production
  code to satisfy a test that has never executed.
- **Fixture and asset reads resolve from `env!("CARGO_MANIFEST_DIR")`**, never
  the working directory — the parser fixtures (`tests/fixtures/formats/`) and
  the media assets (`assets/`) ship in this repository.

## Project Overview

`subx-core` is the core subtitle processing library of the SubX project,
written in Rust (edition 2024). It provides subtitle parsing and format
conversion (SRT/ASS/VTT/SUB), AI-powered file matching, audio synchronization
via Voice Activity Detection (VAD), AI-assisted translation, encoding
detection, archive extraction, and a dependency-injected configuration
system — as a reusable library with **no binary target** and no terminal
presentation code.

- **Repository:** <https://github.com/jim60105/subx-core>
- **License:** GPL-3.0-or-later
- **Crate name:** `subx-core`
- **API reference:** <https://docs.rs/subx-core> (canonical; `subx-cli`'s
  library surface is compatibility re-exports of this crate)

Its two consumers are [`subx-cli`](https://github.com/jim60105/subx-cli),
the command-line front-end (which mounts this repository as a git submodule
and re-exports it), and the Tauri GUI at
[`jim60105/subx`](https://github.com/jim60105/subx), which depends on the
published crate. A library consumer SHALL depend on `subx-core` directly,
never on `subx-cli`. A standalone clone of this repository is a supported
workflow — it is exactly how crates.io, docs.rs, and the GUI consume the
crate — so nothing in this repository may depend on living inside the
`subx-cli` checkout.

## Repository Layout

This repository is consumed as a **git submodule** of
[`subx-cli`](https://github.com/jim60105/subx-cli), mounted at
`subx-cli/subx-core/`. `subx-cli` is the Cargo workspace root; `subx-core` is
one of its two workspace members. The contract that keeps both consumption
modes working:

- **Changes are committed here first.** A change that touches this library is
  committed in this repository, then the submodule pointer (gitlink) in
  `subx-cli` is bumped in a second commit. A `subx-cli` commit that depends on
  an unpushed or unpinned `subx-core` commit is non-conforming.
- **Every configuration file is this repository's own.** `.gitignore`,
  `.gitattributes`, `rustfmt.toml`, `.config/nextest.toml`,
  `.codegraph/.gitignore`, `LICENSE`, and the committed `Cargo.lock` apply to
  a standalone clone; the parent repository's copies never reach inside a
  submodule.
- **Clone/update commands** (for contributors working through `subx-cli`):
  `git clone --recurse-submodules https://github.com/jim60105/subx-cli` for a
  fresh clone; `git submodule update --init --recursive` to repair an existing
  clone; `git config submodule.recurse true` to keep the submodule working
  tree following the pointer on `git pull`/`git checkout` (per-clone setting;
  does not apply to `git clone`).

The `Cargo.toml` prohibitions that keep the standalone clone building (no
`[workspace]` table, no workspace inheritance, no `[profile.*]` tables) are
recorded in the shared conventions region below, verbatim in both
repositories' `AGENTS.md`.

## Continuous Integration

`.github/workflows/build-test-audit-coverage.yml` runs three jobs on every
push/PR to `main`:

| Job | What it does |
|---|---|
| `test` (ubuntu / windows / macOS) | `scripts/quality_check.sh ci` — fmt, `check --all-features`, lib-only clippy `-D warnings`, `cargo doc`, doctests, `nextest --profile ci --features slow-tests` — through bash on all three OSes (Windows runners ship bash), so one script defines the check set |
| `security` | `actions-rust-lang/audit@v1.2.7` on **this repository's own `Cargo.lock`** — core's own resolved graph, deliberately narrower than the superproject's workspace-union audit, which mixes in subx-cli-only dependencies no consumer of this crate ever resolves |
| `coverage` (ubuntu) | one instrumented `cargo llvm-cov nextest` run, LCOV uploaded to codecov with the same `--ignore-filename-regex` exclusion set as `subx-cli/scripts/check_coverage.sh` |

Two rules fall out of this:

- **The pointer is release load-bearing.** A `subx-cli` tag's release build
  compiles the gitlink commit, not `main`. The commit you point at must be
  this repository's `main` HEAD (or an ancestor of it) whose CI you have
  seen pass.
- **No coverage threshold gate exists here, on purpose.** The 90% core floor
  (and the 75% workspace / 82% CLI floors) are enforced in `subx-cli`'s
  scripts against the *workspace-attributed* numbers; the ~thousand CLI-side
  tests that drive core code from the other repository do not exist in this
  one, so a standalone gate must never borrow a workspace-derived floor — it
  needs a measurement of a standalone run (the percentage this job uploads to
  codecov). The first such measurement is recorded in `CHANGELOG.md`:
  **90.62%** lines (19,028/20,998, single instrumented `ci`-profile run).

There is no release workflow in this repository and none is planned: this
repository contains only `build-test-audit-coverage.yml`. crates.io
publication happens exclusively from `subx-cli`'s `publish-crates` job (a
single `cargo publish --workspace` triggered by a `subx-cli` `v*` tag, whose
`Assert the submodule pointer is on subx-core main` step makes a push of this
repository's `main` a precondition of any release).

## Architecture

### Module Guide

The library sources migrated from `subx-cli` at their identical relative
paths:

| Path | Owns |
|---|---|
| `src/config/` | The configuration system: `Config`, the `ConfigService` DI trait, `ProductionConfigService`, the `Test*` utilities, `field_validator`, `validator`, and the twelve `#[macro_export]` test macros (`test_macros.rs`, reachable at the crate root) |
| `src/core/` | The processing engines: `formats` (SRT/ASS/VTT/SUB + `encoding`), `matcher`, `sync`, `translation`, `parallel`, `archive`, `input`, `lock`, `file_manager`, `factory`, `language`, `uuidv7`, `fs_util` |
| `src/core/report/` | The `Reporter` seam: structured progress / AI-usage events (`ProgressEvent`) a host — the CLI or the GUI — renders; core never presents directly |
| `src/error.rs` | `SubXError`, its helper constructors and `From` conversions, and the machine-readable contract (`category`, `machine_code`, `hint`) |
| `src/services/` | External integrations: `ai` (providers, retry, security, error sanitizer), `audio`, `vad` |
| `src/test_support/` | Shared test fixtures (config builder, file managers, mock OpenAI/Azure helpers, response generators), gated by the `test-support` feature — never compiled into a release artifact |

There is no CLI layer and no command layer here; argument parsing, terminal
presentation, exit codes, and user-facing prose belong to the consumer's
binary.

Public module paths are frozen at their pre-split `subx_cli::` shapes (only
the crate name differs) — see the crate-level rustdoc in `src/lib.rs` before
reshaping anything.

### Dependency Direction — A One-Way Boundary

**`subx-core` SHALL NOT name `subx_cli` anywhere: not in code, not in a
`use`, not in a doc comment, and not in an intra-doc link.** No file under
`src/` may contain `subx_cli`, `crate::cli` or `crate::commands` on any
line, comments and doctests included. The dependency between the two
repositories points downward only (`subx-cli` depends on `subx-core`);
there is no dependency edge in the other direction and there never will
be, so an upward reference is *unfixable* rather than merely stale.

The intra-doc link case specifically is a hard build failure, not a style
rule: `subx-cli` is not a dependency of this crate, so under
`broken_intra_doc_links = "deny"` a `[subx_cli::...]` link resolves only
inside the workspace build and breaks the **standalone clone** build that
crates.io, docs.rs, and the GUI perform.

Enforcement: the `core_cli_boundary` guard test in the `subx-cli` repository
(`subx-cli/tests/core_cli_boundary.rs`) walks this crate's `src/` from
`CARGO_MANIFEST_DIR` and fails on any line containing those tokens, comments
included. Human-oriented output from core goes through the
`subx_core::core::report::Reporter` seam; the CLI renders it. Plain rustdoc
prose may mention the `subx-cli` binary by name (never as a link) where a
documented behaviour is written for that binary's terminal.

## Build, Test, and Quality Commands

Trust these instructions — only search the codebase if they are incomplete
or produce errors.

| Task | Command |
|---|---|
| Build | `cargo build` |
| Format | `cargo fmt` |
| Lint | `cargo clippy -- -D warnings` |
| Run tests | `cargo nextest run \|\| true` |
| Local quality gate | `scripts/quality_check.sh [profile]` (optional profile name: `default`, `ci`, `full`) |
| Doc build | `cargo doc --all-features --no-deps --document-private-items` |
| Doc tests | `cargo test --doc --all-features` |

### Important Notes

- **`scripts/quality_check.sh` is this repository's local gate** — fmt,
  `check --all-features`, lib-only clippy `-D warnings`, `cargo doc`,
  doctests, and `nextest --profile "${1:-default}" --features slow-tests`.
  It is a **strict subset** of the authoritative gate: the superproject's
  `subx-cli/scripts/quality_check.sh` is what actually decides a change, run
  there at the moment the submodule pointer is bumped. A green standalone
  run is necessary, never sufficient.
- **Use `cargo nextest run || true` for tests**, never `cargo test` (except
  for doc tests). The `|| true` prevents shell abort due to a known nextest
  issue in this project — **you must still inspect the output and treat any
  test failure as a real failure**.
- **Always run `cargo fmt` and `cargo clippy -- -D warnings`** and fix every
  warning before submitting code.
- Required tooling: Rust stable, `rustfmt`, `clippy`, `cargo-nextest`.

## Error Handling

- Use `SubXError` variants from `src/error.rs` — never invent ad-hoc error
  types.
- The machine contracts `category()`, `machine_code()` and `hint()` stay on
  `SubXError` in `src/error.rs`. Process/terminal presentation (exit codes,
  user-facing prose) belongs to the consumer's CLI layer, not this crate —
  render messages through `Display` here and never call binary-half
  presentation helpers.

## Cargo Features

- `default = []` — no optional features are enabled by default.
- `archive-rar = ["dep:unrar"]` — **this manifest owns the real gate**;
  `subx-cli`'s `archive-rar` is only a pass-through
  (`["subx-core/archive-rar"]`).
- `slow-tests = []` — gates long-running tests; `subx-cli`'s is again a
  pass-through.
- `test-support = ["dep:wiremock", "dep:hound"]` — exposes
  `src/test_support/` and pulls in the HTTP mock server and WAV writer only
  that module needs. `subx-cli` never declares it as a feature and enables
  it only through `[dev-dependencies]`.

<!-- SHARED_CONVENTIONS_START -->
<!-- The text between these two markers (inclusive) is byte-identical in
     subx-cli/AGENTS.md and subx-core/AGENTS.md. The superproject's quality
     gate enforces it: scripts/quality_check.sh --check-spec-governance
     (quality_check.ps1 -CheckSpecGovernance), exit code 2 on failure.
     Edit this region in BOTH repositories in the same change or the gate
     turns red. The tool-managed CODEGRAPH block lives outside the region
     in both files. -->

## Coding Conventions

### General Rules

- All code comments and rustdoc must be written in **English**.
- Do not introduce new `#[deprecated]` attributes. When removing
  functionality, delete the item and update all call sites. Some legacy
  fields in `SyncConfig` still carry `#[deprecated]` for backward
  compatibility — leave those as-is unless actively cleaning them up.
- Unimplemented code must be marked with `// TODO`. Unless requirements
  explicitly permit phased implementation, all TODOs must be resolved
  before submitting.
- Never parse or hand-edit `Cargo.lock` — it is managed by Cargo.
- Formatting: `rustfmt.toml` sets edition 2024 with max width 100 columns.

### Naming Conventions

- Modules: `snake_case`.
- Factory methods: `create_*` on `ComponentFactory`.

## Documentation Conventions

- Write rustdoc in **English** for all public APIs.
- Required sections for public functions: `# Arguments`, `# Returns`,
  `# Errors`, `# Examples`.
- Include `# Panics` and `# Safety` sections when applicable.
- All doc examples must compile — verified by `cargo test --doc --all-features`.
- Use intra-doc links: `` [`crate::module::Type`] ``. Broken links are
  denied (`broken_intra_doc_links = "deny"` in `Cargo.toml`).
- **Cross-crate rustdoc links are one-way**: the CLI's rustdoc may link into
  the core with absolute `subx_core::...` paths; core rustdoc SHALL NOT
  contain any bracketed `[subx_cli::...]` link. The CLI is not a core
  dependency, so the deny'd broken intra-doc link turns the standalone core
  documentation build into a build failure. Core documentation may mention
  CLI behaviour only as backticked prose (`subx_cli`) or a plain GitHub
  URL, never as an intra-doc link.
- **Verify the shared documentation boundary with
  `cargo doc --workspace --all-features`** — not `--no-deps`.
  `cargo doc --no-deps` documents only the local crates without building
  registry dependencies, so it can report success while generating no
  `subx_core` pages at all and leaving every re-export link pointing at a
  page that does not exist.

### Changelog Convention

Follow [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) with
[Semantic Versioning](https://semver.org/). Use sections: `### Added`,
`### Changed`, `### Fixed`, `### Removed`, `### Documentation`. Every
user-facing change (including CI/release behaviour) gets an entry in this
repository's `CHANGELOG.md` under the top `## [Unreleased]` header, and every
released entry lives under its own `## [VERSION]` header. `subx-cli`'s release
workflow parses that repository's `## [VERSION]` headers to generate release
notes — always add a properly formatted entry for every release.

## CPU-Intensive Operations — Main Agent Only

Running the full test suite or coverage check hogs CPU and interferes with
parallel work. Subagents and worker sessions MUST NOT run these themselves:

- `scripts/quality_check.sh` (every profile; the `.ps1` port exists in the
  `subx-cli` repository) — the full check suite
- `scripts/check_coverage.sh` / `.ps1` — the instrumented full-suite coverage
  run (`subx-cli` repository only)
- bare `cargo nextest run` without a `--filter-expr`

**Correct workflow:** subagents run only their own scoped tests with
`cargo nextest run --filter-expr 'test(module_name)' || true`. When a task
needs full-suite or coverage validation, request it from the main agent, who
runs it once after all sub-agent work is consolidated and before every
`git commit`. `cargo check` is fine for quick validation. CI generates
coverage reports on every push, so local coverage is usually unnecessary.

## subx-core Manifest Prohibitions

`subx-core/Cargo.toml` SHALL NOT contain a `[workspace]` table, workspace
inheritance (`version.workspace = true`, `authors.workspace = true`,
`<dep>.workspace = true`, `[lints] workspace = true`), or `[profile.*]`
tables. The superproject root manifest owns the `[profile.release]` settings
and the shared dependency versions; a workspace table or inheritance in the
member would stop `cargo build` from working in a standalone clone of the
library repository, which is a supported workflow (crates.io, docs.rs, and
the Tauri GUI all consume it that way).

## Testing Conventions

### Critical Rules

- **Always use `TestConfigService`** for configuration in tests. Never use
  `ProductionConfigService`.
- **Never modify global state** — no `std::env::set_var`, no `static mut`,
  no `Lazy<Mutex<_>>`, no writes outside `TempDir`.
- **All tests must be parallel-safe** and deterministic.
- **Async tests** use `#[tokio::test]`.

Repository-specific test infrastructure, test patterns, and test
organisation are the Test Infrastructure, Test Patterns, and Test
Organization sections immediately below this shared region.

<!-- SHARED_CONVENTIONS_END -->

### Test Infrastructure

| Helper | Location | Purpose |
|---|---|---|
| `TestConfigService` | `src/config/test_service.rs` | Isolated config without filesystem I/O |
| `TestConfigBuilder` | `src/config/builder.rs` | Fluent builder for test configs |
| `TestEnvironmentProvider` | `src/config/environment.rs` | In-memory env vars for isolated testing |
| `src/test_support/` | behind the `test-support` feature | Cross-repository shared fixtures: config builder, file managers, mock OpenAI/Azure helpers, response generators |

This crate's own integration tests reach `test_support` through the
path-only self dev-dependency (`subx-core = { path = "." }` under
`[dev-dependencies]`): an integration test links the library as an external
crate, which `#[cfg(test)]` never covers.

### Test Patterns

```rust
// Unit test with config
#[tokio::test]
async fn test_feature() {
    let config_service = TestConfigBuilder::new()
        .with_ai_provider("openai")
        .with_ai_model("gpt-4.1-mini")
        .build_service();
    let result = some_function(&*config_service).await;
    assert!(result.is_ok());
}
```

### Test Organization

- **Unit tests:** Inline `#[cfg(test)] mod tests` in source files.
- **Integration tests:** `tests/*.rs`, one file per feature area. The tree
  is **flat** — Cargo auto-discovers `tests/*.rs` only, so never park a test
  file in a `tests/` subdirectory (the superproject's shim guard fails the
  build if one appears).
- **Fixtures:** `tests/fixtures/` (parser fixtures under
  `tests/fixtures/formats/`) — every fixture and asset read resolves from
  `env!("CARGO_MANIFEST_DIR")`, never the working directory.
- **Shared helpers:** come from `subx_core::test_support`, not a
  `tests/common/`.
- **Benchmarks:** `benches/` using Criterion; registered benches are
  `retry_performance` and `file_id_generation_bench`.

## Configuration System

The configuration system uses dependency injection. Components receive
`&dyn ConfigService` — never read config files directly.

### Config Priority (highest → lowest)

1. Environment variables
2. User config file (`~/.config/subx/config.toml` on Linux/macOS,
   `%APPDATA%\subx\config.toml` on Windows)
3. Built-in defaults

### Supported Environment Variables

Provider-specific variables (checked first):

- `OPENAI_API_KEY`, `OPENAI_BASE_URL` — OpenAI provider
- `OPENROUTER_API_KEY` — OpenRouter provider
- `AZURE_OPENAI_API_KEY`, `AZURE_OPENAI_ENDPOINT`,
  `AZURE_OPENAI_API_VERSION` — Azure OpenAI provider
- `LOCAL_LLM_API_KEY`, `LOCAL_LLM_BASE_URL` — Local LLM provider (only
  honored when `ai.provider = "local"`).

General overrides with `SUBX_` prefix (e.g., `SUBX_AI_MODEL`,
`SUBX_GENERAL_WORKSPACE`). Note that env-var handling has special cases
in `src/config/service.rs` — check the implementation if a specific
override doesn't work as expected.

Workspace override: `SUBX_WORKSPACE` or `general.workspace` config changes
the working directory before the consumer dispatches a command.

### Config Sections

- `[ai]` — Provider, API key, model, base URL, retry, timeout (default
  provider: `openai`, default model: `gpt-4.1-mini`)
- `[formats]` — Output format, encoding, styling preservation
- `[sync]` / `[sync.vad]` — Sync method, VAD sensitivity, padding
- `[general]` — Backup, concurrency, timeout, workspace, progress bar
- `[parallel]` — Worker pool, overflow strategy, task queue

### Adding New Config Keys

New configuration keys must be added to all of the following:

1. `src/config/mod.rs` — struct field with serde attributes
2. `src/config/service.rs` — both `get_config_value()` and
   `set_config_value()`
3. `src/config/field_validator.rs` — field-level validation
4. `src/config/validator.rs` — section-level validation

The fifth checklist destination, the user-facing `docs/configuration-guide.md`,
lives in the `subx-cli` repository; see that repository's `AGENTS.md` for the
full five-destination rule.

## OpenSpec and Project Skills

`subx-core/openspec/` is an **independent OpenSpec root** that specifies this
crate. The parent repository's tooling never descends into the submodule:
`openspec validate`/`list` run at `subx-cli`'s root report only that root's
specs, and the same commands run from inside this directory resolve this
root. A change resolves against exactly one root — run its commands from
inside the repository that owns it. `openspec/config.yaml` here is
byte-identical to the unmodified `openspec init` template; do not hand-edit
it.

Twelve capability names (`cache-management`, `component-factory`,
`configuration-management`, `encoding-detection`, `error-handling`,
`format-conversion`, `input-path-handling`, `parallel-processing`,
`secrets-protection`, `subtitle-matching`, `subtitle-translation`,
`timeline-sync`) appear in **both** trees by design — each is one half of
one capability split along the crate boundary, recorded in
`subx-cli/openspec/split-capabilities.txt`. Work spanning both repositories
is authored as **two** changes, one per root, each `## Why` naming the other
as its other half. The rule set every move is checked against
(`spec-governance`) stays in `subx-cli`; the superproject's
`scripts/quality_check.sh --check-spec-governance` enforces the split record
and the byte-identical shared conventions region of these two `AGENTS.md`
files.

<!-- CODEGRAPH_START -->
## CodeGraph

In repositories indexed by CodeGraph (a `.codegraph/` directory exists at the repo root), reach for it BEFORE grep/find or reading files when you need to understand or locate code:

- **MCP tool** (when available): `codegraph_explore` answers most code questions in one call — the relevant symbols' verbatim source plus the call paths between them, including dynamic-dispatch hops grep can't follow. Name a file or symbol in the query to read its current line-numbered source. If it's listed but deferred, load it by name via tool search.
- **Shell** (always works): `codegraph explore "<symbol names or question>"` prints the same output.

If there is no `.codegraph/` directory, skip CodeGraph entirely — indexing is the user's decision.
<!-- CODEGRAPH_END -->
