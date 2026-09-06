# Secrets Protection

## Purpose

Define the core secrets-protection primitives: the `mask_sensitive_value` masking helper in `src/config/masking.rs` with its case-insensitive sensitive-key matching and `****`-suffix rendering, API-key redaction in `Debug` output, the restrictive config-file permission check, and the insecure-HTTP endpoint warning. The `subx-cli` repository carries the same-named `secrets-protection` capability holding the display-site obligation that `config set`, `config list`, and `config get` call the masking helper and never print raw values.
## Requirements

### Requirement: Sensitive Value Masking Helper

The masking primitive SHALL live in this repository as `mask_sensitive_value` in `src/config/masking.rs`. It SHALL treat a config key as sensitive when its name matches `api_key`, `token`, or `secret` case-insensitively, and SHALL render a sensitive value as `****<last 4 chars>` (or `****` when the value is ≤4 chars). The obligation that the `config set`, `config list`, and `config get` display sites call the helper and never print the raw value alongside the masked one is specified by the `secrets-protection` capability's *Mask sensitive config values in CLI output* requirement in `subx-cli`.

#### Scenario: sensitive key match is case-insensitive
- **WHEN** the helper is asked to mask the value of a key named `AI.API_KEY`
- **THEN** the returned string SHALL be masked, not the plaintext value

#### Scenario: short secret is fully masked
- **WHEN** the api_key value is 3 characters
- **THEN** the helper SHALL return `****`

### Requirement: Redact API keys in Debug output

All structs holding API keys (OpenAIClient, OpenRouterClient, AzureOpenAIClient, AIConfig) SHALL NOT expose the `api_key` field via `Debug` formatting. The Debug output MUST show `[REDACTED]` for the api_key field.

#### Scenario: Debug format redacts key

- **WHEN** `format!("{:?}", client)` is called on an AI client struct
- **THEN** the output contains `[REDACTED]` instead of the actual API key

### Requirement: Restrict config file permissions

On Unix systems, the system SHALL set file permissions to `0o600` on the config file after every write operation. The config directory SHALL be set to `0o700`.

#### Scenario: new config file has restricted permissions

- **WHEN** `config set` writes a new config file
- **THEN** the file permission mode is `0o600` on Unix

#### Scenario: existing config file permissions corrected

- **WHEN** `save_config` writes to an existing config file
- **THEN** the file permission mode is set to `0o600`

### Requirement: Warn on insecure HTTP endpoint

When an AI client is constructed with a `http://` (non-HTTPS) base URL and a non-empty API key, the system SHALL emit a warning log message indicating that the API key will be transmitted in plaintext.

#### Scenario: HTTP endpoint with key triggers warning

- **WHEN** the base URL is `http://example.com` and api_key is set
- **THEN** a warning is logged about plaintext transmission

#### Scenario: HTTPS endpoint produces no warning

- **WHEN** the base URL is `https://api.openai.com` and api_key is set
- **THEN** no insecure-transport warning is logged

#### Scenario: HTTP localhost produces no warning

- **WHEN** the base URL is `http://127.0.0.1:8080` and api_key is set
- **THEN** no insecure-transport warning is logged (localhost is acceptable for development)

