# AGENTS.md

Instructions for AI coding agents working on **subx-core**.

## Project Overview

`subx-core` is the core subtitle processing library of the SubX project,
written in Rust (edition 2024). It provides subtitle matching, format
conversion, audio synchronization via Voice Activity Detection (VAD),
encoding detection, and AI service integrations as a reusable library with no
binary and no terminal presentation code.

- **Repository:** <https://github.com/jim60105/subx-core>
- **License:** GPL-3.0-or-later
- **Crate name:** `subx-core`

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
- **`Cargo.toml` must never contain a `[workspace]` table.** This crate is a
  workspace *member* inside `subx-cli`; a member cannot also be a root, and a
  nested `[workspace]` is a hard Cargo error, not a warning.
- **No workspace inheritance of any kind.** No `version.workspace = true`,
  `authors.workspace = true`, `<dep>.workspace = true`, or
  `[lints] workspace = true`. All of those resolve inside `subx-cli`'s
  workspace but break a standalone `git clone` of this repository — which is
  exactly how crates.io consumers, `docs.rs`, and the Tauri GUI see this crate.
  Lint tables are therefore written out literally in `Cargo.toml` and kept in
  agreement with `subx-cli`'s by review.
- **No `[profile.*]` tables.** Cargo ignores profiles in a non-root workspace
  member and warns on every build when one is present. Release/dev profiles
  live in `subx-cli/Cargo.toml` only.
- **Every configuration file is this repository's own.** `.gitignore`,
  `.gitattributes`, `rustfmt.toml`, `.config/nextest.toml`, `.llvm-cov.toml`,
  `.codegraph/.gitignore`, `LICENSE`, and the committed `Cargo.lock` apply to
  a standalone clone; the parent repository's copies never reach inside a
  submodule.
- **Clone/update commands** (for contributors working through `subx-cli`):
  `git clone --recurse-submodules https://github.com/jim60105/subx-cli` for a
  fresh clone; `git submodule update --init --recursive` to repair an existing
  clone; `git config submodule.recurse true` to keep the submodule working
  tree following the pointer on `git pull`/`git checkout` (per-clone setting;
  does not apply to `git clone`).

## Module Guide

The library sources migrated from `subx-cli` at their identical relative
paths:

| Path | Owns |
|---|---|
| `src/config/` | The configuration system: `Config`, the `ConfigService` DI trait, `ProductionConfigService`, the `Test*` utilities, `field_validator`, `validator`, and the twelve `#[macro_export]` test macros (`test_macros.rs`, reachable at the crate root) |
| `src/core/` | The processing engines: `formats` (SRT/ASS/VTT/SUB + `encoding`), `matcher`, `sync`, `translation`, `parallel`, `archive`, `input`, `report` (the `Reporter` seam), `lock`, `file_manager`, `factory`, `language`, `uuidv7`, `fs_util` |
| `src/error.rs` | `SubXError`, its helper constructors and `From` conversions, and the machine-readable contract (`category`, `machine_code`, `hint`) |
| `src/services/` | External integrations: `ai` (providers, retry, security, error sanitizer), `audio`, `vad` |

Public module paths are frozen at their pre-split `subx_cli::` shapes (only
the crate name differs) — see the crate-level rustdoc in `src/lib.rs` before
reshaping anything.

## Dependency Direction — A One-Way Boundary

**No file under `src/` may name `subx_cli`, `crate::cli` or `crate::commands`
anywhere in a line — not in code, not in an intra-doc link, not in a
doctest.** The dependency between the two repositories points downward only
(`subx-cli` depends on `subx-core`); there is no dependency edge in the other
direction and there never will be, so an upward reference is *unfixable*
rather than merely stale — and under `broken_intra_doc_links = "deny"` an
upward doc link is a hard build failure with no repair short of deleting it.

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
| Doc build | `cargo doc --all-features --no-deps --document-private-items` |
| Doc tests | `cargo test --doc --all-features` |

### Important Notes

- **Use `cargo nextest run || true` for tests**, never `cargo test` (except
  for doc tests). The `|| true` prevents shell abort due to a known nextest
  issue in this project — **you must still inspect the output and treat any
  test failure as a real failure**.
- **Always run `cargo fmt` and `cargo clippy -- -D warnings`** and fix every
  warning before submitting code.
