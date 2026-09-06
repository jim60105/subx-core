## Why

This change adds no behaviour. It exists because an archived change is OpenSpec's only mechanism for populating `openspec/specs/`, and 91 requirements that describe code this crate already owns — the library halves of twelve capabilities split by `split-mixed-capabilities-across-repos` in [`subx-cli`](https://github.com/jim60105/subx-cli) — must have their provenance readable from this repository, the way C2a's `import-core-specs` did for thirteen whole capabilities.

Each of the twelve capabilities this change adds is **one half of a capability of the same name in `subx-cli`**: this repository's file specifies the library obligations, that repository's file specifies the command-surface half, and `subx-cli`'s `openspec/split-capabilities.txt` is the enumerable record of which names are split. The other half of this change is therefore `split-mixed-capabilities-across-repos` in `subx-cli`: it authors the deletions from that tree and the `**Migration**` notes that enumerate every intentional difference between a donor requirement and its arriving copy. Neither change is independently reviewable; they are two halves of one division, and reviewing this one means reading its `**Migration**` notes against the sending change's deltas.

## What Changes

- Twelve capabilities are added to `openspec/specs/`, carrying **91 requirements** composed as follows (requirement count per capability; the arriving titles are generated from the sending change's 84 `## REMOVED Requirements` entries — 82 arriving under their own names, two arriving renamed — plus seven new core-half titles):
  - `cache-management` — 6 requirements, all verbatim.
  - `component-factory` — 6 requirements, all verbatim.
  - `configuration-management` — 14 requirements: 13 verbatim (with the `AI Environment Variable Overrides` line range re-verified and `Local Provider Validation Rules` superseding C2a's cross-repository edit per its `**Migration**` note) plus the composed *Tolerant Configuration Load Path*.
  - `encoding-detection` — 2 requirements: *Low-Confidence Fallback To Default Encoding* verbatim plus the composed *Detector Tolerates Empty and Binary Input*.
  - `error-handling` — 10 requirements: 7 verbatim plus three composed (*Display Is the Library's Error Rendering*, *Library Code Surfaces Recoverable Failures as Errors*, and *Library Error Surface Holds Only Machine Contracts*, the renamed core half of the removed *Library and Binary Error Surface Split*).
  - `format-conversion` — 5 requirements: 4 verbatim plus the composed *Target Format Conversion Semantics*.
  - `input-path-handling` — 13 requirements: 7 verbatim plus six composed because a CLI clause was lifted out (*Core-Owned Input Collection*, *Unified Path Merging*, *Extension Filtering*, *Recursive vs Flat Traversal*, *CollectedFiles Additional APIs*, and *No-Extract Collection Switch*, the renamed core half of the removed *No-Extract CLI Flag*).
  - `parallel-processing` — 8 requirements: 7 verbatim plus the composed *Task Scheduler Entry Point* (its two CLI obligations lifted into the `subx-cli` addition; *Parallel match over a directory* restated here without its reporting clause).
  - `secrets-protection` — 4 requirements: 3 verbatim plus the composed *Sensitive Value Masking Helper*.
  - `subtitle-matching` — 6 requirements: 2 verbatim plus four composed (*AI-Based File Pairing*, *Confidence Threshold Enforcement*, *File Relocation Modes*, *AI-Driven Language and Globally-Unique Target Naming*; *Optional Backup Before Move* arrives losing only its `--backup` flag-definition clause).
  - `subtitle-translation` — 7 requirements: 6 verbatim (five with the scenario re-phrasings their `**Migration**` notes prescribe) plus the composed *Translation Prompt Guidance Inputs*.
  - `timeline-sync` — 10 requirements: 7 verbatim plus three composed (*Sync Method Selection*, *Core-Owned Sync Pairing Resolution*, *Core-Owned Default Output Path Derivation*).
- Twelve `## Purpose` paragraphs are written by hand after archiving (the archive writes `TBD`), each naming only this repository's paths and stating in prose that a capability of the same name in `subx-cli` holds the command-surface half.
- The citation edits enumerated in the sending change's `**Migration**` notes are applied: seven CLI-bound test/documentation citations qualified `subx-cli:`, the `subx-core/`-prefixed self-qualifications de-qualified to root-relative per `spec-governance`.
- `local-llm-provider` is modified: three requirements restated with C2a's `in `subx-cli`` cross-repository qualifications replaced by same-repository references, because their targets — `configuration-management` and `error-handling` — land here by this change.

## Capabilities

### New Capabilities

- `cache-management` — the match cache's location, keying, invalidation, and reuse semantics inside `src/core/matcher/`.
- `component-factory` — config-driven construction of services and engines in `src/core/factory.rs`.
- `configuration-management` — the schema, service, validation, and environment-override machinery under `src/config/`.
- `encoding-detection` — the charset detector's semantics in `src/core/formats/encoding/`.
- `error-handling` — the `SubXError` machine contract in `src/error.rs`.
- `format-conversion` — subtitle parsing, writing, and conversion semantics in `src/core/formats/`.
- `input-path-handling` — input collection, traversal, and archive expansion in `src/core/input/`.
- `parallel-processing` — the task scheduler, worker pool, and UUIDv7 identity in `src/core/parallel/`.
- `secrets-protection` — the masking helper and redaction primitives under `src/config/` and `src/services/`.
- `subtitle-matching` — AI pairing, confidence, relocation, and naming semantics in `src/core/matcher/`.
- `subtitle-translation` — the translation engine's prompt, batching, and cue-mapping semantics in `src/core/translation/`.
- `timeline-sync` — the sync engine, pairing resolution, and VAD integration in `src/core/sync/` and `src/services/vad/`.

### Modified Capabilities

- `local-llm-provider` — three requirements re-qualified from `subx-cli` references to same-repository references (see *What Changes*).

## Impact

- **Code:** None. This is a specification-only change; the code the requirements describe is already in this crate.
- **Tests:** None.
- **APIs:** None.
- **Dependencies:** None.
- **Documentation:** `subx-core/CHANGELOG.md` gains an `[Unreleased]` entry recording the import and naming `split-mixed-capabilities-across-repos` in `subx-cli` as the other half.
