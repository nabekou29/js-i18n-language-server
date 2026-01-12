//! Indexer type definitions.

use thiserror::Error;
use tower_lsp::lsp_types::{
    Range,
    Url,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyUsageLocation {
    pub file_uri: Url,
    pub range: Range,
}

#[derive(Error, Debug)]
pub enum IndexerError {
    /// Error when failing to read a file
    #[error("Failed to read file: {0}")]
    InvalidPath(String),
    /// Other generic error
    #[error("An error occurred: {0}")]
    Error(String),
}
