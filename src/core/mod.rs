//! Core processing engine for SubX.
//!
//! This module contains core subsystems for file operations, subtitle format
//! handling, language detection, matching algorithms, parallel processing,
//! synchronization, and dependency injection management.
//!
//! Each subsystem is organized into its own submodule:
//! - `file_manager` for safe file operations with rollback support
//! - `formats` for parsing and converting subtitle formats
//! - `language` for language detection and handling
//! - `input` for input path collection, directory scanning and archive extraction
//! - `matcher` for AI-powered subtitle matching algorithms
//! - `parallel` for task scheduling and parallel execution
//! - `report` for the transport-agnostic reporting seam core reports through
//! - `sync` for audio-text synchronization engines
//! - `factory` for component creation with dependency injection
//! - `services` for service container and dependency management
//!
#![allow(dead_code)]

pub mod archive;
pub mod factory;
pub mod file_manager;
pub mod formats;
pub mod fs_util;
pub mod input;
pub mod language;
pub mod lock;
pub mod matcher;
pub mod parallel;
pub mod report;
pub mod sync;
pub mod translation;
pub mod uuidv7;

// Re-export commonly used types
pub use factory::ComponentFactory;

/// Compile-time thread-safety contract for the orchestration surface.
///
/// The list below is the contract, not a record of one change's edits:
/// every type named here is guaranteed `Send + Sync + 'static`, and a new
/// public engine, factory or manager type must be added by the change that
/// adds it (see the `async-runtime-safety` requirement *Library Engine
/// Types Are `Send` and `Sync`*).
///
/// A compile failure on one of these lines means a field of that type lost
/// its auto traits (an `Rc`, a `RefCell`, or a trait object whose trait
/// names no `Send`/`Sync` supertraits) — it does not mean the assertion is
/// wrong. Fix the field, or move the type out of the orchestration surface
/// deliberately.
#[cfg(test)]
mod thread_safety {
    const fn assert_send_sync<T: Send + Sync + 'static>() {}

    const _: () = assert_send_sync::<crate::core::formats::manager::FormatManager>();
    const _: () = assert_send_sync::<crate::core::formats::converter::FormatConverter>();
    const _: () = assert_send_sync::<crate::core::translation::TranslationEngine>();
    const _: () = assert_send_sync::<crate::core::matcher::MatchEngine>();
    const _: () = assert_send_sync::<crate::core::sync::SyncEngine>();
    const _: () = assert_send_sync::<crate::core::ComponentFactory>();
    const _: () = assert_send_sync::<crate::core::file_manager::FileManager>();
    const _: () = assert_send_sync::<Box<dyn crate::core::formats::SubtitleFormat>>();
}
