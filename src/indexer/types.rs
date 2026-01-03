//! インデクサーで使用される型定義

use thiserror::Error;
use tower_lsp::lsp_types::{
    Range,
    Url,
};

/// 翻訳キーの使用箇所を表す構造体
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyUsageLocation {
    /// ファイルURI
    pub file_uri: Url,
    /// ソースコード上の位置
    pub range: Range,
}

/// インデクサーで発生するエラー
#[derive(Error, Debug)]
pub enum IndexerError {
    /// Error when failing to read a file
    #[error("Failed to read file: {0}")]
    InvalidPath(String),
    /// Other generic error
    #[error("An error occurred: {0}")]
    Error(String),
}
