# Error Handling

## Purpose

Define the library error surface: the `SubXError` taxonomy, its automatic conversions and chained sources, the library-side `Display` rendering, API error source enumeration, the no-panic obligation for recoverable failures, sanitized upstream messages, the no-sensitive-data rule, the stable machine-readable category and code, and the rule that the library surface holds only machine contracts. The `subx-cli` repository carries the same-named `error-handling` capability holding the presentation half: user-facing formatting, exit-code mapping, top-level rendering, the no-panic obligation on subcommands, and the `SubXErrorExt` binary extension trait. Implemented in `src/error.rs`.

## Requirements

### Requirement: Typed Error Taxonomy

The system SHALL expose a single top-level error enum `SubXError` (`src/error.rs`) covering at minimum the following categories, each represented by a distinct variant: I/O (`Io`), configuration (`Config`), subtitle format (`SubtitleFormat`), AI service (`AiService`, `Api`), audio processing including VAD (`AudioProcessing`), file discovery/matching (`FileMatching`), file-existence outcomes (`FileAlreadyExists`, `FileNotFound`, `InvalidFileName`, `FileOperationFailed`), path handling (`NoInputSpecified`, `InvalidPath`, `PathNotFound`, `DirectoryReadError`), sync validation (`InvalidSyncConfiguration`), unsupported-file-type rejection (`UnsupportedFileType`), generic command execution (`CommandExecution`), and a catch-all `Other(#[from] anyhow::Error)`. The crate SHALL also publish `pub type SubXResult<T> = Result<T, SubXError>`.

#### Scenario: All SubX operations return Result rather than panicking
- **GIVEN** any caller of a fallible library function — `subx-cli`'s command entry points under `subx-cli:src/commands/` are one such caller
- **WHEN** a recoverable failure (invalid input, missing file, network error, decoder error, AI service error, etc.) occurs
- **THEN** the function SHALL return `Err(SubXError::…)` instead of panicking or aborting the process

#### Scenario: Helper constructors exist for common variants
- **GIVEN** the public API of `SubXError`
- **WHEN** application code calls `SubXError::config(msg)`, `SubXError::subtitle_format(fmt, msg)`, `SubXError::audio_processing(msg)`, `SubXError::ai_service(msg)`, or `SubXError::file_matching(msg)`
- **THEN** each helper SHALL produce the corresponding variant whose `Display` output starts with the documented category prefix (e.g., `Configuration error: …`, `Subtitle format error [SRT]: …`, `Audio processing error: …`, `AI service error: …`, `File matching error: …`)

### Requirement: Automatic Error Conversions

