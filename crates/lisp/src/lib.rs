//! Compatibility layer for the Lisp interpreter crate.
//!
//! The actual interpreter implementation lives in `lisp_core`.
//! This crate re-exports the existing public API so downstream users can
//! continue importing `lisp::...` while the repository is organized as a workspace.

mod errors;

pub use errors::LispError;
pub use lisp_core::*;
