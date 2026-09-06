# Import Core Specs

## Why

This change adds no behaviour and satisfies no new requirement. It exists because `subx-core` is now a separately clonable crate whose specification otherwise lives in another repository, because OpenSpec's only mechanism for populating `openspec/specs/` is an archived change, and because the record of where these requirements came from has to be readable from this repository. Every requirement here was written in `subx-cli` and is reproduced verbatim; the only editorial changes are the thirteen `## Purpose` paragraphs and the seventeen citation scopes enumerated in the sending change's `**Migration**` notes. Its other half is `move-core-capabilities-to-subx-core` in `subx-cli`, which removes these same thirteen capabilities from that repository's spec tree and records the ownership argument; this change is not independently reviewable apart from it.

## What Changes

- **New:** thirteen core capabilities enter `subx-core/openspec/specs/` with 96 requirements and 254 scenarios, reproduced verbatim from `subx-cli`:

| Capability | Requirements | Scenarios |
|---|---|---|
| `ai-provider-integration` | 12 | 40 |
| `archive-extraction` | 10 | 36 |
| `async-runtime-safety` | 4 | 11 |
| `core-reporting` | 7 | 29 |
| `file-operation-safety` | 4 | 10 |
| `file-organization` | 7 | 13 |
| `input-size-guards` | 3 | 7 |
| `language-detection` | 5 | 14 |
| `local-llm-provider` | 7 | 20 |
| `media-discovery` | 8 | 15 |
| `subtitle-parser-hardening` | 8 | 26 |
| `subtitle-styling` | 11 | 16 |
| `vad-speech-detection` | 10 | 17 |
| **Total** | **96** | **254** |

- **Written by hand:** the thirteen `## Purpose` paragraphs. Twelve carry their pre-split text (six verbatim with core-path `Implemented in` closings, four verbatim and path-less, two — `archive-extraction`, `subtitle-styling` — with dead file citations modernized to the directory modules that exist here); `core-reporting`'s is written new because its old Purpose names a `subx-cli` path.
- **Carried citation edits (seventeen scopes, from the sending change's `**Migration**` notes):** four corrected line ranges in `language-detection`; one `subx-cli:` test-path qualification in `file-organization`; `slow-tests`/`Cargo.lock`/`fuzz/`/quality-script framings in `subtitle-parser-hardening`; three cross-capability qualifications to `subx-cli` in `local-llm-provider`; the `--recursive` qualification in `media-discovery`; the `-i`/`archive-rar` gate framings in `archive-extraction`; the `crate::core::report` → `subx_core::core::report` readings plus the crate-topology pointer in `core-reporting`; and fifteen self-qualification `subx-core/` prefix strips across the D1/D2-added requirements in `async-runtime-safety` and `core-reporting`.

## Capabilities

### New Capabilities

- `ai-provider-integration`
- `archive-extraction`
- `async-runtime-safety`
- `core-reporting`
- `file-operation-safety`
- `file-organization`
- `input-size-guards`
- `language-detection`
- `local-llm-provider`
- `media-discovery`
- `subtitle-parser-hardening`
- `subtitle-styling`
- `vad-speech-detection`

### Modified Capabilities

_None._

## Impact

- **Code:** None.
- **Tests:** None.
- **APIs:** None.
- **Dependencies:** None.
- **Documentation:** `subx-core/CHANGELOG.md` records the import and the handoff notes for C2b/C3.
