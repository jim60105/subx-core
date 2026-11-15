# Design

## Context

See `proposal.md` for motivation. The constraints that shape the approach:

- `ProgressEvent<'a>` is `#[non_exhaustive]`, derives `Debug, Clone, PartialEq, Eq`, and may carry no floating-point field (`core-reporting`, *Transport-Agnostic Reporter Seam*).
- `core-reporting`, *Structured Progress Stream Semantics*, forbids converting `TranslationEngine`'s per-batch `Message` to `Advanced`: the CLI renders those bytes and its characterisation tests lock them. It also allows at most one `Started`/`Advanced`/`Finished` stream per reporter.
- `Reporter` may gain no supertrait; new capability arrives as an enum variant or a defaulted method.
- `TranslationEngine::translate_subtitle` (`src/core/translation/engine.rs`) issues, per file: one terminology request (`extract_terminology`), one request per chunk from `chunk_ranges`, an optional second request for a chunk whose response had an unknown cue ID (`translate_batch_with_unknown_retry`), and an optional single fill-in request (`retry_missing_translations`). It emits `Message` after each accepted chunk and after the fill-in pass, and a `Message` retry notice before a resend.
- The engine is safe to drop at any `.await`; consumers cancel by dropping the future.

## Goals / Non-Goals

**Goals:**

- A consumer can draw per-file translation progress from typed events alone: no text parsing, no provider wrapping.
- Zero change to any byte the CLI prints and to any existing signature.
- Additive, minor-version change.

**Non-Goals:**

- Token streaming or progress inside one request.
- Carrying cue text, terminology entries or file names in events.
- Cancellation inside `translate_subtitle` through `Reporter::cancelled`; dropping the future remains the mechanism.
- Changing how `subx-cli` renders translation progress.

## Decisions

### D1 — One new variant with a dedicated payload enum

`ProgressEvent::Translation(TranslationProgress)`, with `TranslationProgress` and `TranslationPass` defined in `src/core/report/mod.rs` beside `AiUsage`.

- **Why not reuse `Started`/`Advanced`/`Finished` over cues?** That stream cannot say "the terminology request is in flight", "batch 6 is being resent" or "this is the fill-in pass", which is exactly what a consumer has to guess today. It would also consume the one-stream-per-reporter slot.
- **Why not several top-level variants (`TranslationBatchStarted`, …)?** One variant keeps the generic vocabulary small and lets the translation payload grow under its own `#[non_exhaustive]` enum without touching `ProgressEvent` again.
- **Why in `core::report` and not `core::translation`?** The reporting module must not depend on engine modules; `AiUsage` set the precedent of a core-owned payload living with the seam.

### D2 — Payload is `Copy`, integers and enums only

No lifetime, no `String`. A consumer forwards the value across threads or an IPC channel without cloning. Counts are `u64`, matching `Started`/`Advanced`; `attempt` is `u32`. File identity is not included: the caller knows which file it handed to `translate_subtitle`.

### D3 — Started and finished events, not a single "advanced"

A consumer needs to show the batch in flight between two arrivals several seconds apart, so the send moment must be observable. `BatchStarted` before the request and `BatchFinished` after acceptance give both. The same shape for terminology keeps the phases uniform.

### D4 — A resend is a second `BatchStarted` with `attempt: 2`

The engine resends at most once, so `attempt` is `1` or `2`. Reusing `BatchStarted` avoids a separate "discarded" variant: the consumer learns of the discard from the fact that the same batch starts again. The existing retry notice `Message` stays where it is, immediately before the resend's `BatchStarted`.

### D5 — The fill-in pass is a batch with `pass: FillIn`

It is one request (`batch: 1, batches: 1`) and goes through the same unknown-ID retry, so it reuses both batch variants. Its `BatchFinished` is emitted after the empty-fallback insertion, so `processed_cues == total_cues`, matching the text line emitted at the same place.

### D6 — Emit from `translate_subtitle` only

Emission wraps the calls inside `translate_subtitle` (and therefore `translate_content`, which delegates to it). A direct call to the public `extract_terminology` emits nothing: that method is a preview helper with no surrounding operation to report on.

To keep the batch index and count available at the resend site, `translate_batch_with_unknown_retry` takes a small private descriptor (`pass`, `batch`, `batches`) and emits both `BatchStarted` events itself; `translate_subtitle` emits `BatchFinished` after the returned map is inserted (for the fill-in pass, after the empty-fallback insertion), immediately before the corresponding `Message`.

### D7 — No closing event on failure or drop

A failed request emits no `…Finished`; the error travels in the return value, as `core-reporting` already states for the unit-counted stream. A dropped future emits nothing further. A consumer therefore treats "started without finished" plus an error (or its own cancellation) as the end of that step.

### D8 — Version `1.1.0`

Only public additions. `cargo public-api`-style breakage cannot occur for implementors because both the outer enum and the new enums are `#[non_exhaustive]`.

## Risks / Trade-offs

- **[A consumer matching `TranslationProgress` exhaustively inside this crate's tests could mask the non-exhaustive contract]** → The contract scenario is verified with a `compile_fail` doctest, which compiles as an external crate.
- **[Two channels for the same fact (text line and `BatchFinished`) can drift]** → A test asserts each `BatchFinished` is immediately followed by a `Message` carrying the same two numbers.
- **[`BatchFinished.processed_cues` for the fill-in pass counts empty fallbacks as processed]** → Documented in rustdoc and in the spec; it mirrors the existing text line, and the count of fallbacks is not something a progress display needs.
- **[The release must precede the GUI change]** → The GUI change lists `subx-core` `1.1.0` on crates.io as its prerequisite; until then it can develop against a `[patch.crates-io]` path override that is removed before merge.

## Migration Plan

1. Land the change on `master`, CI green.
2. Bump the `subx-cli` submodule pointer and run its authoritative quality gate; the CLI needs no code change.
3. Tag `v1.1.0` here; `release.yml` publishes to crates.io.
4. The GUI raises its requirement to `subx-core = "1.1"`.

Rollback is a revert: nothing persisted, no format changed.
