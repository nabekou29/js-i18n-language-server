//! TODO

use thiserror::Error;
use tower_lsp::lsp_types::{
    Range,
    Url,
};

/// TODO
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyUsageLocation {
    /// ファイルURI
    pub file_uri: Url,
    /// ソースコード上の位置
    pub range: Range,
}

/// TODO
#[derive(Error, Debug)]
pub enum IndexerError {
    /// Error when failing to read a file
    #[error("Failed to read file: {0}")]
    InvalidPath(String),
    /// Other generic error
    #[error("An error occurred: {0}")]
    Error(String),
}
