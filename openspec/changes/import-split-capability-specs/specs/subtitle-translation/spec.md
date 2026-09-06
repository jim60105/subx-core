## ADDED Requirements

### Requirement: Subtitle Structure Preservation

The translate command SHALL preserve subtitle cue timing, cue ordering, cue
count, and format metadata that is representable by the existing parser/writer
pipeline while replacing only translatable user-visible text.

#### Scenario: SRT timing is preserved

- **GIVEN** an SRT file with two cues and valid time ranges
- **WHEN** the command translates it successfully
- **THEN** the output SHALL contain two cues in the same order
- **AND** each cue SHALL retain its original start and end timestamp

#### Scenario: ASS supported style fields are preserved

- **GIVEN** an ASS file with style definitions and dialogue lines containing
  visible text
- **WHEN** the command translates it successfully
- **THEN** the output SHALL retain style definitions supported by the current
  ASS parser/writer
- **AND** dialogue timing/style fields SHALL remain unchanged except for the
  translated visible text field

#### Scenario: unsupported styling is not promised

- **GIVEN** a subtitle file contains styling constructs that the current
  parser/writer cannot represent
- **WHEN** the command translates it
- **THEN** the command SHALL NOT promise preservation beyond the existing
  format parser/writer behavior

### Requirement: AI Provider Translation

The translate command SHALL use the configured AI provider and existing AI
request safety behavior to translate cue text through structured prompts and
parseable responses.

#### Scenario: configured provider is used

- **GIVEN** `ai.provider = "openrouter"` in configuration
- **WHEN** a single-file translation is requested — in practice via `subx-cli`'s `subx translate movie.srt --target-language ko`
- **THEN** translation requests SHALL be sent through the OpenRouter provider
  constructed by `ComponentFactory`

#### Scenario: malformed AI response is rejected

- **GIVEN** the AI provider returns a response that is not valid translation
  JSON
- **WHEN** the translation of one file parses the response
- **THEN** that translation SHALL return a typed AI service error for that file
- **AND** SHALL NOT produce a partial translated subtitle for that file

### Requirement: Stable Cue Mapping

The system SHALL include stable cue identifiers in translation requests and
SHALL validate AI responses against requested cue IDs before applying translated
text.

#### Scenario: cue IDs use UUIDv7

- **GIVEN** a subtitle file contains multiple cues
- **WHEN** the command assigns cue IDs for translation requests
- **THEN** each cue ID SHALL be a UUIDv7 value
- **AND** the IDs SHALL be assigned in subtitle cue order

#### Scenario: UUIDv7 timestamps strictly increase

- **GIVEN** cue ID generation assigns IDs to adjacent subtitle cues
- **WHEN** the generator creates the next UUIDv7 cue ID
- **THEN** the generator SHALL intentionally wait at least 1ms after the
  previous UUIDv7 cue ID generation
- **AND** the next UUIDv7 `unix_time_ts` SHALL be greater than the previous
  cue ID's `unix_time_ts`

#### Scenario: missing cue translation is retried once after initial batches

- **GIVEN** a translation batch contains UUIDv7 cue IDs `A`, `B`, and `C`
- **WHEN** the AI response omits cue ID `B`
- **THEN** the command SHALL keep translations for `A` and `C`
- **AND** continue translating all remaining initial batches
- **AND** resend cue `B` for translation once after all initial batches complete

#### Scenario: missing cue retry falls back to empty text

- **GIVEN** cue ID `B` was omitted from the initial translation response
- **AND** the one retry response also omits cue ID `B`
- **WHEN** the command applies translated text
- **THEN** cue `B` SHALL be written with an empty text value
- **AND** the translated subtitle output SHALL still be generated

#### Scenario: unknown cue ID retries the discarded batch once

- **GIVEN** a translation batch contains UUIDv7 cue IDs `A` and `B`
- **WHEN** the AI response includes unknown UUIDv7 cue ID `Z`
- **THEN** the command SHALL treat the response as hallucinated
- **AND** discard the entire response for that batch
- **AND** retry the same batch once

#### Scenario: unknown cue ID after retry fails the file

- **GIVEN** a translation batch response included unknown cue ID `Z`
- **AND** the one retry response for the same batch also includes an unknown cue
  ID
- **WHEN** translation validates the retry response
- **THEN** the command SHALL fail that file with an AI service error
- **AND** no translated output SHALL be written for that file

### Requirement: Two-Pass Terminology Consistency

Before translating subtitle cue batches, the translate command SHALL perform a
terminology extraction pass that identifies proper nouns such as person names
and place names, returns a structured source-to-target terminology map, and
then includes that map in subsequent translation prompts.

#### Scenario: terminology extraction runs before translation

- **GIVEN** a subtitle file contains recurring names such as `Alice` and
  `Wonderland`
