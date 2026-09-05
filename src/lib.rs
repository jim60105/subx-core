//! subx-core: Core Subtitle Processing Library for SubX
//!
//! `subx-core` is the reusable library half of the SubX project. It is
//! licensed under GPL-3.0-or-later and is designed to be consumed by the
//! `subx-cli` command-line tool, by graphical frontends such as the Tauri
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
//! # Current Status
//!
//! At this commit the crate is a skeleton: its public surface is a single
//! version constant, and the processing engines are being migrated into this
//! repository as part of the two-crate split. See the crate README for the
//! migration status.
//!
//! # Examples
//!
//! ```rust
//! use subx_core::VERSION;
//!
//! assert!(!VERSION.is_empty());
//! println!("subx-core version: {}", VERSION);
//! ```

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
