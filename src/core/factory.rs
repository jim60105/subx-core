//! Component factory for creating configured instances of core components.
//!
//! This module provides a centralized factory for creating instances of core
//! components with proper configuration injection, eliminating the need for
//! global configuration access within individual components.

use crate::services::ai::openai::OpenAIClient;
use crate::services::ai::openrouter::OpenRouterClient;
use crate::services::vad::{LocalVadDetector, VadAudioProcessor, VadSyncDetector};
use crate::{
    Result,
    config::{Config, ConfigService},
    core::{file_manager::FileManager, matcher::engine::MatchEngine},
    error::SubXError,
    services::ai::AIProvider,
};

/// Component factory for creating configured instances.
///
/// This factory provides a centralized way to create core components
/// with proper configuration injection, ensuring consistent component
/// initialization across the application.
///
/// # Examples
///
/// ```rust
/// use subx_core::core::ComponentFactory;
/// use subx_core::config::ProductionConfigService;
/// use std::sync::Arc;
///
/// # async fn example() -> subx_core::Result<()> {
/// let config_service = Arc::new(ProductionConfigService::new()?);
/// let factory = ComponentFactory::new(config_service.as_ref())?;
///
/// // Create components with proper configuration
/// let match_engine = factory.create_match_engine()?;
/// let file_manager = factory.create_file_manager();
/// let ai_provider = factory.create_ai_provider()?;
/// # Ok(())
/// # }
/// ```
pub struct ComponentFactory {
    config: Config,
    reporter: std::sync::Arc<dyn crate::core::report::Reporter>,
}

impl ComponentFactory {
    /// Create a new component factory with the given configuration service.
    ///
    /// # Arguments
    ///
    /// * `config_service` - Configuration service to load configuration from
    ///
    /// # Errors
    ///
    /// Returns an error if configuration loading fails.
    pub fn new(config_service: &dyn ConfigService) -> Result<Self> {
        let config = config_service.get_config()?;
        Ok(Self {
            config,
            reporter: crate::core::report::noop(),
        })
    }

    /// Attach a reporting sink, consuming and returning the factory.
    ///
    /// The reporter is propagated into every component this factory builds,
    /// so one call at a command boundary wires an entire command.
    ///
    /// # Arguments
    ///
    /// * `reporter` - Sink shared by every component created afterwards.
    pub fn with_reporter(
        mut self,
        reporter: std::sync::Arc<dyn crate::core::report::Reporter>,
    ) -> Self {
        self.reporter = reporter;
        self
    }

    /// Clone the factory's reporting sink for attachment to a component.
    ///
    /// Public so callers that construct a component outside the factory's
    /// control (e.g. a `MatchEngine` around an externally supplied AI
    /// client) can still wire the command's single reporter — the CLI's
    /// `match` command does exactly this while adopting
    /// [`ComponentFactory::match_config`].
    pub fn reporter(&self) -> std::sync::Arc<dyn crate::core::report::Reporter> {
        std::sync::Arc::clone(&self.reporter)
    }

    /// Return the [`MatchConfig`] this factory's loaded [`Config`] implies.
    ///
    /// The returned value carries the four configuration-derived fields read
    /// from the loaded config (`max_sample_length`, `ai_model`,
    /// `backup_enabled`, `max_subtitle_bytes`) plus the four defaults this
    /// factory pins for every caller: `confidence_threshold` 0.8 (the
    /// default value, kept configurable by overriding it here rather than in
    /// the config file), `enable_content_analysis: true`,
    /// `relocation_mode: FileRelocationMode::None` and
    /// `conflict_resolution: ConflictResolution::AutoRename`.
    ///
    /// `confidence_threshold`, `backup_enabled`, `relocation_mode` and
    /// `conflict_resolution` are caller-controlled: every `MatchConfig`
    /// field is public, so callers SHOULD mutate the returned value (two
    /// lines: `let mut c = factory.match_config(); c.relocation_mode = mode;`)
    /// instead of writing a `MatchConfig` struct literal — a literal must be
    /// edited whenever the struct gains a ninth field, this route never
    /// needs touching.
    ///
    /// # Returns
    ///
    /// A freshly built `MatchConfig`; identical in every field to the one
    /// [`ComponentFactory::create_match_engine`] constructs.
    ///
    /// # Examples
    ///
    /// ```
    /// use subx_core::config::TestConfigBuilder;
    /// use subx_core::core::ComponentFactory;
    ///
    /// # fn example() -> subx_core::Result<()> {
    /// let config_service = TestConfigBuilder::new().build_service();
    /// let factory = ComponentFactory::new(&config_service)?;
    /// let mut match_config = factory.match_config();
    /// match_config.confidence_threshold = 0.9;
    /// assert_eq!(match_config.confidence_threshold, 0.9);
    /// # Ok(())
    /// # }
    /// ```
    pub fn match_config(&self) -> crate::core::matcher::MatchConfig {
        crate::core::matcher::MatchConfig {
            confidence_threshold: 0.8, // Default value, can be configurable
            max_sample_length: self.config.ai.max_sample_length,
            enable_content_analysis: true,
            backup_enabled: self.config.general.backup_enabled,
            relocation_mode: crate::core::matcher::engine::FileRelocationMode::None,
            conflict_resolution: crate::core::matcher::engine::ConflictResolution::AutoRename,
            ai_model: self.config.ai.model.clone(),
            max_subtitle_bytes: self.config.general.max_subtitle_bytes,
        }
    }

