# Import Core Specs — Tasks

## 1. Author and Validate the Deltas

- [x] 1.1 Mechanically extract all thirteen deltas from `subx-cli/openspec/specs/<cap>/spec.md` (drop through `## Requirements`, prepend `## ADDED Requirements`); verify losslessness: per-capability title-set equality, totals 96/254
- [x] 1.2 Apply the seventeen migration-scope edits (four line-range corrections, one `subx-cli:` path, three capability qualifications, one `--recursive` framing, two `archive-extraction` framings, three `subtitle-parser-hardening` framings, two `crate::` readings + crate-topology pointer, fifteen `subx-core/` prefix strips); diff each delta against its extraction — every changed line belongs to the manifest
- [x] 1.3 `openspec validate import-core-specs --strict` green from inside `subx-core/`

## 2. Archive, Repair, Verify

- [x] 2.1 `openspec archive import-core-specs -y` (no `--skip-specs`); expect `"added": 96`
- [x] 2.2 Repair the thirteen H1s to Title Case capability names and the thirteen Purposes per design Decision 2; restore blank lines after `## Purpose`/`## Requirements`
- [x] 2.3 `grep -rn 'TBD - created by archiving' openspec/` returns zero; `openspec validate --specs --strict` → 13 passed
- [x] 2.4 Purpose path existence check: every `Implemented in` path resolves inside this repository; no unqualified `subx-cli` path in any Purpose

## 3. History Merge

- [ ] 3.1 `git remote add spec-history /tmp/spec-history && git fetch spec-history`; `git merge --allow-unrelated-histories --no-commit spec-history/master`; resolve the thirteen add/add collisions to ours; verify post-merge tree byte-identical to pre-merge HEAD tree
- [ ] 3.2 `git log --oneline -- openspec/specs/<capability>/spec.md | wc -l` > 1 for every capability with multi-commit donor history (ai-provider-integration 4, media-discovery 4, file-organization 3, subtitle-parser-hardening 3, archive-extraction 2, async-runtime-safety 2, core-reporting 2; single-commit capabilities may show 1 filtered commit + merge + import — record the per-capability table)
- [ ] 3.3 `openspec validate --specs --strict` still 13/13 after the merge

## 4. Documentation

- [x] 4.1 `CHANGELOG.md` `[Unreleased]` `### Added`: OpenSpec root + thirteen capabilities (96 requirements / 254 scenarios); `### Documentation`: provenance + handoff record for C2b (re-qualify `local-llm-provider`'s three references, `configuration-management`/`error-handling` split halves) and C3 (drift-check home, `subx-core/AGENTS.md` handoff block)
- [ ] 4.2 Commit `feat(specs): import thirteen core capabilities from subx-cli`; merge commit `chore(specs): merge filtered spec history from subx-cli`; push `origin main`

## 5. Quality Gate

- [ ] 5.1 Run `subx-core/scripts/quality_check.sh` (this repository's own gate, C1 Decision 4) or the scoped equivalent; for a spec-only change the evidence is an unchanged build/test surface — no `.rs`, manifest, lockfile or workflow touched: `git diff --stat <base>..HEAD -- . ':!openspec' ':!CHANGELOG.md'` empty