The system SHALL provide `From` conversions so that lower-level errors are automatically lifted into `SubXError` at `?` sites: `std::io::Error` → `SubXError::Io` (`#[from]`); `anyhow::Error` → `SubXError::Other` (`#[from]`); `reqwest::Error` → `SubXError::AiService`; `walkdir::Error` → `SubXError::FileMatching`; `symphonia::core::errors::Error` → `SubXError::AudioProcessing`; `config::ConfigError` → `SubXError::Config` (mapping `NotFound` to `Configuration file not found: <path>`); `serde_json::Error` → `SubXError::Config` with a `JSON serialization/deserialization error:` prefix; and `Box<dyn std::error::Error>` → `SubXError::AudioProcessing` (used by the resampler's `Box<dyn Error>` signature).

#### Scenario: std::io::Error lifts into SubXError::Io via ?
- **GIVEN** a function returning `SubXResult<T>` that calls a `std::io` API and propagates with `?`
- **WHEN** the underlying I/O call fails with `io::ErrorKind::NotFound`
- **THEN** the propagated error SHALL match `SubXError::Io(_)`

#### Scenario: Symphonia decode error becomes AudioProcessing
- **GIVEN** a `symphonia::core::errors::Error` produced while decoding
- **WHEN** it is converted into `SubXError` through `From`
- **THEN** the result SHALL match `SubXError::AudioProcessing { .. }` and its `Display` text SHALL start with `Audio processing error:`

#### Scenario: Config crate NotFound error is rewritten with a hint
- **GIVEN** a `config::ConfigError::NotFound("ai.api_key".into())`
- **WHEN** it is converted to `SubXError`
- **THEN** the result SHALL be `SubXError::Config { message }` where `message` contains `Configuration file not found:` and the original path

### Requirement: Chained Error Sources

The system SHALL preserve causal chains via `std::error::Error::source()`. Variants that wrap an underlying error SHALL do so using either `#[from]` (`Io`, `Other`) or `#[source]` (`DirectoryReadError.source`) so that downstream consumers — including integration tests and future logging layers — can walk the chain without losing context.

#### Scenario: DirectoryReadError exposes the originating io::Error
- **GIVEN** a `SubXError::DirectoryReadError { path, source }` produced when reading a directory fails
- **WHEN** a caller inspects `std::error::Error::source()` on the error
- **THEN** the returned reference SHALL be the wrapped `std::io::Error`

### Requirement: Display Is the Library's Error Rendering

`SubXError::Display` (derived via `thiserror`) SHALL produce a concise single-line English message prefixed by the error category. `Display` is an inherent, library-side capability of `SubXError`, available to any caller without importing a trait. All messages and prefixes SHALL be written in English — the same English-language rule the `subx-cli` half states for its presentation layer; it is a project-wide editorial constraint rather than an obligation on a particular function, and it is deliberately stated in both halves.

The multi-line, hinted rendering — the `Hint:` line and the remediation prose — is `subx-cli`'s presentation contract, specified by the `error-handling` capability's *User-Facing Error Formatting* and *Binary Error Surface Adds Presentation Through an Extension Trait* requirements there.

#### Scenario: Display is a single-line English message
- **GIVEN** `SubXError::subtitle_format("SRT", "invalid timestamp")`
- **WHEN** `to_string()` is called
- **THEN** the output SHALL equal `Subtitle format error [SRT]: invalid timestamp` with no embedded newline

### Requirement: API Error Source Enumeration

Errors originating from external HTTP APIs SHALL be modelled as `SubXError::Api { message, source: ApiErrorSource }` where `ApiErrorSource` distinguishes at least `OpenAI` and `Whisper`. The helper `SubXError::whisper_api(msg)` SHALL produce an `Api` variant whose source is `ApiErrorSource::Whisper`, and both `Api` and `AiService` SHALL share the `api` and `ai_service` categories' process exit code; the numeric mapping itself is specified by this capability's *Process Exit Code Mapping* requirement in `subx-cli`, which maps both categories to the same code.

#### Scenario: Whisper API helper carries the Whisper source
- **GIVEN** `SubXError::whisper_api("rate limited")`
- **WHEN** the variant is inspected
- **THEN** it SHALL match `SubXError::Api { source: ApiErrorSource::Whisper, .. }`, and `exit_code()` — `subx-cli`'s extension-trait method per this capability's *Process Exit Code Mapping* requirement there — SHALL return the code that requirement maps for the `api` category (`3`)

### Requirement: Library Code Surfaces Recoverable Failures as Errors

Library code SHALL NOT panic, `unwrap`, or `expect` on conditions that represent user-facing recoverable failures (invalid configuration, missing or unreadable files, unsupported formats, network failures, AI response errors, empty inputs, etc.); every such failure SHALL instead be returned as an appropriately typed `SubXError`. The configuration loader (`src/config/`) and the match engine (`src/core/matcher/`) SHALL both surface invalid input through `SubXError::Config` / `SubXError::FileMatching` rather than aborting, as verified by `tests/config_validation_tests.rs` and `tests/match_engine_error_display_integration_tests.rs`.

The equivalent obligation on `subx-cli`'s command entry points — that subcommands SHALL NOT panic and SHALL propagate typed errors to the process entry point, verified there by `subx-cli:tests/match_engine_error_handling_integration_tests.rs` — is specified by the `error-handling` capability's *No Panics On Recoverable Errors* requirement in `subx-cli`.

#### Scenario: Invalid configuration value is reported, not panicked
- **GIVEN** a configuration value that fails validation (e.g. out-of-range `sync.vad.sensitivity`)
- **WHEN** validation runs
- **THEN** the code path SHALL return `Err(SubXError::Config { .. })` and the code SHALL NOT unwind via panic

### Requirement: Sanitized upstream error messages

When an AI API returns an error response, the system SHALL truncate the error body to a maximum of 500 characters before including it in `SubXError`. The system SHALL strip any HTTP headers or request metadata from the error message to avoid leaking sensitive or excessive information to end users.

#### Scenario: long error body is truncated
- **WHEN** the AI API returns a 10 KiB error body
- **THEN** the `SubXError` message SHALL contain at most 500 characters of the body followed by `... (truncated)`

#### Scenario: short error body is preserved
- **WHEN** the AI API returns a 200-character error body
- **THEN** the full body SHALL be included in the error message

### Requirement: No sensitive data in error chains

Error types and messages SHALL NOT include API keys, authentication tokens, or full request/response URLs that contain query parameters. If a URL must be included, only the scheme, host, and path components SHALL be shown.

#### Scenario: error with URL strips query params
- **WHEN** an HTTP error includes a URL with query parameters
- **THEN** the error message SHALL show only `scheme://host/path`

#### Scenario: API key never in error message
- **WHEN** an AI service error occurs
- **THEN** the error message and its chain SHALL NOT contain any API key value

### Requirement: Stable Machine-Readable Category and Code

`SubXError` SHALL expose three pure helper methods on every variant, as **inherent** methods available without importing any trait:

- `pub fn category(&self) -> &'static str` returning a stable snake_case identifier from the closed set: `io`, `config`, `subtitle_format`, `ai_service`, `api`, `audio_processing`, `file_matching`, `file_already_exists`, `file_not_found`, `invalid_file_name`, `file_operation_failed`, `command_execution`, `no_input_specified`, `invalid_path`, `path_not_found`, `directory_read_error`, `invalid_sync_configuration`, `unsupported_file_type`, `other`.
- `pub fn machine_code(&self) -> &'static str` returning a stable upper-snake-case identifier prefixed with `E_` (for example `E_IO`, `E_CONFIG`, `E_SUBTITLE_FORMAT`, `E_AI_SERVICE`, `E_API`, `E_AUDIO_PROCESSING`, `E_FILE_MATCHING`, `E_FILE_ALREADY_EXISTS`, `E_FILE_NOT_FOUND`, `E_INVALID_FILE_NAME`, `E_FILE_OPERATION_FAILED`, `E_COMMAND_EXECUTION`, `E_NO_INPUT_SPECIFIED`, `E_INVALID_PATH`, `E_PATH_NOT_FOUND`, `E_DIRECTORY_READ_ERROR`, `E_INVALID_SYNC_CONFIGURATION`, `E_UNSUPPORTED_FILE_TYPE`, `E_OTHER`).
- `pub fn hint(&self) -> Option<&'static str>` returning a short remediation string, or `None` where none applies.

The `category()` and `machine_code()` implementations SHALL use an exhaustive `match` (no wildcard arm) so the compiler enforces updates whenever a new variant is added; the `OutputModeUnsupported` variant SHALL therefore remain part of the enum. All three helpers SHALL be pure (no I/O, no allocation) and SHALL NOT change `Display`, `SubXErrorExt::exit_code`, or `SubXErrorExt::user_friendly_message` — the latter two being `subx-cli`'s extension-trait methods, specified by the `error-handling` capability there.

Because these three are the contract consumed by non-terminal front ends, they SHALL remain inherent methods on `SubXError` and SHALL NOT be moved to the binary's extension trait — including `hint()`, whose prose is deliberately CLI-flavoured (see the "Library Error Surface Holds Only Machine Contracts" requirement).

#### Scenario: Every variant has a category and machine code
- **GIVEN** any `SubXError` value
- **WHEN** `category()` and `machine_code()` are called
- **THEN** both calls SHALL return non-empty `&'static str` values from the documented closed sets

#### Scenario: Category and exit code mapping are consistent
- **GIVEN** a `SubXError::AiService(_)` value
- **WHEN** `category()` and `machine_code()` are called, and `exit_code()` — `subx-cli`'s extension-trait method per this capability's *Process Exit Code Mapping* requirement there — is called on the same value
- **THEN** the three SHALL return `"ai_service"`, `"E_AI_SERVICE"`, and `3` respectively

#### Scenario: Adding a new variant breaks the build until mapped
- **GIVEN** the source code is modified to add a new `SubXError` variant
- **WHEN** the crate is compiled
- **THEN** compilation SHALL fail in `category()` and `machine_code()` due to the exhaustive match, forcing the contributor to assign stable identifiers

#### Scenario: Hint is reachable without the presentation trait
- **GIVEN** a consumer that imports only `SubXError` and no extension trait
- **WHEN** it calls `err.hint()`
- **THEN** the call SHALL compile and SHALL return `Some(_)` for exactly the variants that returned `Some(_)` before the split

### Requirement: Library Error Surface Holds Only Machine Contracts

The library half of the `SubXError` surface SHALL hold the machine contracts and nothing else: the enum itself and all of its variants, every `From` conversion, every helper constructor, and `ApiErrorSource`; `category()`, `machine_code()`, and `hint()`. `Display` is unchanged by the surface split: every message, prefix, and newline is byte-identical to the pre-split rendering.

Additional constraints:

- Code under `src/core/` and `src/services/` SHALL NOT call `exit_code()` or `user_friendly_message()`, and SHALL NOT import `SubXErrorExt` (defined at `subx-cli:src/cli/error_ext.rs`). Where such code needs a rendered message it SHALL use `Display` (`to_string()`), optionally combined with `hint()`.
- `hint()` SHALL remain an inherent method on `SubXError` even though its returned prose names the `subx-cli` binary and its flags. Its rustdoc SHALL record that the text is written for the terminal, that library consumers should treat the return value as an availability signal rather than display copy, and that the identity of the variants returning `Some` is the stable part of the contract.
- The `OutputModeUnsupported` variant SHALL remain a variant of the core enum even though only the binary constructs it, so that `category()` and `machine_code()` keep their wildcard-free exhaustive matches. Its rustdoc SHALL record that only the binary constructs it.

The binary half — `SubXErrorExt` with its two methods, their unchanged bodies so that no exit code, message, prefix, or `Hint:` line differs from before the split, and the trait import sites — is specified by the `error-handling` capability's *Binary Error Surface Adds Presentation Through an Extension Trait* requirement in `subx-cli`.

#### Scenario: Machine contracts need no import
- **GIVEN** any module holding a `SubXError` value, with no trait imported
- **WHEN** it calls `err.category()`, `err.machine_code()`, or `err.hint()`
- **THEN** all three calls SHALL compile and SHALL return the same values as before the split

#### Scenario: Core does not depend on presentation
- **GIVEN** the source trees `src/core/` and `src/services/`
- **WHEN** they are searched for `SubXErrorExt`, `exit_code`, and `user_friendly_message`
- **THEN** there SHALL be no call site and no import of any of them

#### Scenario: Core renders operation errors through Display
- **GIVEN** the audit path in `src/core/matcher/engine.rs` that turns a failed file operation into per-operation error metadata
- **WHEN** it renders the error's `message` field
- **THEN** it SHALL use the error's `Display` output, and for the only variant it constructs — `SubXError::FileOperationFailed` — that output SHALL be byte-identical to `user_friendly_message()`, preserving the per-item message contract of the `machine-readable-output` capability in `subx-cli`

