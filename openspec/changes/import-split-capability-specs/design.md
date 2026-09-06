## Context

This change moves nothing. It records, in this repository, the library halves of twelve capabilities that `split-mixed-capabilities-across-repos` divided in `subx-cli` — 91 requirements that describe code this crate already owns after B2, specified until now only in the parent repository. An archived change is OpenSpec's only mechanism for populating `openspec/specs/`, so the text must be carried through a change of its own, authored here, validated here, and archived here.

The classification that decided which requirements are library halves is not re-argued in this file. It is `subx-cli`'s `openspec/changes/archive/<date>-split-mixed-capabilities-across-repos/design.md`, Decisions 1–4 (the ownership test, the per-capability table, the clause-by-clause allocation for the twenty-three divided requirements, and the cross-repository citation rules). This design covers only how the arriving text is reproduced, framed, and repaired.

## Goals / Non-Goals

- **Goal:** `subx-core/openspec/specs/` ends with twenty-five capabilities — the thirteen C2a imported — and twelve new ones whose titles match this change's deltas title-for-title, validating green under `--strict`.
- **Goal:** every arriving requirement is byte-identical to its `subx-cli` source except the enumerated citation and composition edits, so the sending change's `**Migration**` notes are the complete diff record.
- **Non-goal:** the ownership classification, the donor-side deletions, and the `subx-cli` retained halves. Those belong to `split-mixed-capabilities-across-repos` and are applied there while that change is live.
- **Non-goal:** any `.rs`, manifest, or `tests/` change. This crate's code already satisfies the arriving text; B2 and B3 did that.

## Decisions

### Decision 1: the arriving text is the sending change's deltas' text, and the expected diff is empty apart from the enumerated edits

The twelve `## ADDED Requirements` delta files were assembled from `split-mixed-capabilities-across-repos`'s own deltas, not re-transcribed: for a requirement that migrates whole, the block is `subx-cli`'s main-spec text unchanged; for a divided requirement, it is the core half that change's `## REMOVED Requirements` `**Migration**` notes and Decision 3 prescribe. The complete list of intentional differences from the donor text is: the seven CLI-bound test/documentation citations qualified with `subx-cli:`; the `subx-core/`-prefixed self-qualifications de-qualified to root-relative paths; the fourteen lifted-clause compositions (six `input-path-handling`, three `timeline-sync`, five `subtitle-matching`) each losing exactly the clause its note names; the nine new or renamed core halves composed per Decision 3 (cases 1–9); and the two deliberate prose duplications — the English-language sentence restated in *Display Is the Library's Error Rendering*, and *Parallel match over a directory* restated core-side without its reporting clause. Anything else that differs is a defect to fix before archiving, not a variant to reconcile after. No filtered git history is attempted — `filter-repo` cannot express a subset of a file's requirements, and `subx-cli/openspec/changes/archive/**` is the series' established authoring record.

### Decision 2: the twelve `## Purpose` paragraphs are written by hand, after archive, and each names its counterpart

`openspec archive` writes `TBD - created by archiving change import-split-capability-specs. Update Purpose after archive.`, which passes `validate --strict` and must not be shipped. Each replacement paragraph is two to four sentences: what the capability governs inside this crate, which of `src/core/`, `src/services/`, `src/config/`, or `src/error.rs` implement it, ending with "Implemented in `…`" naming only this repository's paths, and one closing prose sentence stating that a capability of the same name in `subx-cli` specifies the command-surface half. The counterpart is named in prose without citing any `subx-cli` file path — a Purpose is a pointer, and a path there would rot in two repositories at once. No Purpose cites `src/cli/`, `src/commands/`, `src/main.rs`, `docs/`, or a `tests/` file that stays in `subx-cli`, except behind a `subx-cli:` qualification.

### Decision 3: the post-archive H1 and Purpose repair, and the blank-line restore

`openspec archive` writes each new main spec as `# <capability> Specification`. All twelve H1s are replaced with the Title Case capability name — `# Cache Management`, `# Component Factory`, `# Configuration Management`, `# Encoding Detection`, `# Error Handling`, `# Format Conversion`, `# Input Path Handling`, `# Parallel Processing`, `# Secrets Protection`, `# Subtitle Matching`, `# Subtitle Translation`, `# Timeline Sync` — including `subtitle-translation`, whose `subx-cli` ancestor carries the defective `# Subtitle Translation Specification`; the copy must not inherit it. The blank lines `openspec archive` omits after `## Purpose` and `## Requirements` are restored so all twelve match the shape of the C2a-imported thirteen. The repair is a post-archive commit in this repository; `--skip-specs` is not used here because, unlike the `subx-cli` side, no capability this change touches is emptied.

### Decision 4: `local-llm-provider` is re-qualified, not rewritten

C2a's `import-core-specs` carried three `local-llm-provider` requirements whose cross-repository references named `configuration-management` and `error-handling` "in `subx-cli`" because those capabilities lived there. This change lands both in this repository, so the three references become same-repository references: the delta restates *Local LLM Provider Identifier*, *Local Provider Environment Variable Overrides*, and *Actionable Local-Endpoint Error Mapping* in full with `in `subx-cli`` replaced by "this repository's", changing nothing else — per `spec-governance`, a MODIFIED delta restates the whole requirement.

## Risks / Trade-offs

- **Trade-off: twelve capability names now exist in both trees, and a reader of this repository sees an incomplete capability.** → Accepted; it is `spec-governance`'s chosen design. The mitigations are the shared name, the twelve Purpose paragraphs that each name their counterpart, and `subx-cli`'s `openspec/split-capabilities.txt` as the enumerable record.
- **Trade-off: the requirement count of this repository rises by 91 with zero behaviour change.** → Accepted. The count is in the proposal so it is not later read as scope creep.
- **Risk: a composed core half loses or duplicates a clause of its donor requirement.** → Mitigated before archiving: the title-set equality check against the sending change's 84 REMOVED titles plus the two renames plus the seven new titles, and the per-capability counts, are run against the delta files, and the `**Migration**` notes enumerate every intentional deletion.