    /// Create a match engine from a caller-supplied configuration.
    ///
    /// Builds the AI provider exactly as every other `create_*` method does
    /// (through [`ComponentFactory::create_ai_provider`]), uses `config`
    /// unmodified, and attaches the factory's reporter to the produced
    /// engine — the same propagation terms as
    /// [`ComponentFactory::create_match_engine`].
    ///
    /// # Arguments
    ///
    /// * `config` - The match configuration to use verbatim. Prefer deriving
    ///   it from [`ComponentFactory::match_config`] and overwriting the
    ///   caller-controlled fields.
    ///
    /// # Returns
    ///
    /// A `MatchEngine` wired with the factory's AI provider, the supplied
    /// config, and the factory's reporter.
    ///
    /// # Errors
    ///
    /// Returns an error if AI provider creation fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use subx_core::config::TestConfigBuilder;
    /// use subx_core::core::ComponentFactory;
    /// use subx_core::core::matcher::engine::FileRelocationMode;
    ///
    /// # fn example() -> subx_core::Result<()> {
    /// let config_service = TestConfigBuilder::new().build_service();
    /// let factory = ComponentFactory::new(&config_service)?;
    /// // Two-line relocation-mode override:
    /// let mut match_config = factory.match_config();
    /// match_config.relocation_mode = FileRelocationMode::Copy;
    /// let engine = factory.create_match_engine_with(match_config)?;
    /// # let _ = engine;
    /// # Ok(())
    /// # }
    /// ```
    pub fn create_match_engine_with(
        &self,
        config: crate::core::matcher::MatchConfig,
    ) -> Result<MatchEngine> {
        let ai_provider = self.create_ai_provider()?;
        Ok(MatchEngine::new(ai_provider, config).with_reporter(self.reporter()))
    }

    /// Create a match engine with AI configuration.
    ///
    /// Returns a properly configured MatchEngine instance using
    /// the AI configuration section. Equivalent to
    /// `create_match_engine_with(match_config())`; see
    /// [`ComponentFactory::match_config`] for the field-by-field contract.
    ///
    /// # Errors
    ///
    /// Returns an error if AI provider creation fails.
    pub fn create_match_engine(&self) -> Result<MatchEngine> {
        self.create_match_engine_with(self.match_config())
    }

    /// Create a file manager with general configuration.
    ///
    /// Returns a properly configured FileManager instance using
    /// the general configuration section.
    pub fn create_file_manager(&self) -> FileManager {
        // For now, FileManager doesn't take configuration in its constructor
        // This will be updated when FileManager is refactored to accept config
        FileManager::new().with_reporter(self.reporter())
    }

    /// Create an AI provider with AI configuration.
    ///
    /// Returns a properly configured AI provider instance based on
    /// the provider type specified in the AI configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the provider type is unsupported or
    /// provider creation fails.
    pub fn create_ai_provider(&self) -> Result<Box<dyn AIProvider>> {
        create_ai_provider_with_reporter(&self.config.ai, self.reporter())
    }

    /// Get a reference to the current configuration.
    ///
    /// Returns a reference to the configuration used by this factory.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Create a VAD sync detector with VAD configuration.
    ///
    /// Returns a properly configured VadSyncDetector instance using the VAD settings.
    ///
    /// # Errors
    ///
    /// Returns an error if VAD sync detector creation fails.
    pub fn create_vad_sync_detector(&self) -> Result<VadSyncDetector> {
        VadSyncDetector::new(self.config.sync.vad.clone())
    }

