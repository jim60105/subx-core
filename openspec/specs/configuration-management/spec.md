# Configuration Management

## Purpose

Define the layered configuration model and its core load-and-validate pipeline: the unified schema, the `ConfigService` abstraction, the tolerant load path with its repair mechanism, field-level value validation with boolean flexibility, AI environment-variable and compatibility-variable overrides, custom configuration file paths, legacy sync configuration rejection, service reload, AI provider identifier canonicalization, local-provider validation rules and environment variables, and Unix config-file permission enforcement — all rooted in this repository's `src/config/` tree. The `subx-cli` repository carries the same-named `configuration-management` capability holding the CLI halves: the `config` subcommand operations, the workspace directory override, and sensitive value masking at the display sites.
## Requirements

### Requirement: Unified Configuration Schema

The system SHALL expose a single `Config` structure aggregating `AIConfig`, `FormatsConfig`, `SyncConfig`, `GeneralConfig`, and `ParallelConfig`, serializable to and deserializable from TOML.

#### Scenario: Default configuration is valid
- **GIVEN** a fresh `Config::default()` value
- **WHEN** the defaults are inspected
- **THEN** `config.ai.provider` SHALL equal `"openai"` and `config.formats.default_output` SHALL equal `"srt"`

### Requirement: Configuration Service Abstraction

The system SHALL access all configuration through the `ConfigService` trait rather than global state; production code SHALL use `ProductionConfigService` and tests SHALL use `TestConfigService` (built via `TestConfigBuilder`).

#### Scenario: Command receives an injected service
- **GIVEN** a command dispatcher invocation
- **WHEN** any subcommand executes
- **THEN** the command handler SHALL obtain configuration by calling `config_service.get_config()` on the injected service rather than reading a global or static

### Requirement: Tolerant Configuration Load Path

The configuration loader SHALL provide a tolerant load of `config.toml` that succeeds when the file parses as TOML and each individual field passes field-level validation, even when the contents fail cross-section (strict) validation. The tolerant load SHALL read the configuration from the file only, without applying any environment-variable overlays, so that repair operations always reflect and modify the on-disk state rather than a transient env-merged view.

The tolerant load SHALL NOT silently substitute defaults for malformed individual fields: a file whose field-level values fail validation SHALL fail the tolerant load with a field-level error identifying the malformed field. A file that does not parse as TOML at all SHALL fail with a parse error; no tolerant path reaches it, and manual repair or reset remains the only recovery.

The strict load path used by `get_config`, `reload`, and every consumer that requires a strict-valid configuration SHALL remain unchanged: those entry points SHALL continue to fail with the existing error when the on-disk configuration is strict-invalid, so that strict invariants assumed by command-execution sites are not weakened.

The in-memory configuration cache SHALL only ever hold strict-valid configurations: tolerant loads SHALL NOT populate the cache, and a failed post-mutation validation SHALL NOT alter the cache.

That `subx-cli`'s three `config` repair subcommands use this load path, the post-mutation strict validation performed before the file is written, and everything those commands print are specified by the `configuration-management` capability's *Repair Path For Strict-Invalid Configuration* requirement in `subx-cli`.

#### Scenario: Non-config command still rejects strict-invalid configuration

- **GIVEN** `~/.config/subx/config.toml` is strict-invalid
- **WHEN** the user runs any command other than `config set`, `config get`, `config list`, or `config reset` (for example `subx match` or `subx sync`)
- **THEN** the command SHALL fail at configuration load with the existing strict-validation error, the same error message users see today, and SHALL NOT proceed to the command body

#### Scenario: Cache invariant preserved after failed repair attempt

- **GIVEN** `~/.config/subx/config.toml` is strict-invalid and no `Config` is currently cached
- **WHEN** the user runs `subx config set` with a key/value that does not heal the cross-section error
- **THEN** the command SHALL fail (per the "non-repair edit" scenario of the `subx-cli` requirement) and the in-memory configuration cache SHALL remain empty, so that the next invocation of any other command SHALL still reload and re-validate from disk and SHALL still produce the strict error

#### Scenario: TOML parse failure is still a hard error

- **GIVEN** `~/.config/subx/config.toml` is not valid TOML (for example, an unterminated string)
- **WHEN** the user runs `subx config set ai.provider local`
- **THEN** the command SHALL fail with a parse error, the on-disk file SHALL remain unchanged, and the user SHALL still need to run `subx config reset` or fix the file manually to recover (this scenario lies outside the repair path because the file cannot be loaded into a `Config` struct at all)

