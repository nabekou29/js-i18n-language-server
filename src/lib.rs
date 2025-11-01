//! js-i18n-language-server
//!
//! JavaScript/TypeScript プロジェクト向けの i18n Language Server Protocol (LSP) 実装

pub mod config;
pub mod db;
pub mod ide;
pub mod indexer;
pub mod input;
pub mod interned;
pub mod ir;
pub mod syntax;
pub mod types;

// Backend を再エクスポート
pub use ide::backend::Backend;
