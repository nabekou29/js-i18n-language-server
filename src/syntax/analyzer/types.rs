//! Types for the analyzer module

use std::str::FromStr;

use serde::{
    Deserialize,
    Serialize,
};
use thiserror::Error;
use tower_lsp::lsp_types::Range;
use tree_sitter::Node;

/// Capture names used in tree-sitter queries for i18n syntax analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CaptureName {
    CallTransFn,
    TransKey,
    TransKeyArg,
    GetTransFnName,
    CallTransFnName,
    TransArgs,
    GetTransFn,
    Namespace,
    NamespaceItem,
    ExplicitNamespace,
    KeyPrefix,
    GetTransFnArgs,
}

impl CaptureName {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CallTransFn => "i18n.call_trans_fn",
            Self::TransKey => "i18n.trans_key",
            Self::TransKeyArg => "i18n.trans_key_arg",
            Self::GetTransFnName => "i18n.get_trans_fn_name",
            Self::CallTransFnName => "i18n.call_trans_fn_name",
            Self::TransArgs => "i18n.trans_args",
            Self::GetTransFn => "i18n.get_trans_fn",
            Self::Namespace => "i18n.namespace",
            Self::NamespaceItem => "i18n.namespace_item",
            Self::ExplicitNamespace => "i18n.explicit_namespace",
            Self::KeyPrefix => "i18n.trans_key_prefix",
            Self::GetTransFnArgs => "i18n.get_trans_fn_args",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseCaptureNameError;

impl FromStr for CaptureName {
    type Err = ParseCaptureNameError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "i18n.call_trans_fn" => Ok(Self::CallTransFn),
            "i18n.trans_key" => Ok(Self::TransKey),
            "i18n.trans_key_arg" => Ok(Self::TransKeyArg),
            "i18n.get_trans_fn_name" => Ok(Self::GetTransFnName),
            "i18n.call_trans_fn_name" => Ok(Self::CallTransFnName),
            "i18n.trans_args" => Ok(Self::TransArgs),
            "i18n.get_trans_fn" => Ok(Self::GetTransFn),
            "i18n.namespace" => Ok(Self::Namespace),
            "i18n.namespace_item" => Ok(Self::NamespaceItem),
            "i18n.explicit_namespace" => Ok(Self::ExplicitNamespace),
            "i18n.trans_key_prefix" => Ok(Self::KeyPrefix),
            "i18n.get_trans_fn_args" => Ok(Self::GetTransFnArgs),
            _ => Err(ParseCaptureNameError),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransFnCall {
    /// Translation key with `key_prefix` applied
    pub key: String,
    /// Original argument key without `key_prefix`
    pub arg_key: String,
    pub arg_key_node: Range,
    pub key_prefix: Option<String>,
    pub namespace: Option<String>,
    pub namespaces: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct CallTransFnDetail<'a> {
    pub trans_fn_name: String,
    pub key: String,
    pub key_node: Node<'a>,
    pub arg_key_node: Node<'a>,
    pub explicit_namespace: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct GetTransFnDetail {
    pub trans_fn_name: String,
    pub namespace: Option<String>,
    pub namespaces: Option<Vec<String>>,
    pub key_prefix: Option<String>,
}

impl GetTransFnDetail {
    #[must_use]
    pub fn new(trans_fn_name: impl Into<String>) -> Self {
        Self {
            trans_fn_name: trans_fn_name.into(),
            namespace: None,
            namespaces: None,
            key_prefix: None,
        }
    }
}

/// Defines errors that may occur during the analysis process
#[derive(Error, Debug)]
pub enum AnalyzerError {
    /// Error when failing to set the language for the parser
    #[error("Failed to set language for parser: {0}")]
    LanguageSetup(#[from] tree_sitter::LanguageError),
    /// Error when failing to read a file
    #[error("Failed to read file: {0}")]
    InvalidPath(String),
    /// Error when failing to parse source code
    #[error("Failed to parse source code")]
    ParseFailed,
    /// Error when failing to execute a query
    #[error("Query execution failed: {0}")]
    QueryExecution(String),
}
