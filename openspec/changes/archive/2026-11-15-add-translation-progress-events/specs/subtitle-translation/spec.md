# Spec Delta

## ADDED Requirements

### Requirement: Structured Terminology Progress

While translating a subtitle, the engine SHALL report the terminology pass through its `Reporter` as `ProgressEvent::Translation` events: `TerminologyStarted` immediately before the terminology request is sent, and `TerminologyFinished { terms }` once the response has been accepted, where `terms` is the number of terms that response supplied before any glossary entry is merged.

#### Scenario: terminology events bracket the request

- **GIVEN** an engine with a recording `Reporter` and a subtitle with at least one cue
- **WHEN** the file is translated successfully
- **THEN** the first two structured events SHALL be `TerminologyStarted` and then `TerminologyFinished`
- **AND** the provider SHALL have received the terminology request after `TerminologyStarted` and before `TerminologyFinished`

#### Scenario: terms counts the generated map only

- **GIVEN** a terminology response supplying two terms and a request carrying three glossary entries
- **WHEN** the terminology pass completes
- **THEN** the event SHALL be `TerminologyFinished { terms: 2 }`

#### Scenario: failed terminology request emits no finish

- **GIVEN** a provider whose terminology request fails
- **WHEN** the file is translated
- **THEN** `TerminologyStarted` SHALL have been reported, no `TerminologyFinished` SHALL follow, and the failure SHALL be carried by the returned error

### Requirement: Structured Batch Progress

For every cue batch of the initial pass the engine SHALL report `BatchStarted` immediately before the batch request is sent and `BatchFinished` once that batch's response has been accepted. Both SHALL carry `pass: TranslationPass::Initial`, the 1-based `batch` index and `batches`, the number of batches the initial pass will issue. `BatchStarted.cues` SHALL be the number of cues in that batch, and `BatchFinished` SHALL carry the processed and total cue counts.

#### Scenario: batch events for a multi-batch file

- **GIVEN** a subtitle with 250 cues, batch size 100, and a recording `Reporter`
- **WHEN** translation completes with every cue returned
- **THEN** the reporter SHALL have received, in order, `BatchStarted` and `BatchFinished` for `batch` 1, 2 and 3, each with `batches: 3`
- **AND** the `BatchStarted.cues` values SHALL be 100, 100 and 50
- **AND** the `BatchFinished.processed_cues` values SHALL be 100, 200 and 250, each with `total_cues: 250`

#### Scenario: batches follow the terminology pass

- **GIVEN** a recording `Reporter`
- **WHEN** a file is translated
- **THEN** no `BatchStarted` SHALL be reported before `TerminologyFinished`

#### Scenario: processed count excludes missing cues

- **GIVEN** a batch of 100 cues whose response omits 4 of them
- **WHEN** that response is accepted
- **THEN** its `BatchFinished.processed_cues` SHALL have grown by 96, not 100

#### Scenario: failed batch emits no finish

- **GIVEN** a batch whose request fails with an error other than an unknown cue ID
- **WHEN** the file is translated
- **THEN** that batch's `BatchStarted` SHALL have been reported, no `BatchFinished` SHALL follow it, and the failure SHALL be carried by the returned error

### Requirement: Structured Resend Attempt

`BatchStarted.attempt` SHALL be `1` for the first request of a batch. When a batch response is discarded for an unknown cue ID and the batch is sent again, the engine SHALL report a second `BatchStarted` for the same `pass` and `batch` with `attempt: 2`, after the existing retry notice. A batch SHALL be reported finished at most once, however many attempts it took.

#### Scenario: resend is announced with attempt 2

- **GIVEN** a batch whose first response contains an unknown cue ID and whose second response is valid
- **WHEN** the file is translated
- **THEN** the reporter SHALL have received `BatchStarted { attempt: 1, .. }`, then the retry notice `Message`, then `BatchStarted { attempt: 2, .. }` with the same `pass`, `batch`, `batches` and `cues`, then exactly one `BatchFinished` for that batch

#### Scenario: second unknown cue ID fails the file

- **GIVEN** a batch whose first and second responses both contain an unknown cue ID
- **WHEN** the file is translated
- **THEN** no `BatchFinished` SHALL be reported for that batch and the file SHALL fail as it does today

### Requirement: Structured Fill-In Pass

When cues are still missing after the initial pass, the engine SHALL report the request that re-asks for them as a batch with `pass: TranslationPass::FillIn`, `batch: 1` and `batches: 1`. Its `BatchStarted.cues` SHALL be the number of missing cues, and its `BatchFinished.processed_cues` SHALL equal `total_cues`, because every cue then holds a result or the empty fallback. No fill-in event SHALL be reported when nothing is missing.

#### Scenario: fill-in pass is reported after the initial pass

- **GIVEN** a 250-cue file whose initial pass leaves 4 cues missing
- **WHEN** the file is translated
- **THEN** after the last initial `BatchFinished` the reporter SHALL receive `BatchStarted { pass: FillIn, batch: 1, batches: 1, cues: 4, attempt: 1 }` and then `BatchFinished { pass: FillIn, batch: 1, batches: 1, processed_cues: 250, total_cues: 250 }`

#### Scenario: no fill-in events when every cue returned

- **GIVEN** a file whose initial pass returns every cue
- **WHEN** the file is translated
- **THEN** no event with `pass: FillIn` SHALL be reported

### Requirement: Structured Events Accompany the Text Progress

The structured events SHALL be emitted in addition to the existing `ProgressEvent::Message` lines, which SHALL keep their exact bytes and call sites. Each `BatchFinished` SHALL be emitted immediately before the `Processed cues: <processed>/<total>` line it corresponds to and SHALL carry the same two numbers. The events are not a `Started`/`Advanced`/`Finished` stream and SHALL NOT open one.

#### Scenario: BatchFinished precedes its text line

- **GIVEN** a recording `Reporter` and a file needing two batches
- **WHEN** the file is translated
- **THEN** each `BatchFinished { processed_cues, total_cues, .. }` SHALL be immediately followed by a `Message` whose text contains `Processed cues: <processed_cues>/<total_cues>`

#### Scenario: text output is unchanged

- **GIVEN** a `Reporter` that records only `ProgressEvent::Message` events
- **WHEN** a file is translated before and after this change with the same scripted provider
- **THEN** the recorded messages SHALL be identical

#### Scenario: no unit-counted stream is opened

- **GIVEN** a recording `Reporter`
- **WHEN** a file is translated
- **THEN** it SHALL have received no `ProgressEvent::Started`, `Advanced` or `Finished` event

#### Scenario: nothing to translate reports nothing

- **GIVEN** a subtitle with no cues, or a request with an empty target language
- **WHEN** translation is requested
- **THEN** no `ProgressEvent::Translation` event SHALL be reported

#### Scenario: engine without a reporter stays silent

- **GIVEN** an engine constructed with no reporter attached
- **WHEN** a file is translated
- **THEN** the result SHALL be identical to the result before this change and no bytes SHALL be written to stdout or stderr by the progress path
