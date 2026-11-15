# Tasks

## 1. Reporting payload

- [ ] 1.1 In `src/core/report/mod.rs`, add `#[non_exhaustive] pub enum TranslationPass { Initial, FillIn }` and `#[non_exhaustive] pub enum TranslationProgress` with `TerminologyStarted`, `TerminologyFinished { terms: u64 }`, `BatchStarted { pass, batch, batches, cues, attempt }` and `BatchFinished { pass, batch, batches, processed_cues, total_cues }`, both deriving `Debug, Clone, Copy, PartialEq, Eq`; verify `cargo check --all-features` passes
- [ ] 1.2 Add `ProgressEvent::Translation(TranslationProgress)` and verify the existing `ProgressEvent` derives still compile and the in-crate `match` sites over `ProgressEvent` (`src/core/report/mod.rs`, both already wildcarded) still build. The matcher and worker modules only construct events; the cross-crate matches in `tests/match_progress_tests.rs` and `subx-cli/src/cli/reporter.rs` are already wildcarded and need no edit, but their compilation is confirmed in the submodule bump step
- [ ] 1.3 Write rustdoc for both enums, every variant and every field (meaning of `batch`, `batches`, `cues`, `attempt`, `processed_cues` including the fill-in fallback note), with a runnable example of a `Reporter` that handles `ProgressEvent::Translation` plus a `_` arm; verify `cargo test --doc --all-features` passes
- [ ] 1.4 Add a `compile_fail` doctest showing that a `match` over `TranslationProgress` without a `_` arm does not compile from outside the crate, and unit tests in the report module for: `Translation` values comparing by value, the payload being `Copy + Send` with no lifetime (static assertion), and a four-variant-plus-wildcard implementor receiving a `Translation` event in its `_` arm; verify with `cargo nextest run --filter-expr 'test(report)' || true` and inspect the output

## 2. Engine emission

- [ ] 2.1 In `src/core/translation/engine.rs`, emit `TerminologyStarted` before and `TerminologyFinished { terms }` after the `extract_terminology` call inside `translate_subtitle` (not inside `extract_terminology` itself), with `terms` taken before `merge_terminology`; verify with a test using a recording reporter and a scripted provider that asserts the order and the `terms` value with glossary entries present
- [ ] 2.2 Pass a private batch descriptor (`pass`, `batch`, `batches`) into `translate_batch_with_unknown_retry` and emit `BatchStarted { attempt: 1 }` before the first request and `BatchStarted { attempt: 2 }` after the existing retry notice; verify with a test scripting one unknown-cue-ID response followed by a valid one that asserts the sequence `BatchStarted(1)`, retry `Message`, `BatchStarted(2)`, one `BatchFinished`
- [ ] 2.3 Emit `BatchFinished` for each initial batch after its map is inserted and immediately before the existing `Processed cues` `Message`; verify with a 250-cue, batch-size-100 test asserting indices 1..=3, `batches: 3`, `cues` of 100/100/50 and `processed_cues` of 100/200/250, plus a 100-cue batch whose response omits 4 cues asserting `BatchFinished { processed_cues: 96, total_cues: 250 }` and the following `Message` reading `Processed cues: 96/250`
- [ ] 2.4 Emit the fill-in pass as `BatchStarted`/`BatchFinished` with `pass: FillIn`, `batch: 1`, `batches: 1`, the finish placed after the empty-fallback insertion and before the existing `Message`; verify with tests for a file that leaves cues missing (fill-in events present, `processed_cues == total_cues`) and a file that does not (no `FillIn` event)
- [ ] 2.5 Add tests for the no-finish cases: a failing terminology request, a failing batch request, and two consecutive unknown-cue-ID responses each report the started event, no finished event, and return the same error as before
- [ ] 2.6 Add tests that the text channel is unchanged: every `BatchFinished` is immediately followed by a `Message` containing `Processed cues: <processed>/<total>` with the same numbers, a reporter recording only `Message` sees the same lines as on `master` for a fixed script, no `Started`/`Advanced`/`Finished` is emitted, and an empty subtitle or empty target language emits no `Translation` event
- [ ] 2.7 Update the rustdoc of `TranslationEngine::with_reporter` and `translate_subtitle` to list the structured events; verify `cargo doc --all-features --no-deps --document-private-items` builds without warnings

## 3. Release preparation

- [ ] 3.1 Add an `### Added` entry under `## [Unreleased]` in `CHANGELOG.md` describing `ProgressEvent::Translation`, `TranslationProgress` and `TranslationPass`, and stating that the text progress lines are unchanged; verify the entry follows the Keep a Changelog sections used in the file
- [ ] 3.2 Bump `version` in `Cargo.toml` to `1.1.0` and refresh `Cargo.lock`; verify `cargo check` leaves the tree with only those two files changed by the bump
- [ ] 3.3 Run `cargo fmt` and `cargo clippy -- -D warnings`; verify both pass. Hand the full `scripts/quality_check.sh ci` gate to the main agent, which is the only session allowed to run this CPU-intensive suite, and record its result
- [ ] 3.4 Promote the changelog entry: rename `## [Unreleased]` to `## [1.1.0] - <release date>`, add its link reference, and leave a fresh empty `## [Unreleased]` above it, as the changelog convention requires released entries under their own version header; verify `release.yml`'s extraction of `## [1.1.0]` yields the new notes
- [ ] 3.5 Run `openspec validate add-translation-progress-events --strict`; verify it passes without warnings

## Workflow follow-up

- Bump the `subx-cli` submodule pointer to the merged commit and run `subx-cli/scripts/quality_check.sh`, the authoritative gate; the CLI's characterisation tests must pass without edits.
- Tag `v1.1.0` in this repository so `release.yml` publishes the crate; the SubX GUI change `redesign-translate-ledger-and-progress` depends on that release.
- Archive this change after the release.
