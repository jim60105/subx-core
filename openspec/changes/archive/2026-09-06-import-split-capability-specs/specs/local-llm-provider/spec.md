## MODIFIED Requirements

### Requirement: Local LLM Provider Identifier

The system SHALL recognize the AI provider identifier `local` (with `ollama` accepted as an alias normalized to `local` by the single canonicalization function `normalize_ai_provider` defined in this repository's `configuration-management` capability) as a first-class value of `config.ai.provider`. `ComponentFactory::create_ai_provider` SHALL receive only the canonical form (`local`) — the alias `ollama` SHALL never reach the factory dispatch — and SHALL dispatch this identifier to a dedicated `LocalLLMClient` implementing the `AIProvider` trait under `src/services/ai/local.rs`.

#### Scenario: `local` selects the local LLM client
- **GIVEN** `ai.provider = "local"`, `ai.base_url = "http://localhost:11434/v1"`, and `ai.model = "llama3.1:8b-instruct"`
- **WHEN** `ComponentFactory::create_ai_provider` is invoked
- **THEN** it SHALL return a boxed `LocalLLMClient` configured with the supplied `base_url`, `model`, `temperature`, `max_tokens`, retry settings, and request timeout

#### Scenario: `ollama` alias is normalized to `local`
- **GIVEN** the user runs `subx config set ai.provider ollama`
- **WHEN** the field validator runs (invoking `normalize_ai_provider`) and configuration is persisted
- **THEN** the persisted `ai.provider` value SHALL be `"local"` and a subsequent `ComponentFactory::create_ai_provider` call SHALL return the `LocalLLMClient`

### Requirement: Local Provider Environment Variable Overrides

`ProductionConfigService` SHALL recognize `LOCAL_LLM_BASE_URL` (mapping to `ai.base_url`) and `LOCAL_LLM_API_KEY` (mapping to `ai.api_key`) but SHALL apply them only when the canonicalized `ai.provider` (after `normalize_ai_provider`) is `"local"`. The standard `SUBX_AI_*` overrides (`SUBX_AI_PROVIDER`, `SUBX_AI_MODEL`, `SUBX_AI_BASE_URL`, `SUBX_AI_APIKEY`) SHALL continue to take precedence over file-backed configuration as defined by this repository's `configuration-management` capability. `SUBX_AI_PROVIDER=ollama` SHALL be supported and normalized to `"local"` so that a user who sets only the env var still receives the local-provider behavior including the hosted-provider env-var carve-out.

#### Scenario: `LOCAL_LLM_BASE_URL` honored only for local provider
- **GIVEN** `ai.provider = "local"` and `LOCAL_LLM_BASE_URL=http://localhost:11434/v1`
- **WHEN** the configuration is loaded
- **THEN** `config.ai.base_url` SHALL equal `"http://localhost:11434/v1"`

#### Scenario: `LOCAL_LLM_*` ignored when provider is not local
- **GIVEN** `ai.provider = "openai"`, `LOCAL_LLM_BASE_URL=http://localhost:11434/v1`, and `LOCAL_LLM_API_KEY=secret`
- **WHEN** the configuration is loaded
- **THEN** `config.ai.base_url` SHALL NOT be set from `LOCAL_LLM_BASE_URL` and `config.ai.api_key` SHALL NOT be set from `LOCAL_LLM_API_KEY`

### Requirement: Actionable Local-Endpoint Error Mapping

`LocalLLMClient` SHALL classify common local-endpoint failures into distinct `SubXError::AiService` messages so users can distinguish "server not running" from "model not loaded" from "incompatible response". Sensitive request bodies and headers SHALL be sanitized via the existing `error_sanitizer` before being included in error messages.

When a `base_url` is included in an error message, it SHALL be a sanitized form that contains only the `scheme`, `host`, `port`, and `path` components. The sanitized form SHALL NOT include any query string, fragment, userinfo (`user:password@`), or `Authorization` / `api_key` value, in conformance with this repository's `error-handling` capability's "Error types and messages SHALL NOT include … full request/response URLs that contain query parameters" requirement. The sanitization SHALL be performed by a dedicated helper (e.g. `sanitize_base_url(&str) -> String`) that is unit-tested independently and reused across all local-endpoint error variants.

#### Scenario: Connection refused yields a server-not-running error
- **GIVEN** `ai.base_url` points at a port where no server is listening
- **WHEN** `LocalLLMClient::analyze_content` is invoked
- **THEN** the returned error SHALL be `SubXError::AiService` whose message contains `local LLM endpoint unreachable` and the **sanitized** base URL (scheme + host + port + path only)

#### Scenario: HTTP 404 for an unknown model yields a model-not-found error
- **GIVEN** the local endpoint responds with HTTP 404 and a body indicating an unknown model
- **WHEN** `LocalLLMClient::analyze_content` is invoked
- **THEN** the returned error SHALL be `SubXError::AiService` whose message contains `local LLM model not found` and the configured `ai.model` value

#### Scenario: Non-JSON response yields a parse-shape error
- **GIVEN** the local endpoint responds with HTTP 200 and a body that is not OpenAI-compatible JSON
- **WHEN** `LocalLLMClient::analyze_content` is invoked
- **THEN** the returned error SHALL be `SubXError::AiService` whose message indicates that the response was not OpenAI-compatible JSON

#### Scenario: Base URL with credentials and query string is sanitized in error
- **GIVEN** `ai.base_url = "http://user:secret@localhost:11434/v1?token=abc"` and the endpoint is unreachable
- **WHEN** `LocalLLMClient::analyze_content` is invoked
- **THEN** the returned `SubXError::AiService` message SHALL contain `http://localhost:11434/v1` and SHALL NOT contain `user:secret`, `secret`, `token=`, or `?abc`