#### Scenario: Field-level malformed value in file fails the tolerant load

- **GIVEN** `~/.config/subx/config.toml` is parseable TOML but contains a syntactically broken individual field (for example `ai.base_url = "not a url"`)
- **WHEN** the user runs `subx config set ai.provider local`
- **THEN** the command SHALL fail with a field-level error identifying the malformed field, and the on-disk file SHALL remain unchanged (the tolerant load SHALL NOT silently substitute defaults for malformed individual fields)

### Requirement: Value Validation

The system SHALL validate configuration values at the moment they are set, rejecting out-of-range numerics, empty required strings, and values of the wrong type.

#### Scenario: Invalid value is rejected
- **GIVEN** the user runs `subx config set sync.max_offset_seconds -5`
- **WHEN** the field validator runs
- **THEN** the command SHALL fail with an error explaining the acceptable range and the persisted configuration SHALL remain unchanged

#### Scenario: Enum field rejects out-of-set values
- **GIVEN** the user runs `subx config set sync.default_method whisper`
- **WHEN** the field validator runs
- **THEN** the command SHALL fail because `sync.default_method` only accepts the values `auto`, `vad`, or `manual`, and the persisted configuration SHALL remain unchanged

### Requirement: Boolean Value Flexibility

The system SHALL accept common boolean aliases (`true`/`false`, `1`/`0`, `yes`/`no`, `on`/`off`, `enabled`/`disabled`) when setting boolean-typed keys, treating them as equivalent to the canonical `true`/`false` values.

#### Scenario: Alternative boolean syntax
- **GIVEN** the user runs `subx config set general.backup_enabled yes`
- **WHEN** the command completes
- **THEN** `general.backup_enabled` SHALL be persisted as `true`

### Requirement: AI Environment Variable Overrides

