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

## Current Status

At this commit the crate is a **placeholder skeleton**: its entire public
surface is [`VERSION`](src/lib.rs). The processing engines that will populate
it currently live in `subx-cli/src/core/`, `subx-cli/src/services/`,
`subx-cli/src/config/`, and `subx-cli/src/error.rs`, and arrive here with the
source-migration changes of the two-crate split. Until then, do not add
application logic here ahead of that migration.

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
