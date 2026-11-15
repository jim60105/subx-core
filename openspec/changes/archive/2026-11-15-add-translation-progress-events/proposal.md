# Proposal

## Why

`TranslationEngine` reports its per-file progress only as a formatted text line (`Processed cues: <processed>/<total>`) and a free-form retry notice. A consumer that wants to draw real progress — the SubX desktop GUI is about to — must parse that text and wrap the AI provider to guess which request is in flight. Both are fragile: the text is presentation, and the request order is an implementation detail.

## What Changes

- Add a core-owned payload type `TranslationProgress` and a `ProgressEvent::Translation(TranslationProgress)` variant, so a `Reporter` receives typed events for the four things a translation does per file:
  - the terminology request starting and finishing;
  - each cue batch starting (with its 1-based index, the batch count, its cue count, and whether this is the resend after an unknown-cue-ID response);
  - each cue batch finishing (with processed and total cue counts);
  - the fill-in pass for cues that came back missing, as a batch of its own pass kind.
- `TranslationEngine::translate_subtitle` emits these events through its attached `Reporter`, next to the text it emits today.
- The existing `ProgressEvent::Message` lines stay byte-for-byte as they are. The CLI renders them and its characterisation tests lock them; nothing is converted or removed.
- Release as `1.1.0`: new public items only, and `ProgressEvent` is already `#[non_exhaustive]`, so no implementor breaks.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `core-reporting`: `ProgressEvent` gains a fifth variant carrying a core-owned translation progress payload.
- `subtitle-translation`: the engine reports terminology, batch, resend and fill-in progress as structured events in addition to the existing text.

## Impact

- **Code**: `src/core/report/mod.rs` (new types, new variant, rustdoc), `src/core/translation/engine.rs` (emission points), tests in both modules and in `tests/translation_engine_test.rs`.
- **Public API**: additive. `subx_core::core::report::{TranslationProgress, TranslationPass}` and `ProgressEvent::Translation`. Every existing `match` over `ProgressEvent` already needs a wildcard arm and keeps compiling.
- **Consumers**: `subx-cli` ignores the new variant through its wildcard arm and prints the same bytes. The SubX GUI (`jim60105/subx`) will consume the events in its own change `redesign-translate-ledger-and-progress`, which requires `subx-core` `1.1.0` to be published first.
- **Dependencies**: none added.
- **Docs**: `CHANGELOG.md` entry under `## [Unreleased]`, version bump to `1.1.0` in `Cargo.toml`.