The system SHALL recognize a fixed set of `SUBX_`-prefixed environment variables that map to AI configuration fields, and SHALL apply them over file-backed configuration when present. Implemented in `src/config/service.rs:287-319` (the partial-load path over the `config` crate's `Environment` source). The supported variables are exactly:

- `SUBX_AI_APIKEY` → `ai.api_key`
- `SUBX_AI_PROVIDER` → `ai.provider`
- `SUBX_AI_MODEL` → `ai.model`
- `SUBX_AI_BASE_URL` → `ai.base_url`

#### Scenario: AI provider overridden by environment
- **GIVEN** the configuration file sets `ai.provider = "openai"` and the process environment has `SUBX_AI_PROVIDER=openrouter`
- **WHEN** `ProductionConfigService` loads configuration
- **THEN** `config.ai.provider` SHALL equal `"openrouter"`

### Requirement: Custom Configuration File Path

The system SHALL honor the `SUBX_CONFIG_PATH` environment variable as an override for the configuration file location used by `ProductionConfigService`, instead of the default platform config directory.

#### Scenario: Custom path loaded
- **GIVEN** `SUBX_CONFIG_PATH` points to an existing TOML file with valid settings
- **WHEN** `ProductionConfigService` is constructed
- **THEN** it SHALL read configuration from the custom path rather than from the default `dirs::config_dir()` location

### Requirement: Legacy Sync Configuration Rejected

The system SHALL reject legacy `[sync]` TOML that lacks the new required fields (such as `default_method`), failing to deserialize into `Config` rather than silently applying partial defaults. Implemented by the `SyncConfig` schema and exercised by `tests/config_migration_tests.rs`.

#### Scenario: Old sync schema fails parsing
- **GIVEN** a TOML document with `[sync] max_offset_seconds = 10.0` and `correlation_threshold = 0.8` but no `default_method`
- **WHEN** the document is deserialized into `Config`
- **THEN** deserialization SHALL fail with a parse error

### Requirement: Config Service Reload

`ProductionConfigService` SHALL expose a `reload()` operation that re-reads configuration (file and environment) and SHALL produce a subsequent `get_config()` result consistent with the latest on-disk and environment state.

#### Scenario: Reload returns fresh configuration
- **GIVEN** a `ProductionConfigService` has been constructed and has returned a configuration once
- **WHEN** `service.reload()` is called and then `service.get_config()` is called again
- **THEN** the second call SHALL succeed and SHALL reflect any applicable on-disk or environment changes without restarting the process

### Requirement: Compatibility Environment Variables For Third-Party Providers

In addition to `SUBX_AI_*` overrides, `ProductionConfigService` SHALL recognize industry-standard environment variables for each supported provider and apply them on top of the loaded configuration: `OPENAI_API_KEY` (sets `ai.api_key` when no key is already configured), `OPENAI_BASE_URL` (sets `ai.base_url`), `OPENROUTER_API_KEY` (sets `ai.api_key` and switches `ai.provider` to `openrouter`), and `AZURE_OPENAI_API_KEY` / `AZURE_OPENAI_ENDPOINT` / `AZURE_OPENAI_API_VERSION` / `AZURE_OPENAI_DEPLOYMENT_ID` (switch `ai.provider` to `azure-openai` and populate the Azure fields). Implemented in `src/config/service.rs`.

**Local-provider carve-out:** when the canonicalized `ai.provider` (after `normalize_ai_provider` has been applied to the resolved `SUBX_AI_PROVIDER` value or the configuration-file value) is `"local"`, `ProductionConfigService` SHALL skip the entire hosted-provider env-var application path. It SHALL NOT switch `ai.provider` away from `"local"`, SHALL NOT populate `ai.api_key`, `ai.base_url`, `ai.api_version`, or `ai.model` from any of `OPENAI_API_KEY`, `OPENAI_BASE_URL`, `OPENROUTER_API_KEY`, `AZURE_OPENAI_API_KEY`, `AZURE_OPENAI_ENDPOINT`, `AZURE_OPENAI_API_VERSION`, or `AZURE_OPENAI_DEPLOYMENT_ID`. This preserves the user's explicit privacy choice when they have selected a local provider.

#### Scenario: `OPENROUTER_API_KEY` switches provider
- **GIVEN** configuration file leaves `ai.provider` at its default and `OPENROUTER_API_KEY=sk-or-...`
- **WHEN** `ProductionConfigService` loads configuration
- **THEN** the resolved `Config.ai.provider` SHALL equal `"openrouter"` and `Config.ai.api_key` SHALL equal `Some("sk-or-...")`

#### Scenario: `AZURE_OPENAI_*` variables populate Azure fields
- **GIVEN** `AZURE_OPENAI_API_KEY=k`, `AZURE_OPENAI_ENDPOINT=https://x.openai.azure.com/`, `AZURE_OPENAI_API_VERSION=2024-02-15-preview`, and `AZURE_OPENAI_DEPLOYMENT_ID=gpt-4o`
- **WHEN** configuration is loaded
- **THEN** `Config.ai.provider` SHALL equal `"azure-openai"`, `Config.ai.base_url` SHALL equal the endpoint, `Config.ai.api_version` SHALL equal `Some("2024-02-15-preview")`, and `Config.ai.model` SHALL equal `"gpt-4o"`

#### Scenario: `OPENAI_API_KEY` is backward-compatible fallback
- **GIVEN** the configuration file has no `ai.api_key` and `OPENAI_API_KEY=sk-...` is set
- **WHEN** configuration is loaded
- **THEN** `Config.ai.api_key` SHALL equal `Some("sk-...")`

#### Scenario: `OPENAI_API_KEY` does not leak into local provider
- **GIVEN** the configuration file sets `ai.provider = "local"`, `ai.api_key = None`, and the environment has `OPENAI_API_KEY=sk-leak`
- **WHEN** `ProductionConfigService` loads configuration
- **THEN** `config.ai.provider` SHALL equal `"local"` and `config.ai.api_key` SHALL equal `None`

#### Scenario: `OPENROUTER_API_KEY` does not switch provider away from local
- **GIVEN** the configuration file sets `ai.provider = "local"` and the environment has `OPENROUTER_API_KEY=or-test`
- **WHEN** the configuration is loaded
- **THEN** `config.ai.provider` SHALL equal `"local"` and SHALL NOT be switched to `"openrouter"`

#### Scenario: `AZURE_OPENAI_*` variables ignored for local provider
- **GIVEN** the configuration file sets `ai.provider = "local"` and the environment has `AZURE_OPENAI_API_KEY`, `AZURE_OPENAI_ENDPOINT`, and `AZURE_OPENAI_DEPLOYMENT_ID` all set
- **WHEN** the configuration is loaded
- **THEN** `config.ai.provider` SHALL equal `"local"`, `config.ai.base_url` SHALL NOT be populated from `AZURE_OPENAI_ENDPOINT`, and `config.ai.model` SHALL NOT be populated from `AZURE_OPENAI_DEPLOYMENT_ID`

#### Scenario: `SUBX_AI_PROVIDER=ollama` triggers the local carve-out
- **GIVEN** the configuration file sets `ai.provider = "openai"`, the environment has `SUBX_AI_PROVIDER=ollama`, and the environment also has `OPENAI_API_KEY=sk-leak` and `OPENROUTER_API_KEY=or-leak`
- **WHEN** `ProductionConfigService` loads configuration
- **THEN** the resolved `config.ai.provider` SHALL equal `"local"` (after `normalize_ai_provider`) and neither `OPENAI_API_KEY` nor `OPENROUTER_API_KEY` SHALL populate `ai.api_key` or change the provider

### Requirement: AI Provider Identifier Canonicalization

The configuration system SHALL provide a single canonicalization function `normalize_ai_provider(value: &str) -> String` (located in `src/config/field_validator.rs`) that lowercases and trims its input and maps the alias `"ollama"` to the canonical identifier `"local"`. All other recognized providers (`openai`, `openrouter`, `azure-openai`, `local`) SHALL pass through unchanged. Unknown values SHALL be returned unchanged so downstream allow-list validation still rejects them with the existing error.

This function SHALL be the **only** place where the `ollama -> local` alias is resolved. Every component that reads or writes `ai.provider` SHALL invoke `normalize_ai_provider` before using the value, including:

1. `subx-cli`'s `config set ai.provider <value>` command surface (field validator) — the persisted on-disk value SHALL be the canonical form.
2. `subx-cli`'s `config get ai.provider` command surface (field validator) returns the canonical form.
3. `ProductionConfigService` env-var loading — `SUBX_AI_PROVIDER=ollama` SHALL be accepted and normalized to `"local"` before any precedence or scoping decision (including the hosted-provider env-var carve-out) is made.
4. `validate_ai_config` in `src/config/validator.rs` — validation arms key off the canonicalized value.
5. `ComponentFactory::create_ai_provider` in `src/core/factory.rs` — the dispatch match arm uses the canonicalized value, so the factory only ever sees `"local"` (never `"ollama"`).

#### Scenario: `ollama` is normalized when set via CLI
- **GIVEN** the user runs `subx config set ai.provider ollama`
- **WHEN** the field validator runs and the configuration is persisted
- **THEN** the persisted `ai.provider` value SHALL be `"local"` and a subsequent `subx config get ai.provider` SHALL return `"local"`

#### Scenario: `SUBX_AI_PROVIDER=ollama` is normalized
- **GIVEN** the configuration file has no `ai.provider` override and `SUBX_AI_PROVIDER=ollama` is set in the environment
- **WHEN** `ProductionConfigService` loads configuration
- **THEN** the resolved `config.ai.provider` SHALL equal `"local"` and the hosted-provider env-var carve-out SHALL apply as if the user had set `ai.provider = "local"` directly

#### Scenario: Canonical values pass through unchanged
- **GIVEN** any input in `{"openai", "openrouter", "azure-openai", "local"}`
- **WHEN** `normalize_ai_provider` is invoked
- **THEN** the returned string SHALL equal the input

### Requirement: Local Provider Validation Rules

When the canonicalized `ai.provider` (after `normalize_ai_provider`) equals `"local"`, `validate_ai_config` SHALL apply a dedicated validation arm in `src/config/validator.rs` that:
- Treats `ai.api_key` as optional: a missing or empty value SHALL be accepted; a non-empty value SHALL be validated through the same `validate_api_key` helper used by other providers (no provider-specific prefix is required).
- Requires `ai.base_url` to be a non-empty string and SHALL run it through `validate_url_format`.
- Validates `ai.model` (non-empty), `ai.temperature`, and `ai.max_tokens` using the same helpers as the hosted providers.
- Accepts BOTH `http://` and `https://` schemes for `ai.base_url`. The `local` provider is endpoint-agnostic and may target any reachable host (loopback, LAN, VPN, public). The HTTPS-required rule documented for hosted providers in the `ai-provider-integration` capability SHALL NOT apply to `local`.

`field_validator.rs` SHALL list both `local` and `ollama` in the allow-list for the `ai.provider` key (so that `subx config set` accepts either), and SHALL document that `ai.api_key` is optional and `ai.base_url` is required when the canonicalized provider is `local`. The persisted value after `subx config set` SHALL always be the canonical form produced by `normalize_ai_provider`.

#### Scenario: Local provider config without API key validates
- **GIVEN** `ai.provider = "local"`, `ai.base_url = "http://localhost:11434/v1"`, `ai.model = "llama3.1:8b-instruct"`, and `ai.api_key = None`
- **WHEN** `validate_ai_config` runs
- **THEN** it SHALL return `Ok(())`

#### Scenario: Local provider rejects missing base URL
- **GIVEN** `ai.provider = "local"`, `ai.base_url = ""`, and `ai.model = "llama3.1"`
- **WHEN** `validate_ai_config` runs
- **THEN** it SHALL return a configuration error whose message indicates that `ai.base_url` is required when `ai.provider` is `local`

#### Scenario: `ollama` alias is normalized
- **GIVEN** the user runs `subx config set ai.provider ollama`
- **WHEN** the field validator runs
- **THEN** the persisted `ai.provider` value SHALL be `"local"` (produced by `normalize_ai_provider`)

#### Scenario: Local provider accepts HTTP base URL
- **GIVEN** `ai.provider = "local"`, `ai.base_url = "http://192.168.1.50:11434/v1"`, and `ai.model = "llama3.1"`
- **WHEN** `validate_ai_config` runs
- **THEN** it SHALL return `Ok(())`

#### Scenario: Local provider accepts HTTPS base URL on a non-loopback host
- **GIVEN** `ai.provider = "local"`, `ai.base_url = "https://ollama.tailnet.ts.net/v1"`, and `ai.model = "qwen2.5:7b"`
- **WHEN** `validate_ai_config` runs
- **THEN** it SHALL return `Ok(())`

### Requirement: Local Provider Environment Variables

`ProductionConfigService` SHALL recognize the environment variables `LOCAL_LLM_BASE_URL` (mapping to `ai.base_url`) and `LOCAL_LLM_API_KEY` (mapping to `ai.api_key`), and SHALL apply them only when the canonicalized `ai.provider` (after `normalize_ai_provider` is applied to the resolved `SUBX_AI_PROVIDER` and config-file value) is `"local"`. These overrides SHALL apply with lower precedence than `SUBX_AI_BASE_URL` and `SUBX_AI_APIKEY` so that the unified `SUBX_*` namespace remains authoritative.

#### Scenario: `LOCAL_LLM_BASE_URL` honored when provider is local
- **GIVEN** the configuration file sets `ai.provider = "local"` and the environment has `LOCAL_LLM_BASE_URL=http://localhost:8080/v1` set
- **WHEN** `ProductionConfigService` loads configuration
- **THEN** `config.ai.base_url` SHALL equal `"http://localhost:8080/v1"`

#### Scenario: `LOCAL_LLM_*` ignored for non-local providers
- **GIVEN** the configuration file sets `ai.provider = "openai"` and the environment has `LOCAL_LLM_BASE_URL=http://localhost:11434/v1` and `LOCAL_LLM_API_KEY=secret` set
- **WHEN** the configuration is loaded
- **THEN** `config.ai.base_url` SHALL NOT be populated from `LOCAL_LLM_BASE_URL` and `config.ai.api_key` SHALL NOT be populated from `LOCAL_LLM_API_KEY`

#### Scenario: `SUBX_AI_BASE_URL` outranks `LOCAL_LLM_BASE_URL`
- **GIVEN** `ai.provider = "local"`, `LOCAL_LLM_BASE_URL=http://localhost:11434/v1`, and `SUBX_AI_BASE_URL=http://localhost:8080/v1`
- **WHEN** the configuration is loaded
- **THEN** `config.ai.base_url` SHALL equal `"http://localhost:8080/v1"`

### Requirement: Config file permissions enforcement

On Unix systems, the config file SHALL be created with restrictive permissions from the start — not fixed up after creation. The config directory SHALL be created with mode `0o700` before the config file is written. The config file SHALL be opened with `OpenOptionsExt::mode(0o600)` (or created via a temp-file with `0o600` permissions then atomically renamed) so that it is never world-readable at any point.

#### Scenario: save_config creates file with restrictive permissions
- **WHEN** `save_config()` writes a new config file on Unix
- **THEN** the file is created with permission mode `0o600` from the start (never temporarily world-readable)

#### Scenario: config directory has restrictive permissions
- **WHEN** the config directory is created
- **THEN** it is created with permission mode `0o700`

#### Scenario: existing config file permissions corrected on write
- **WHEN** `save_config()` writes to an existing config file on Unix
- **THEN** the file permissions are set to `0o600` after the write