    /// Create a local VAD detector for audio processing.
    ///
    /// Returns a properly configured LocalVadDetector instance.
    ///
    /// # Errors
    ///
    /// Returns an error if local VAD detector initialization fails.
    pub fn create_vad_detector(&self) -> Result<LocalVadDetector> {
        LocalVadDetector::new(self.config.sync.vad.clone())
    }

    /// Create an audio processor for VAD operations.
    ///
    /// Returns a properly configured VadAudioProcessor instance.
    ///
    /// # Errors
    ///
    /// Returns an error if audio processor initialization fails.
    pub fn create_audio_processor(&self) -> Result<VadAudioProcessor> {
        VadAudioProcessor::new()
    }

    /// Create a translation engine using the configured AI provider and
    /// translation settings.
    ///
    /// # Errors
    ///
    /// Returns an error when AI provider creation fails or the configured
    /// translation batch size is invalid.
    pub fn create_translation_engine(&self) -> Result<crate::core::translation::TranslationEngine> {
        let ai_provider: std::sync::Arc<dyn AIProvider> =
            std::sync::Arc::from(self.create_ai_provider()?);
        let engine = crate::core::translation::TranslationEngine::new(
            ai_provider,
            self.config.translation.batch_size,
        )?;
        Ok(engine.with_reporter(self.reporter()))
    }
}

/// Create an AI provider from AI configuration.
///
/// This function creates the appropriate AI provider based on the
/// provider type specified in the configuration.
///
/// # Arguments
///
/// * `ai_config` - AI configuration containing provider settings
///
/// # Errors
///
/// Returns an error if the provider type is unsupported or creation fails.
/// Validate AI configuration parameters.
fn validate_ai_config(ai_config: &crate::config::AIConfig) -> Result<()> {
    let canonical = crate::config::field_validator::normalize_ai_provider(&ai_config.provider);
    let is_local = canonical == "local";

    // The `local` provider treats `api_key` as optional because most local
    // OpenAI-compatible runtimes (Ollama, LM Studio, llama.cpp `llama-server`)
    // accept unauthenticated requests. All hosted providers still require
    // an api_key.
    if !is_local && ai_config.api_key.as_deref().unwrap_or("").trim().is_empty() {
        return Err(SubXError::config(
            "AI API key is required. Set ai.api_key in configuration or use environment variable."
                .to_string(),
        ));
    }
    if ai_config.model.trim().is_empty() {
        return Err(SubXError::config(
            "AI model is required. Set ai.model in configuration.".to_string(),
        ));
    }
    if ai_config.temperature < 0.0 || ai_config.temperature > 2.0 {
        return Err(SubXError::config(
            "AI temperature must be between 0.0 and 2.0.".to_string(),
        ));
    }
    if ai_config.max_tokens == 0 {
        return Err(SubXError::config(
            "AI max_tokens must be greater than 0.".to_string(),
        ));
    }
    Ok(())
}

/// Create an AI provider from AI configuration.
///
/// This function creates the appropriate AI provider based on the
/// provider type specified in the configuration.
pub fn create_ai_provider_with_reporter(
    ai_config: &crate::config::AIConfig,
    reporter: std::sync::Arc<dyn crate::core::report::Reporter>,
) -> Result<Box<dyn AIProvider>> {
    let canonical = crate::config::field_validator::normalize_ai_provider(&ai_config.provider);
    match canonical.as_str() {
        "openai" => {
            validate_ai_config(ai_config)?;
            let client = OpenAIClient::from_config(ai_config)?.with_reporter(reporter);
            Ok(Box::new(client))
        }
        "openrouter" => {
            validate_ai_config(ai_config)?;
            let client = OpenRouterClient::from_config(ai_config)?.with_reporter(reporter);
            Ok(Box::new(client))
        }
        "azure-openai" => {
            validate_ai_config(ai_config)?;
            let client =
                crate::services::ai::azure_openai::AzureOpenAIClient::from_config(ai_config)?
                    .with_reporter(reporter);
            Ok(Box::new(client))
        }
        "local" => {
            validate_ai_config(ai_config)?;
            let client = crate::services::ai::local::LocalLLMClient::from_config(ai_config)?
                .with_reporter(reporter);
            Ok(Box::new(client))
        }
        other => Err(SubXError::config(format!(
            "Unsupported AI provider: {}. Supported providers: openai, openrouter, anthropic, azure-openai, local",
            other
        ))),
    }
}

