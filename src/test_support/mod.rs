//! Shared test fixtures for the SubX integration suites.
//!
//! This module exists solely to share fixtures across the `subx-core` /
//! `subx-cli` repository boundary: `subx-cli`'s integration tests link
//! `subx-core` as an external crate and cannot reach `subx-core`'s own
//! `#[cfg(test)]` helpers, while `subx-core` must not reference `subx-cli`
//! in either direction of the dependency. A `test-support` feature gates
//! the whole module, activated only through each crate's
//! `[dev-dependencies]` declaration, so it is absent from every shipping
//! build. Nothing here is part of the public API surface intended for
//! downstream consumers; the documentation lint is relaxed on the module
//! declaration in `lib.rs` (the `broken_intra_doc_links = "deny"` lint still
//! applies to the links it does carry).

pub mod file_managers;
pub mod mock_azure_openai;
pub mod mock_generators;
pub mod mock_openai;
pub mod responses;
pub mod workspace;