- **WHEN** the command translates the file
- **THEN** the command SHALL send a terminology-extraction request before the
  cue translation requests
- **AND** the extraction response SHALL be parsed as a structured terminology
  map

#### Scenario: translation prompt includes terminology map

- **GIVEN** the terminology extraction pass returns `Alice -> 愛麗絲`
- **WHEN** the command builds a cue translation prompt
- **THEN** the prompt SHALL include the terminology map
- **AND** SHALL instruct the provider to use `愛麗絲` whenever translating
  `Alice`

#### Scenario: established translations are preferred

- **GIVEN** a proper noun has an established conventional translation in the
  target language
- **WHEN** the command builds the terminology extraction prompt
- **THEN** the prompt SHALL instruct the provider to use the established
  conventional translation

#### Scenario: coined translations prefer transliteration

- **GIVEN** a proper noun has no established conventional translation
- **WHEN** the command builds the terminology extraction prompt
- **THEN** the prompt SHALL instruct the provider to prefer phonetic
  transliteration before semantic translation

#### Scenario: explicit glossary overrides generated terminology

- **GIVEN** the glossary entries supplied by the caller map `Alice -> 艾莉絲`
- **AND** the generated terminology map contains `Alice -> 愛麗絲`
- **WHEN** the command builds the cue translation prompt
- **THEN** the prompt SHALL use the glossary mapping `Alice -> 艾莉絲` as the
  effective terminology entry

#### Scenario: empty terminology map is allowed

- **GIVEN** the terminology extraction pass returns an empty valid map
- **WHEN** the command translates cue batches
- **THEN** translation SHALL proceed without terminology entries

### Requirement: Batching and Ordering

The translate command SHALL split large subtitles into configurable cue batches
for AI requests and SHALL reassemble translated entries in their original order.

#### Scenario: multiple batches preserve final order

- **GIVEN** a subtitle file has 250 cues and translation batch size is 100
- **WHEN** translation completes successfully for all batches
- **THEN** the output SHALL contain all 250 cues in the original order

#### Scenario: translation progress logs processed cue count

- **GIVEN** a subtitle file has 250 cues and translation batch size is 100
- **WHEN** each translation response is accepted
- **THEN** the engine SHALL report the number of processed cues and total cues
  through its `Reporter` in the form `Processed cues: <processed>/<total>`, per
  the `core-reporting` capability's progress-vocabulary requirement in this
  repository

#### Scenario: one malformed batch prevents partial output

- **GIVEN** a subtitle file requires three translation batches
- **WHEN** the second batch fails validation due to malformed JSON, duplicate
  IDs, or unknown IDs after its one retry
- **THEN** the command SHALL report the file as failed
- **AND** SHALL NOT write an output file containing only the first batch
  translations

### Requirement: Translation Prompt Guidance Inputs

The translation engine SHALL accept optional caller-supplied guidance — a source language, glossary entries, and inline context — and SHALL include it in the translation prompt as terminology and tone guidance without changing subtitle timing or file discovery behavior. Inline context SHALL be treated as prompt text and SHALL NOT be interpreted as a filesystem path. An omitted source language SHALL produce a prompt that requests translation from the detected or unspecified source language into the target language. Reading the `--glossary` file, parsing `--context`, and handing the values to the engine are specified by the `subtitle-translation` capability's *Translation Guidance Options* requirement in `subx-cli`.

#### Scenario: glossary entries are included in prompt
- **GIVEN** a translation request carrying caller-supplied glossary entries
- **WHEN** the engine builds the translation prompt
- **THEN** the prompt SHALL include the entries as terminology guidance
- **AND** the engine SHALL still require the AI response to use the structured cue ID mapping

#### Scenario: inline context is included in prompt
- **GIVEN** a translation request built with the inline context `"Use formal business tone"`
- **WHEN** the engine builds the translation prompt
- **THEN** the prompt SHALL include that text as domain or tone guidance
- **AND** SHALL NOT interpret the context value as a filesystem path

#### Scenario: source language is optional
- **GIVEN** a translation request that omits the source language
- **WHEN** the engine builds the translation prompt
- **THEN** the prompt SHALL request translation from the detected or unspecified source language into the target language

### Requirement: Translation Configuration

The system SHALL expose validated translation configuration for default batch
size and optional default target language without requiring a new external
translation service dependency.

#### Scenario: configured batch size is used

- **GIVEN** translation batch size is configured to `50`
- **WHEN** the user runs `subx-cli`'s `subx translate movie.srt --target-language fr`
- **THEN** the engine SHALL split AI translation requests into batches of at
  most 50 cues

#### Scenario: invalid batch size is rejected

- **GIVEN** the user configures translation batch size as `0`
- **WHEN** configuration validation runs
- **THEN** validation SHALL fail with an error describing the invalid batch size
