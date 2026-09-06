# subx-core

[![Build, Test, Audit & Coverage](https://github.com/jim60105/subx-core/actions/workflows/build-test-audit-coverage.yml/badge.svg)](https://github.com/jim60105/subx-core/actions/workflows/build-test-audit-coverage.yml)

Core subtitle processing library for [SubX](https://github.com/jim60105/subx-cli).

`subx-core` is the reusable library half of the SubX project: subtitle
parsing and conversion (SRT, ASS, VTT, SUB), AI-powered file matching,
audio-driven timing synchronization via Voice Activity Detection (VAD),
AI-assisted translation, encoding detection, archive extraction, and the
dependency-injected configuration system behind them. It is a pure library —
no binary target, no clap surface, no terminal presentation code.

- **Repository:** <https://github.com/jim60105/subx-core>
- **License:** GPL-3.0-or-later (see [LICENSE](LICENSE))
- **Crate name:** `subx-core`
- **API reference:** <https://docs.rs/subx-core> (canonical; live once the
  crate's first release is published)

## Who consumes this crate

1. **[`subx-cli`](https://github.com/jim60105/subx-cli)** — the command-line
   front-end, which mounts this repository as a git submodule at
   `subx-cli/subx-core/` (a Cargo workspace member) and depends on the crate
   with a caret requirement (`subx-core = { version = "1.0", path =
   "subx-core" }`), so a compatible `subx-cli` release always resolves a
   `subx-core` version it was built against.
2. **The Tauri GUI at [`jim60105/subx`](https://github.com/jim60105/subx)** —
   which consumes this crate directly (through its repository until the
   first crates.io release lands, through the registry after).

A library consumer SHOULD depend on `subx-core` directly and never on
`subx-cli`, whose library surface exists only as compatibility re-exports of
this crate. Want the end-user tool instead? Install `subx-cli` — see
[its README](https://github.com/jim60105/subx-cli/blob/master/README.md);
this repository ships no installer and documents no commands.

## Installation

The crate is published together with `subx-cli` releases; until its first
release lands on crates.io, depend on the git URL (the published version line
below is what the caret above expects):

```toml
[dependencies]
subx-core = { git = "https://github.com/jim60105/subx-core", version = "1.0" }
```

Once published:

```sh
cargo add subx-core
```

A **standalone clone is supported** and is exactly how crates.io, docs.rs,
and the GUI see the crate — the repository never uses Cargo workspace
inheritance or a `[workspace]` table, so a plain `git clone` followed by
`cargo build` must always work:

```sh
git clone https://github.com/jim60105/subx-core
cd subx-core
cargo build
cargo nextest run
```

## Module Map

The library sources migrated from `subx-cli` at their identical relative paths:

| Module | Contents |
|---|---|
| `src/config/` | Configuration system: `Config`, the `ConfigService` DI trait, production/test services, validation |
| `src/core/` | Processing engines: `formats` (SRT/ASS/VTT/SUB + encoding), `matcher`, `sync`, `translation`, `parallel`, `archive`, `input`, `report`, `lock`, `file_manager`, `factory` |
| `src/error.rs` | `SubXError` and the machine-readable error contract (`category`, `machine_code`, `hint`) |
| `src/services/` | External integrations: AI providers (`services::ai`), audio processing, VAD |
| `src/test_support/` | Shared test fixtures (behind the `test-support` feature; see below) |

Public paths are deliberately identical to the pre-split `subx_cli::` paths
with only the crate name swapped — including the redundant `core::` segment
(`subx_core::core::matcher::MatchEngine`). The rationale is recorded in the
crate-level rustdoc in [`src/lib.rs`](src/lib.rs).

### Feature flags

- `archive-rar` — enables RAR extraction via the optional `unrar` dependency
  (this manifest owns the real gate; `subx-cli`'s is a pass-through)
- `slow-tests` — compiles the long-running format round-trip tests
- `test-support` — exposes `src/test_support/` (config builder, file managers,
  mock OpenAI/Azure helpers used by `subx-cli`'s test suite) and pulls in its
  optional `wiremock`/`hound` dependencies. It is never a shipping default;
  `subx-cli` enables it through `[dev-dependencies]` only, and that is the
  recommended placement for any consumer — it is test scaffolding, not a
  feature to activate in a shipping dependency.

### Git history note

This branch carries **two roots**. The B1 skeleton commits are one; the other
is the history of these sources inside `subx-cli`, extracted with
`git filter-repo` restricted to the four migrated paths and merged with
`--allow-unrelated-histories` so `git log`/`git blame` keep working across the
repository boundary. It reads oddly in `git log --graph`; it is intentional.

## Documentation

- [AGENTS.md](AGENTS.md) — development conventions, including the submodule
  contract that governs how changes here reach `subx-cli`
- [Configuration guide](https://github.com/jim60105/subx-cli/blob/master/docs/configuration-guide.md)
  (lives in `subx-cli`) — behaviour of every setting this crate implements
- [Machine-readable output contract](https://github.com/jim60105/subx-cli/blob/master/docs/machine-readable-output.md)
  (lives in `subx-cli`) — the `category`/`machine_code`/`hint` mapping
  `SubXError` carries for the CLI's JSON mode
- [Technical architecture](https://github.com/jim60105/subx-cli/blob/master/docs/tech-architecture.md)
  (lives in `subx-cli`) — the narrative overview across both crates

## License

GPL-3.0-or-later. See [LICENSE](LICENSE).
