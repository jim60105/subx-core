# Encoding Detection

## Purpose

Define the core encoding-detection behavior: the detector's tolerance of empty and binary input without panicking, and the low-confidence fallback to the configured default encoding. Implemented in this repository's charset-detection module (per the archived requirements' own path citations). The `subx-cli` repository carries the same-named `encoding-detection` capability holding the CLI halves: the per-file encoding report, input source selection, verbose sample output, the batch-level robustness framing, legacy positional paths, and the structured JSON payload.
## Requirements

### Requirement: Detector Tolerates Empty and Binary Input

The charset detector SHALL NOT panic when its input is empty or contains binary (non-text) bytes: for such input it SHALL return either a normal detection result (a best-effort encoding with its confidence) or a typed per-file error, so that a caller processing a batch of files retains full control over whether the batch continues. The batch loop's own resilience and the process exit status are specified by the `encoding-detection` capability's *Robust Handling of Empty and Binary Files* requirement in `subx-cli`.

#### Scenario: Detector on empty input does not panic
- **GIVEN** a zero-byte buffer supplied to the detector
- **WHEN** detection runs
- **THEN** the detector SHALL return a result — a best-effort detection or an error — and SHALL NOT panic or abort the process

#### Scenario: Detector on binary input does not panic
- **GIVEN** a buffer containing binary (non-text) bytes supplied to the detector
- **WHEN** detection runs
- **THEN** the detector SHALL return a best-effort detection result or a per-file error, and SHALL NOT panic

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

