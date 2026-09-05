//! SubX Core: Reusable Subtitle Processing Library
//!
//! `subx-core` is the library half of the SubX project: the configuration
//! system, the processing engines, the shared error type and the external
//! service integrations, packaged so they can be consumed without the
//! command-line interface. It features AI-powered matching, format
//! conversion, audio synchronization, and advanced encoding detection.
//!
//! It is licensed under GPL-3.0-or-later and is designed to be consumed by
//! the `subx-cli` command-line tool, by graphical frontends such as the Tauri
//! GUI at <https://github.com/jim60105/subx>, and by any Rust program that
//! needs subtitle processing capabilities.
//!
//! # Repository Relationship
//!
//! In the `subx-cli` repository this crate is mounted as a git submodule at
//! `subx-cli/subx-core/` and built as a Cargo workspace member. It is also a
//! fully standalone crate: a plain `git clone` of this repository followed by
//! `cargo build` works without any other repository present, which is the
//! contract that keeps crates.io consumers and `docs.rs` builds working.
//!
//! This crate never references `subx-cli` — not in code, not in an intra-doc
//! link, not in a doctest. The dependency only points the other way, so any
//! such reference would be unresolvable rather than merely stale, and it is
//! rejected by a boundary guard test in the `subx-cli` repository.
//!
//! # Modules
//!
//! - [`config`] - Configuration management and validation
//! - [`core`] - Core processing engines (formats, matching, sync)
//! - [`error`] - Comprehensive error handling system
//! - [`services`] - External service integrations (AI, audio processing)
//!
//! # Module Path Stability (a deliberate decision, not an oversight)
//!
//! Engine paths keep the redundant `core::` segment: the canonical path of
//! the match engine is [`core::matcher::MatchEngine`], written
//! `subx_core::core::matcher::MatchEngine`. This stutters, and a flatter
//! `subx_core::matcher::…` surface would read better in isolation. It is
//! rejected deliberately: these items were reachable at exactly these
//! relative paths under the `subx-cli` crate's name for the library's entire
//! history, and downstream consumers — chiefly the Tauri GUI — reach roughly
//! thirty of them by path. Preserving the tree makes their migration a pure
//! crate-name substitution. Flattening or otherwise reshaping these paths is
//! therefore a breaking change that requires a major version of this crate,
//! and no module is re-exported at the crate root alongside its canonical
//! path, because a second public path would make intra-doc links ambiguous
//! under `broken_intra_doc_links = "deny"`.
//!
//! # Features
//!
//! - `archive-rar` - enable RAR archive extraction (optional `unrar`
//!   dependency; without it the archive layer exposes a disabled-feature
//!   stub)
//! - `slow-tests` - compile long-running format round-trip tests
//!
//! # Examples
//!
//! Configuration access through the injected service:
//!
//! ```rust,no_run
//! use subx_core::config::{ConfigService, TestConfigService};
//!
//! // Create a configuration service
//! let config_service = TestConfigService::with_defaults();
//! let config = config_service.config();
//!
//! // Use the configuration for processing...
//! ```
//!
//! All operations return a [`Result<T>`] type that wraps [`error::SubXError`]:
//!
//! ```rust
//! use subx_core::{error::SubXError, Result};
//!
//! fn example_operation() -> Result<String> {
//!     // This could fail with various error types
//!     Err(SubXError::config("Missing configuration"))
//! }
//! ```
//!
//! Dependency-injected configuration with AI settings:
//!
//! ```rust,no_run
//! use subx_core::config::{Config, TestConfigService};
//!
//! // Create configuration service with AI settings
//! let config_service = TestConfigService::with_ai_settings("openai", "gpt-4.1");
//! let config = config_service.config();
//!
//! // Access configuration values
//! println!("AI Provider: {}", config.ai.provider);
//! println!("AI Model: {}", config.ai.model);
//! ```

#![allow(
    clippy::new_without_default,
    clippy::manual_clamp,
    clippy::useless_vec,
    clippy::items_after_test_module,
    clippy::needless_borrow,
    clippy::uninlined_format_args,
    clippy::collapsible_if
)]
#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]

pub mod config;
pub mod core;
pub mod error;
pub mod services;

/// Shared test fixtures for the SubX integration suites.
///
/// Compiled only with the `test-support` feature, which no shipping build
/// activates — `subx-cli` turns it on through a `[dev-dependencies]`
/// declaration of this crate, and this crate's own integration tests reach
/// it through a path-only self dev-dependency, so neither mechanism can
/// leak into a release artifact. The missing-documentation lint is relaxed
/// on this declaration because the module's items are test scaffolding
/// documented by their surrounding prose rather than rustdoc; the
/// `broken_intra_doc_links = "deny"` lint still applies inside it.
#[cfg(feature = "test-support")]
#[allow(missing_docs)]
pub mod test_support;

pub use config::Config;
// Re-export the configuration service system at the crate root.
pub use config::{
    ConfigService, EnvironmentProvider, ProductionConfigService, SystemEnvironmentProvider,
    TestConfigBuilder, TestConfigService, TestEnvironmentProvider,
};

/// Convenient type alias for `Result<T, SubXError>`.
///
/// This type alias simplifies error handling throughout the SubX library
/// by providing a default error type for all fallible operations.
pub type Result<T> = error::SubXResult<T>;

/// Library version string.
///
/// This constant provides the current version of the `subx-core` library,
/// automatically populated from `Cargo.toml` at compile time. It reports
/// this crate's own version and is independent of the version of any
/// consumer such as `subx-cli`.
///
/// # Examples
///
/// ```rust
/// use subx_core::VERSION;
///
/// // The version is always present and follows semver.
/// assert!(!VERSION.is_empty());
/// assert!(VERSION.split('.').next().is_some());
/// ```
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::VERSION;

    #[test]
    fn version_is_not_empty() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn version_matches_cargo_pkg_version() {
        assert_eq!(VERSION, env!("CARGO_PKG_VERSION"));
    }
}
