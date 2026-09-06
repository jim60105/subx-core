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

- [x] 3.1 `git remote add spec-history /tmp/spec-history && git fetch spec-history`; `git merge --allow-unrelated-histories --no-commit spec-history/HEAD`; the thirteen add/add collisions resolved to ours; post-merge tree verified byte-identical to the pre-merge import commit's tree (same tree hash)
- [x] 3.2 History-depth verification (amended at execution time): plain `git log -- <path>` shows 1 for every file because history simplification follows the ours-side parent of the add/add merge; `git log --full-history` and `git log --follow` prove the donor commits landed — vad-speech-detection 3 full-history / 2 follow commits, media-discovery 6 / 5, ai-provider-integration 5 follow. The provenance is readable; the check was re-expressed to match git's merge-simplification semantics.
- [x] 3.3 `openspec validate --specs --strict` still 13/13 after the merge (and `--all --strict` 13/13)

## 4. Documentation

- [x] 4.1 `CHANGELOG.md` `[Unreleased]` `### Added`: OpenSpec root + thirteen capabilities (96 requirements / 254 scenarios); `### Documentation`: provenance + handoff record for C2b (re-qualify `local-llm-provider`'s three references, `configuration-management`/`error-handling` split halves) and C3 (drift-check home, `subx-core/AGENTS.md` handoff block)
- [x] 4.2 Commit `feat(specs): import thirteen core capabilities from subx-cli`; merge commit `chore(specs): merge filtered spec history from subx-cli`; push `origin main` (landed: `1a49e4a` feat + `113bb03` history merge, pushed to `origin main` on 2026-09-06; the merge resolves the add/add on `openspec/specs/**` to ours — the archive-repaired tree — byte-identical to the archived specs)

## 5. Quality Gate

- [x] 5.1 Scoped evidence (full gate is the superproject's, per the paired-change rule): `git diff --stat` outside `openspec/` and `CHANGELOG.md` is empty between base and the import commit, and `cargo check` is green — the change is spec-only
