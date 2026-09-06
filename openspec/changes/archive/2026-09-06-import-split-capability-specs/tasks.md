## 1. Author the Receiving Artifacts

- [x] 1.1 Create `.openspec.yaml` containing exactly two lines: `schema: spec-driven` and `created: 2026-09-06`
- [x] 1.2 Write `proposal.md` with the four H2s: `## Why` states the change adds no behaviour, names `split-mixed-capabilities-across-repos` in `subx-cli` as its other half, states that each of the twelve capabilities is one half of a same-named capability there and that this change is not independently reviewable; `## Capabilities` lists the twelve under `### New Capabilities` and `local-llm-provider` under `### Modified Capabilities`; `## Impact` sets Code/Tests/APIs/Dependencies to `None.` and names `subx-core/CHANGELOG.md` under Documentation
- [x] 1.3 Write `design.md` with exactly the four decisions of the sending change's Decision 8 (verbatim-versus-composed reproduction; the twelve Purposes; the post-archive H1/Purpose repair; the `local-llm-provider` re-qualification), citing `subx-cli`'s archived `split-mixed-capabilities-across-repos` design for the classification without re-arguing it
- [x] 1.4 Write this `tasks.md` with numbered H2 phases, ending with the documentation phase and the quality-gate phase run against this repository's own quality script

## 2. The Delta Specs

- [x] 2.1 Create the twelve `specs/<capability>/spec.md` files starting with `## ADDED Requirements`, no H1 and no `## Purpose`, carrying the core halves in donor source order
- [x] 2.2 Verify the arriving title set equals the sending change's 84 `## REMOVED Requirements` titles with the two renames applied (*Library and Binary Error Surface Split* → *Library Error Surface Holds Only Machine Contracts*, *No-Extract CLI Flag* → *No-Extract Collection Switch*) plus the seven new core-half titles, per capability: `cache-management` 6, `component-factory` 6, `configuration-management` 14, `encoding-detection` 2, `error-handling` 10, `format-conversion` 5, `input-path-handling` 13, `parallel-processing` 8, `secrets-protection` 4, `subtitle-matching` 6, `subtitle-translation` 7, `timeline-sync` 10 — **91** total, with zero duplicate titles in any file
- [x] 2.3 Write `specs/local-llm-provider/spec.md` as `## MODIFIED Requirements` restating *Local LLM Provider Identifier*, *Local Provider Environment Variable Overrides*, and *Actionable Local-Endpoint Error Mapping* in full with the `in `subx-cli`` cross-repository qualifications replaced by same-repository references
- [x] 2.4 Confirm the only differences from the donor texts are the enumerated edits: seven `subx-cli:`-qualified citations, the de-qualified `subx-core/` self-qualifications, the fourteen lifted-clause deletions, the nine composed halves, and the two deliberate duplications (the English-language sentence in *Display Is the Library's Error Rendering*; *Parallel match over a directory* without its reporting clause)
- [x] 2.5 Sweep the twelve ADDED files: `src/cli/`, `src/commands/`, `src/main.rs` appear only behind a `subx-cli:` or prose `subx-cli` qualification; no path is prefixed `subx-core/`

## 3. Validate and Archive

- [ ] 3.1 Run `openspec validate import-split-capability-specs --strict` from this repository's root and require green before archiving
- [ ] 3.2 Run `openspec archive import-split-capability-specs -y` — **without** `--skip-specs` — and confirm `"specsUpdated": true`
- [ ] 3.3 Repair the twelve H1s to the Title Case capability names (including `# Subtitle Translation`, not the ancestor's `# Subtitle Translation Specification`) and the twelve Purposes per design Decision 2; restore the blank lines after `## Purpose` and `## Requirements`; grep-zero `TBD` in `openspec/specs/`
- [ ] 3.4 Confirm the three `local-llm-provider` requirements in `openspec/specs/local-llm-provider/spec.md` now reference this repository's `configuration-management` and `error-handling` and cite nothing `subx-cli`-bound unqualified
- [ ] 3.5 Run `openspec validate --specs --strict` (expect twenty-five capabilities green) and `openspec validate --all --strict` from this repository's root
- [ ] 3.6 Dedup guard: enumerate `### Requirement:` titles per main-spec file and confirm zero duplicates across all twenty-five files
- [ ] 3.7 Confirm the per-capability main-spec counts: cache-management 6, component-factory 6, configuration-management 14, encoding-detection 2, error-handling 10, format-conversion 5, input-path-handling 13, parallel-processing 8, secrets-protection 4, subtitle-matching 6, subtitle-translation 7, timeline-sync 10, and that no pre-existing capability lost a requirement

## 4. Documentation

- [ ] 4.1 Add a `[Unreleased]` entry to `CHANGELOG.md` recording that the library halves of the twelve split capabilities are now specified in this repository by `import-split-capability-specs`, and that `split-mixed-capabilities-across-repos` in `subx-cli` is their other half

## 5. Quality Gate

- [ ] 5.1 Run this repository's own quality script (`scripts/quality_check.sh`, per C1 Decision 4) and confirm green — spec-only change, so `cargo check --workspace` output must be byte-identical to the pre-change run