- Required tooling: Rust stable, `rustfmt`, `clippy`, `cargo-nextest`.

### CPU-Intensive Operations — Main Agent Only

**NEVER** run the following commands in sub-agents or in parallel:

- `cargo nextest run` without a `--filter-expr` (runs the full test suite)

These operations are CPU-intensive and will saturate all cores. Running them
in multiple sub-agents simultaneously will cause system overload, timeouts,
and unreliable results.

**Correct workflow:**

- Sub-agents writing tests should run only their own scoped tests using
  `cargo nextest run --filter-expr 'test(module_name)' || true`.
- The **main agent** runs the full test suite once after all sub-agents have
  finished and changes are consolidated.
- The main agent runs the checks **before every `git commit`** to ensure all
  changes pass together.

## Coding Conventions

### General Rules

- All code comments and rustdoc must be written in **English**.
- Do not introduce new `#[deprecated]` attributes. When removing
  functionality, delete the item and update all call sites.
- Unimplemented code must be marked with `// TODO`. Unless requirements
  explicitly permit phased implementation, all TODOs must be resolved
  before submitting.
- Never parse or hand-edit `Cargo.lock` — it is managed by Cargo.
- Formatting: `rustfmt.toml` sets edition 2024 with max width 100 columns.

### Error Handling

- Use `SubXError` variants from `src/error.rs` — never invent ad-hoc error
  types.
- The machine contracts `category()`, `machine_code()` and `hint()` stay on
  `SubXError` in `src/error.rs`. Process/terminal presentation (exit codes,
  user-facing prose) belongs to the consumer's CLI layer, not this crate —
  render messages through `Display` here and never call binary-half
  presentation helpers.

### Naming Conventions

- Modules: `snake_case`.
- Factory methods: `create_*` on `ComponentFactory`.

## Testing Conventions

### Critical Rules

- **Always use `TestConfigService`** for configuration in tests. Never use
  `ProductionConfigService`.
- **Never modify global state** — no `std::env::set_var`, no `static mut`,
  no `Lazy<Mutex<_>>`, no writes outside `TempDir`.
- **All tests must be parallel-safe** and deterministic.
- **Async tests** use `#[tokio::test]`.

### Test Infrastructure

| Helper | Location | Purpose |
|---|---|---|
| `TestConfigService` | `src/config/test_service.rs` | Isolated config without filesystem I/O |
| `TestConfigBuilder` | `src/config/builder.rs` | Fluent builder for test configs |
| `TestEnvironmentProvider` | `src/config/environment.rs` | In-memory env vars for isolated testing |

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
- **Integration tests:** `tests/*.rs`, one file per feature area. Import
  shared helpers via `mod common;` at the top.
- **Shared helpers:** `tests/common/` — mocks, generators, fixtures.

## Documentation Conventions

- Write rustdoc in **English** for all public APIs.
- Required sections for public functions: `# Arguments`, `# Returns`,
  `# Errors`, `# Examples`.
- Include `# Panics` and `# Safety` sections when applicable.
- All doc examples must compile — verified by `cargo test --doc --all-features`.
- Use intra-doc links: `` [`crate::module::Type`] ``. Broken links are
  denied (`broken_intra_doc_links = "deny"` in `Cargo.toml`).

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
the working directory before command dispatch.

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

## OpenSpec and Project Skills

Specification governance for this repository is currently owned by the
`subx-cli` repository's `openspec/` directory; `openspec init` for this
repository is a planned later step of the two-crate split. Author changes in
`subx-cli`'s OpenSpec until then.

<!-- CODEGRAPH_START -->
## CodeGraph

In repositories indexed by CodeGraph (a `.codegraph/` directory exists at the repo root), reach for it BEFORE grep/find or reading files when you need to understand or locate code:

- **MCP tool** (when available): `codegraph_explore` answers most code questions in one call — the relevant symbols' verbatim source plus the call paths between them, including dynamic-dispatch hops grep can't follow. Name a file or symbol in the query to read its current line-numbered source. If it's listed but deferred, load it by name via tool search.
- **Shell** (always works): `codegraph explore "<symbol names or question>"` prints the same output.

If there is no `.codegraph/` directory, skip CodeGraph entirely — indexing is the user's decision.
<!-- CODEGRAPH_END -->
