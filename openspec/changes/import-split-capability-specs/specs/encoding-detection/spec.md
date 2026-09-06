## ADDED Requirements

### Requirement: Low-Confidence Fallback To Default Encoding

When no encoding candidate scores above `formats.encoding_detection_confidence`, the detector SHALL fall back to the configured default encoding (e.g. UTF-8), SHALL report a fixed fallback confidence of `0.5`, and SHALL prefix the sample text with a `Low confidence detection, using default:` marker. When there are no candidates at all, the fallback SHALL instead use confidence `0.1` and prefix the sample with `Unable to detect encoding, using default:`. Implemented in `src/core/formats/encoding/detector.rs::select_best_encoding`.

#### Scenario: Best candidate below threshold
- **GIVEN** a byte sequence whose highest-scoring encoding candidate has a confidence strictly less than `formats.encoding_detection_confidence`
- **WHEN** the encoding detector selects a result
- **THEN** the returned `EncodingInfo.charset` SHALL be the configured default encoding, `EncodingInfo.confidence` SHALL equal `0.5`, and `EncodingInfo.sample_text` SHALL start with `Low confidence detection, using default:`

#### Scenario: No viable candidates at all
- **GIVEN** a byte sequence for which no charset yields a confidence above the internal lower bound
- **WHEN** the encoding detector selects a result
- **THEN** the returned `EncodingInfo.confidence` SHALL equal `0.1` and `EncodingInfo.sample_text` SHALL start with `Unable to detect encoding, using default:`

