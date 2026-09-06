# Import Core Specs — Design

## Context

`subx-core` became a standalone crate (B1/B2) and a standalone OpenSpec root (this change's parent, `move-core-capabilities-to-subx-core` in `subx-cli`, C2a). The thirteen capabilities classified as pure-core there have always described code that lives at these relative paths in this repository; their specifications, however, were authored and archived in `subx-cli`. A standalone clone of this repository — how crates.io consumers, docs.rs and the Tauri GUI see the crate — could not read a single requirement its code satisfies. The ownership-classification argument is **not** re-argued here; it lives in `subx-cli`'s `openspec/changes/archive/<date>-move-core-capabilities-to-subx-core/design.md` Decision 1, and this file cites it rather than duplicating it.

## Goals / Non-Goals

- **Goal:** this repository's `openspec/specs/` holds the thirteen capabilities, 96 requirements, 254 scenarios, verbatim, with provenance readable from `openspec/changes/archive/`.
- **Goal:** the delta files are mechanically extracted from `subx-cli`'s main spec tree, so the only differences from the source are the enumerated editorial edits.
- **Non-goal:** no behaviour, no code, no test, no API, no dependency change.
- **Non-goal:** no re-titling, re-ordering, merging, or improvement of any requirement.

## Decisions

### Decision 1: the text is reproduced verbatim, and the diff against the source is expected to be empty apart from the enumerated edits

Each delta is produced mechanically: take `subx-cli/openspec/specs/<cap>/spec.md`, drop everything up to and including the `## Requirements` header, prepend `## ADDED Requirements`. Losslessness is verified by title-set equality per capability (96/254 totals) **before** any edit. The only post-extraction edits are the seventeen `**Migration**-scope` areas enumerated in the parent proposal — de-qualifications, corrected line ranges, cross-repository qualifications, framings. A diff of each delta against its extracted source must touch nothing else; this is verified task-by-task, not assumed.

### Decision 2: the thirteen Purposes are written by hand, twelve carried and one new

`openspec archive` writes a `TBD` placeholder Purpose, so each is authored after the archive. Twelve carry their pre-split text: six verbatim (`ai-provider-integration`, `file-organization`, `language-detection`, `local-llm-provider`, `media-discovery`, `vad-speech-detection`), four verbatim and path-less (`async-runtime-safety`, `file-operation-safety`, `input-size-guards`, `subtitle-parser-hardening`), two with dead path citations modernized to directory form (`archive-extraction`: `src/core/archive.rs` → `src/core/archive/`; `subtitle-styling`: `ass.rs`/`vtt.rs`/`sub.rs` → `src/core/formats/ass/`, `src/core/formats/vtt/`, `src/core/formats/sub/`) — a factual correction, not a rewrite. `core-reporting` is written new (its pre-split Purpose names `src/cli/reporter.rs`, a `subx-cli` path this root must not cite unqualified): *Provide a transport-agnostic reporting sink through which every module under `src/core/` and `src/services/` emits human-oriented status, diagnostic, warning, progress and AI-usage output, so that the library never writes to a terminal and never learns that a machine-readable output mode exists. The consumer supplies the sink; the default is silent. Implemented in `src/core/report/mod.rs` (`Reporter`, `NoopReporter`, `AiUsage`, `ProgressEvent`, `noop()`) and attached to `MatchEngine`, `TranslationEngine`, `SyncEngine`, `FileManager`, `WorkerPool` and `ComponentFactory` through `with_reporter`.*

### Decision 3: the post-archive H1 and Purpose repair exists because the tool's output cannot be trusted

`openspec archive` writes `# <capability> Specification` as the H1 and the `TBD` Purpose, and omits the blank lines after `## Purpose`/`## Requirements` that this project's main-spec shape (SDR §10) uses. Both pass `validate --strict` while being wrong, so the repair is scripted against exact expected strings, not eyeballed, and gated by a zero-`TBD` grep plus 13/13 `validate --specs --strict` before anything is committed.

### Decision 4: spec-file history is preserved by the unrelated-histories merge, executed after the import commit, ours-wins

`git filter-repo` over a scratch clone of `subx-cli` master, filtered to the thirteen `openspec/specs/<cap>` directories (17 commits), is merged into `main` with `--allow-unrelated-histories` **after** the import commit lands — verified probe: archiving an ADDED delta aborts atomically when requirement titles already exist, so the merge cannot precede the archive. The merge is a deliberate thirteen-file add/add collision resolved to ours (`--no-commit`, then `git checkout --ours -- <the thirteen paths>`, verify each first-parent diff is pure addition-of-history-only and the resulting tree is byte-identical to pre-merge). Fallback if `git filter-repo` is unavailable: plain copy plus an orphan `pre-split-spec-history` branch — not taken; `git filter-repo` 249bceb is installed.

## Risks / Trade-offs

- The merge makes `git log --follow` per spec file cross the repository boundary; accepted — provenance beats a clean per-file log.
- The duplication window (requirements present in both trees between the import commit here and the donor-side removal commit in `subx-cli`) is minutes long and deliberate; the reverse window (requirements nowhere) is unrecoverable and is what this ordering prevents.
- Line-range citations corrected today can drift again with future code edits; they were re-verified against this tree at import time, which is the most any text citation can promise.
