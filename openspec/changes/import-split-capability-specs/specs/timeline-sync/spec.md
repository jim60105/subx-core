## ADDED Requirements

### Requirement: Offset Clamping Against Maximum

The system SHALL enforce `sync.max_offset_seconds`: manual offsets exceeding this absolute value SHALL be rejected with an error, and VAD-detected offsets exceeding it SHALL be clamped (preserving sign) and accompanied by a warning in the sync result.

The guard's location is normative, because the transform it guards is reachable independently:

- The manual-offset guard SHALL live on `SyncEngine::apply_manual_offset`, which reads `self.config.max_offset_seconds` and returns the configuration error before any entry is modified.
- The guard SHALL NOT be moved into `shift_subtitle_timing`. That function is specified as unguarded (*VAD-Independent Manual Offset Application*) so that a host which validates the offset against its own presentation of `sync.max_offset_seconds` — for example in whole milliseconds, to keep its own pre-check and the library's check on the same side of a rounding boundary — is not subjected to a second, differently-rounded check.
- A caller reaching the transform directly SHALL be responsible for its own bound. This division SHALL be stated in both items' rustdoc.

#### Scenario: Manual offset exceeds maximum
- **GIVEN** `sync.max_offset_seconds = 60` and the user supplies `--offset 120`
- **WHEN** `apply_manual_offset` runs
- **THEN** the call SHALL return a configuration error referencing `sync.max_offset_seconds` and the subtitle entries SHALL remain unchanged

#### Scenario: VAD offset clamping
- **GIVEN** `sync.max_offset_seconds = 30` and VAD detects an offset of 45s
- **WHEN** `vad_detect_sync_offset` returns
- **THEN** the resulting `SyncResult.offset_seconds` SHALL equal 30 (sign preserved), `SyncResult.warnings` SHALL contain a message explaining the clamping, and `additional_info` SHALL record the original and clamped values

#### Scenario: The guard is not duplicated into the free function
- **GIVEN** a proposal to add the `sync.max_offset_seconds` check to `shift_subtitle_timing`
- **WHEN** it is evaluated against this requirement
- **THEN** it SHALL be rejected, because the free function has no `SyncConfig` and adding one would restore the construction-time configuration dependency the function exists to avoid

### Requirement: Subtitle Timing Application

The system SHALL shift every subtitle entry's start and end time by the applied offset, clamping negative results to zero rather than producing negative timestamps.

The transform SHALL have exactly one implementation:

- The shift SHALL be implemented once, in `shift_subtitle_timing` (`src/core/sync/mod.rs`). A positive offset SHALL use a checked addition and SHALL return an audio-processing error if any entry's time would overflow; a negative offset SHALL saturate at `Duration::ZERO` rather than erroring.
- The returned `SyncResult` SHALL carry `offset_seconds` as supplied, `confidence = 1.0`, `method_used = SyncMethod::Manual`, `correlation_peak = 1.0`, an `additional_info` object recording the applied offset and the number of entries modified, and the measured processing duration.
- `SyncEngine::apply_manual_offset` SHALL obtain both the shift and the `SyncResult` from that function and SHALL NOT construct either itself.

#### Scenario: Positive offset delays subtitles
- **GIVEN** a subtitle entry with `start_time = 10s` and the engine applies a +2.5s offset
- **WHEN** `apply_manual_offset` runs
- **THEN** the entry's new `start_time` SHALL be 12.5s

#### Scenario: Negative offset clamps to zero
- **GIVEN** a subtitle entry with `start_time = 1s` and the engine applies a -5s offset within the maximum
- **WHEN** `apply_manual_offset` runs
- **THEN** the entry's new `start_time` SHALL be `Duration::ZERO` rather than a negative value

#### Scenario: Positive offset beyond the representable range is rejected
- **GIVEN** a subtitle entry whose `end_time` is `Duration::MAX` and a positive offset within `sync.max_offset_seconds`
- **WHEN** the shift is applied through either entry point
- **THEN** an audio-processing error SHALL be returned rather than a wrapped or truncated timestamp

### Requirement: VAD Audio Processing

The system SHALL provide `VadAudioProcessor` that loads an audio or video file, downmixes multi-channel audio to mono, preserves the file's original sample rate in the returned `AudioInfo`, and converts f32 PCM samples to i16. An invalid or non-existent path SHALL return an error rather than panic. Implemented in `src/services/vad/` and exercised by `tests/vad_audio_processor_tests.rs` and `tests/vad_integration_tests.rs`.

#### Scenario: Multi-channel input downmixed and sample rate preserved
- **GIVEN** a WAV input with sample rate 44100 Hz and two channels
- **WHEN** `VadAudioProcessor::load_and_prepare_audio_direct` processes it
- **THEN** the returned `AudioData.info.sample_rate` SHALL equal 44100 and `AudioData.info.channels` SHALL equal 1

#### Scenario: Float32 PCM converted to i16
- **GIVEN** a WAV file with one f32 sample equal to `0.5`
- **WHEN** the processor loads and prepares the file
- **THEN** the returned `samples[0]` SHALL be an `i16` value within 10 of `i16::MAX / 2`

#### Scenario: Non-existent audio path errors
- **GIVEN** a path that does not exist
- **WHEN** `load_and_prepare_audio_direct` is called
- **THEN** the call SHALL return an error rather than panic

