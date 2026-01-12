//! js-i18n-language-server
//!
//! i18n Language Server Protocol (LSP) implementation for JavaScript/TypeScript.

pub mod config;
pub mod db;
pub mod ide;
pub mod indexer;
pub mod input;
pub mod interned;
pub mod ir;
pub mod syntax;
pub mod types;

#[cfg(test)]
mod test_utils;

pub use ide::backend::Backend;
pub use ide::state::ServerState;