/// Create an AI provider from AI configuration, reporting through a no-op
/// sink.
///
/// Convenience wrapper over [`create_ai_provider_with_reporter`] for callers
/// with no reporting sink to attach.
pub fn create_ai_provider(ai_config: &crate::config::AIConfig) -> Result<Box<dyn AIProvider>> {
    create_ai_provider_with_reporter(ai_config, crate::core::report::noop())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::builder::TestConfigBuilder;
    use crate::config::test_service::TestConfigService;

    #[test]
    fn test_component_factory_creation() {
        let config_service = TestConfigService::default();
        let factory = ComponentFactory::new(&config_service);
        assert!(factory.is_ok());
    }

    #[test]
    fn test_factory_creation() {
        let config_service = TestConfigService::default();
        let factory = ComponentFactory::new(&config_service);
        assert!(factory.is_ok());
    }

    #[test]
    fn test_create_file_manager() {
        let config_service = TestConfigService::default();
        let factory = ComponentFactory::new(&config_service).unwrap();

        let _file_manager = factory.create_file_manager();
        // Basic validation that file manager was created
        // FileManager doesn't expose config yet, so just verify creation succeeds
    }

    #[test]
    fn test_unsupported_ai_provider() {
        let mut config = crate::config::Config::default();
        config.ai.provider = "unsupported".to_string();

        let result: Result<Box<dyn AIProvider>> = create_ai_provider(&config.ai);
        assert!(result.is_err());

        match result {
            Err(e) => {
                let error_msg = e.to_string();
                assert!(error_msg.contains("Unsupported AI provider"));
                // The error message must enumerate all supported providers,
                // including the new `local` provider added by the
                // add-local-llm-provider change.
                assert!(error_msg.contains("openai"), "missing openai: {error_msg}");
                assert!(
                    error_msg.contains("openrouter"),
                    "missing openrouter: {error_msg}"
                );
                assert!(
                    error_msg.contains("azure-openai"),
                    "missing azure-openai: {error_msg}"
                );
                assert!(error_msg.contains("local"), "missing local: {error_msg}");
            }
            Ok(_) => panic!("Expected error for unsupported provider"),
        }
    }

    #[test]
    fn test_create_vad_sync_detector() {
        let config_service = TestConfigService::default();
        let factory = ComponentFactory::new(&config_service).unwrap();
        let result = factory.create_vad_sync_detector();
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_vad_detector() {
        let config_service = TestConfigService::default();
        let factory = ComponentFactory::new(&config_service).unwrap();
        let result = factory.create_vad_detector();
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_audio_processor() {
        let config_service = TestConfigService::default();
        let factory = ComponentFactory::new(&config_service).unwrap();
        let result = factory.create_audio_processor();
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_ai_provider_openai_success() {
        let config_service = TestConfigService::default();
        config_service.set_ai_settings_and_key("openai", "gpt-4.1-mini", "test-api-key");
        let factory = ComponentFactory::new(&config_service).unwrap();
        let result = factory.create_ai_provider();
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_ai_provider_missing_api_key() {
        let config_service = TestConfigService::default();
        config_service.set_ai_settings_and_key("openai", "gpt-4.1-mini", "");
        let factory = ComponentFactory::new(&config_service).unwrap();
        let result = factory.create_ai_provider();
        assert!(result.is_err());
        let error_msg = result.err().unwrap().to_string();
        assert!(error_msg.contains("API key is required"));
    }

    #[test]
    fn test_create_ai_provider_unsupported_provider() {
        let config_service = TestConfigService::default();
        config_service.set_ai_settings_and_key("unsupported-provider", "model", "key");
        let factory = ComponentFactory::new(&config_service).unwrap();
        let result = factory.create_ai_provider();
        assert!(result.is_err());
        let error_msg = result.err().unwrap().to_string();
        assert!(error_msg.contains("Unsupported AI provider"));
    }

    #[test]
    fn test_create_ai_provider_with_custom_base_url() {
        let config_service = TestConfigService::default();
        config_service.set_ai_settings_and_key("openai", "gpt-4.1-mini", "test-api-key");
        config_service.config_mut().ai.base_url = "https://custom-api.com/v1".to_string();
        let factory = ComponentFactory::new(&config_service).unwrap();
        let result = factory.create_ai_provider();
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_ai_provider_openrouter_success() {
        let config_service = TestConfigService::default();
        config_service.set_ai_settings_and_key(
            "openrouter",
            "deepseek/deepseek-r1-0528:free",
            "test-openrouter-key",
        );
        let factory = ComponentFactory::new(&config_service).unwrap();
        let result = factory.create_ai_provider();
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_ai_provider_azure_openai_success() {
        let mut config = crate::config::Config::default();
        config.ai.provider = "azure-openai".to_string();
        config.ai.api_key = Some("azure-key-123".to_string());
        config.ai.model = "dep123".to_string();
        config.ai.api_version = Some("2025-04-01-preview".to_string());
        config.ai.base_url = "https://example.openai.azure.com".to_string();
        let result = create_ai_provider(&config.ai);
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_ai_provider_local_success() {
        // Mirror `test_create_ai_provider_openai_success`: build a
        // `TestConfigService` via `TestConfigBuilder` configured for the
        // local provider, with NO api_key (intentionally absent) and an
        // OpenAI-compatible Ollama-style base URL. The factory must accept
        // this and return a working AI provider.
        use crate::config::builder::TestConfigBuilder;
        let config_service = TestConfigBuilder::new()
            .with_ai_provider("local")
            .with_ai_model("llama3.1")
            .with_ai_base_url("http://localhost:11434/v1")
            .build_service();
        let factory = ComponentFactory::new(&config_service).unwrap();
        let result = factory.create_ai_provider();
        assert!(
            result.is_ok(),
            "local provider must construct without api_key: {:?}",
            result.err()
        );

        // The `ollama` alias normalizes to `local` and must take the same
        // factory path.
        let alias_service = TestConfigBuilder::new()
            .with_ai_provider("ollama")
            .with_ai_model("llama3.1")
            .with_ai_base_url("http://localhost:11434/v1")
            .build_service();
        let alias_factory = ComponentFactory::new(&alias_service).unwrap();
        assert!(
            alias_factory.create_ai_provider().is_ok(),
            "`ollama` alias must reach the local arm"
        );
    }

    fn local_factory() -> ComponentFactory {
        let config_service = TestConfigBuilder::new()
            .with_ai_provider("local")
            .with_ai_model("llama3.1")
            .with_ai_base_url("http://localhost:11434/v1")
            .build_service();
        ComponentFactory::new(&config_service).unwrap()
    }

    #[test]
    fn match_config_matches_field_for_field_contract() {
        // The field-for-field contract: config-derived fields read the
        // loaded config; the other four are the factory's pinned defaults.
        let factory = local_factory();
        let config = factory.match_config();
        assert_eq!(config.confidence_threshold, 0.8);
        assert!(config.enable_content_analysis);
        assert_eq!(
            config.relocation_mode,
            crate::core::matcher::engine::FileRelocationMode::None
        );
        assert!(matches!(
            config.conflict_resolution,
            crate::core::matcher::engine::ConflictResolution::AutoRename
        ));
        // Config-derived fields must mirror the loaded config the factory
        // was built from (local provider, default general section).
        assert_eq!(config.ai_model, "llama3.1");
        assert_eq!(
            config.max_sample_length,
            crate::config::Config::default().ai.max_sample_length
        );
        assert_eq!(
            config.backup_enabled,
            crate::config::Config::default().general.backup_enabled
        );
        assert_eq!(
            config.max_subtitle_bytes,
            crate::config::Config::default().general.max_subtitle_bytes
        );
    }

    #[test]
    fn match_config_tracks_the_loaded_config() {
        // A backup-enabled config must surface in match_config(): the
        // method reads the loaded config, it does not re-derive defaults.
        let config_service = TestConfigBuilder::new()
            .with_ai_provider("local")
            .with_ai_base_url("http://localhost:11434/v1")
            .with_backup_enabled(true)
            .with_max_sample_length(4321)
            .build_service();
        let factory = ComponentFactory::new(&config_service).unwrap();
        let config = factory.match_config();
        assert!(config.backup_enabled);
        assert_eq!(config.max_sample_length, 4321);
    }

    #[test]
    fn create_match_engine_uses_match_config_values() {
        // Behavioural proof of the `new == create_match_engine_with(
        // match_config())` identity: create_match_engine builds its engine
        // through the same two methods, and the observable config half of
        // that engine is exercised end-to-end in
        // tests/factory_match_engine_tests.rs (relocation_mode reaching
        // MatchOperation needs a wired AI client). Here, the factory path
        // must at least construct successfully with the local provider.
        let factory = local_factory();
        assert!(factory.create_match_engine().is_ok());
        let engine = factory.create_match_engine_with(factory.match_config());
        assert!(engine.is_ok(), "{:?}", engine.err());
    }
}
