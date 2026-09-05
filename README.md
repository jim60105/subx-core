# subx-core

Core subtitle processing library for [SubX](https://github.com/jim60105/subx-cli).

`subx-core` is the reusable library half of the SubX project: subtitle
matching, format conversion (SRT, ASS, VTT, SUB), audio-driven timing
synchronization via Voice Activity Detection (VAD), encoding detection, and
the AI service integrations behind them. It is a pure library — no binary, no
clap surface, no terminal presentation code.

- **Repository:** <https://github.com/jim60105/subx-core>
- **License:** GPL-3.0-or-later (see [LICENSE](LICENSE))
- **Crate name:** `subx-core`

## Installation

Add the crate from this repository with:

```toml
[dependencies]
subx-core = { git = "https://github.com/jim60105/subx-core", version = "1.0" }
```

Once the crate's first release is on crates.io, `cargo add subx-core` works as
well:

```sh
cargo add subx-core
```

## Relationship to `subx-cli` and the SubX GUI

This repository is normally consumed in one of two ways:

1. **As a git submodule of [`subx-cli`](https://github.com/jim60105/subx-cli)**,
   mounted at `subx-cli/subx-core/`. `subx-cli` is the Cargo workspace root,
   and `subx-core` is one of its two workspace members, so a
   `git clone --recurse-submodules` of `subx-cli` builds both crates with a
   single `cargo build`.
2. **As a standalone crate** — from crates.io, from this repository's git URL,
   or as a plain `git clone` followed by `cargo build`. The crate is fully
   self-contained: it never uses Cargo workspace inheritance, so a standalone
   clone of this repository must always build, format, lint, and test on its
   own. This is the contract that the Tauri GUI at
   [`jim60105/subx`](https://github.com/jim60105/subx) and every crates.io
   consumer depend on.

## Module Map

The library sources migrated from `subx-cli` at their identical relative paths:

| Module | Contents |
|---|---|
| `src/config/` | Configuration system: `Config`, the `ConfigService` DI trait, production/test services, validation |
| `src/core/` | Processing engines: `formats` (SRT/ASS/VTT/SUB + encoding), `matcher`, `sync`, `translation`, `parallel`, `archive`, `input`, `report`, `lock`, `file_manager`, `factory` |
| `src/error.rs` | `SubXError` and the machine-readable error contract (`category`, `machine_code`, `hint`) |
| `src/services/` | External integrations: AI providers (`services::ai`), audio processing, VAD |

Public paths are deliberately identical to the pre-split `subx_cli::` paths
with only the crate name swapped — including the redundant `core::` segment
(`subx_core::core::matcher::MatchEngine`). The rationale is recorded in the
crate-level rustdoc in [`src/lib.rs`](src/lib.rs).

### Feature flags

- `archive-rar` — enables RAR extraction via the optional `unrar` dependency
- `slow-tests` — compiles the long-running format round-trip tests

### Git history note

This branch carries **two roots**. The B1 skeleton commits are one; the other
is the history of these sources inside `subx-cli`, extracted with
`git filter-repo` restricted to the four migrated paths and merged with
`--allow-unrelated-histories` so `git log`/`git blame` keep working across the
repository boundary. It reads oddly in `git log --graph`; it is intentional.

## Building

```sh
cargo build
cargo nextest run
```

See [AGENTS.md](AGENTS.md) for the full development conventions, including
the submodule contract that governs how changes to this repository reach
`subx-cli`.

## License

GPL-3.0-or-later. See [LICENSE](LICENSE).