### Requirement: VAD Detector Behavior

`LocalVadDetector` SHALL detect speech segments whose `start_time < end_time` and whose durations are bounded below by the configured `min_speech_duration_ms`; higher `sensitivity` SHALL produce at least as many segments as lower sensitivity on the same input. `VadSyncDetector::detect_sync_offset` SHALL return an error when the provided subtitle has no entries, and SHALL return `SyncMethod::LocalVad` as the method used.

#### Scenario: Sensitivity monotonicity
- **GIVEN** the same audio input processed by two detectors configured with `sensitivity = 0.1` and `sensitivity = 0.9`
- **WHEN** both detectors run
- **THEN** the segment count at low sensitivity SHALL be less than or equal to the segment count at high sensitivity (tolerating a difference of at most 1)

#### Scenario: Empty subtitle rejected
- **GIVEN** a subtitle with zero entries
- **WHEN** `VadSyncDetector::detect_sync_offset` is awaited
- **THEN** it SHALL return an error whose message contains `No subtitle entries found`

### Requirement: First-Sentence Offset Annotation

When VAD-assisted sync computes an offset by aligning the first detected speech segment with the first subtitle entry, the resulting `SyncResult` SHALL populate `additional_info` with both `first_speech_start` and `expected_subtitle_start`, such that `offset_seconds == first_speech_start - expected_subtitle_start` (within a small tolerance). Exercised by `tests/sync_first_sentence_offset_integration_tests.rs`.

#### Scenario: Offset equals first-speech minus expected-start
- **GIVEN** a real audio+subtitle asset pair and a VAD-enabled `SyncEngine`
- **WHEN** `detect_sync_offset(..., Some(SyncMethod::Auto))` returns a result
- **THEN** `additional_info.first_speech_start - additional_info.expected_subtitle_start` SHALL equal `offset_seconds` within ±0.01

### Requirement: VAD Padding Chunks Configuration

The system SHALL apply `sync.vad.padding_chunks` (default `3`) as the number of non-speech chunks included before and after each detected speech segment when `LocalVadDetector` labels audio via the VAD backend. A change in `padding_chunks` SHALL be passed through to the VAD labeling step without requiring any other configuration change. Implemented in `src/services/vad/detector.rs` (the `vad.label(..., padding_chunks, ...)` call) and defined in `src/config/mod.rs::VadConfig`.

#### Scenario: Configured padding is applied to the VAD backend
- **GIVEN** `sync.vad.padding_chunks = 5` in configuration
- **WHEN** `LocalVadDetector` runs VAD labeling over an audio buffer
- **THEN** it SHALL invoke the underlying VAD label function with the padding-chunks argument equal to `5`

### Requirement: VAD-Independent Manual Offset Application

The manual-offset timing transform SHALL be reachable without constructing a `SyncEngine`, so that a caller who never performs detection is not subject to the engine's VAD precondition.

- The library SHALL expose `shift_subtitle_timing(subtitle: &mut Subtitle, offset_seconds: f32) -> Result<SyncResult>` as a free function in `subx_core::core::sync` (`src/core/sync/mod.rs`), alongside `resolve_sync_pairing` and `create_default_output_path`.
- The function SHALL apply the offset to every entry with the semantics defined by *Subtitle Timing Application* and SHALL return the `SyncResult` defined there.
- The function SHALL NOT read `sync.max_offset_seconds` and SHALL NOT require, construct, or consult a VAD detector, an audio processor, a `SyncConfig`, or a `ConfigService`. Its only inputs are the subtitle and the offset.
- `SyncEngine::apply_manual_offset` SHALL delegate to this function after performing its own guard, so that exactly one implementation of the transform exists. The two entry points SHALL produce identical `SyncResult` field values and identical entry timings for any input the guard admits.
- The function's rustdoc SHALL state that `sync.max_offset_seconds` is not enforced and SHALL name `SyncEngine::apply_manual_offset` as the entry point that enforces it, so a caller choosing between them is told which obligation it is taking on.

#### Scenario: A caller with no VAD available applies a manual offset
- **GIVEN** a host with `sync.vad.enabled = false`, for which `SyncEngine::new` returns a configuration error
- **WHEN** the host calls `shift_subtitle_timing` with a parsed subtitle and an offset
- **THEN** the offset SHALL be applied and a `SyncResult` with `method_used = SyncMethod::Manual` SHALL be returned, with no engine constructed and no VAD detector involved

#### Scenario: The free function does not enforce the configured maximum
- **GIVEN** `sync.max_offset_seconds = 60` in configuration and a call to `shift_subtitle_timing` with an offset of 120 seconds
- **WHEN** the function runs
- **THEN** it SHALL apply the 120-second offset and SHALL NOT return a configuration error, because the guard belongs to `SyncEngine::apply_manual_offset` and the function is documented as unguarded

#### Scenario: The two entry points cannot drift
- **GIVEN** a subtitle and an offset within `sync.max_offset_seconds`
- **WHEN** the same input is passed to `SyncEngine::apply_manual_offset` and to `shift_subtitle_timing`
- **THEN** the resulting entry timings SHALL be identical and the returned `SyncResult`'s `offset_seconds`, `confidence`, `method_used`, `correlation_peak`, `additional_info` and `warnings` SHALL be identical

