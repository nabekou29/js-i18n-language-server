//! Types for the analyzer module

use std::str::FromStr;

use serde::{
    Deserialize,
    Serialize,
};
use thiserror::Error;
use tower_lsp::lsp_types::Range;
use tree_sitter::Node;

/// Tree-sitter クエリで使用するキャプチャ名
///
/// i18n 関連の構文解析で使用するキャプチャ名を型安全に表現します。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CaptureName {
    /// 翻訳関数の呼び出し全体 (e.g., `t("key")`)
    CallTransFn,
    /// 翻訳キーの文字列リテラル部分
    TransKey,
    /// 翻訳キーの引数ノード（引用符を含む）
    TransKeyArg,
    /// 翻訳関数を取得する関数名 (e.g., `useTranslation`)
    GetTransFnName,
    /// 翻訳関数呼び出しの関数名 (e.g., `t`)
    CallTransFnName,
    /// 翻訳関数呼び出しの引数全体
    TransArgs,
    /// 翻訳関数を取得する呼び出し全体 (e.g., `useTranslation()`)
    GetTransFn,
    /// 名前空間
    Namespace,
    /// 配列形式の名前空間の個別要素 (e.g., `useTranslation(["ns1", "ns2"])` の各要素)
    NamespaceItem,
    /// 明示的な名前空間 (e.g., `t("key", { ns: "common" })` の `ns` 値)
    ExplicitNamespace,
    /// キープレフィックス
    KeyPrefix,
    /// 翻訳関数を取得する呼び出しの引数全体 (e.g., `getFixedT(...)`の引数)
    GetTransFnArgs,
}

impl CaptureName {
    /// Tree-sitter クエリで使用する文字列表現を取得
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

/// 文字列から `CaptureName` への変換エラー
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

/// Information about translation function calls
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransFnCall {
    /// Translation key (`key_prefix` が適用済み)
    pub key: String,
    /// Translation arguments (`コード上の引数、key_prefix` なし)
    pub arg_key: String,
    /// Translation key node
    pub arg_key_node: Range,
    /// Key prefix from useTranslation options
    pub key_prefix: Option<String>,
    /// Namespace from useTranslation (single namespace)
    pub namespace: Option<String>,
    /// Namespaces from useTranslation (array of namespaces, e.g., `useTranslation(["ns1", "ns2"])`)
    pub namespaces: Option<Vec<String>>,
}

/// Details about a `trans_fn` call
#[derive(Debug, Clone)]
pub struct CallTransFnDetail<'a> {
    /// 翻訳関数名（例: `t`, `i18n.t`）
    pub trans_fn_name: String,
    /// 翻訳キー（`key_prefix` 適用済み）
    pub key: String,
    /// 翻訳キーのノード（引用符を除いた文字列部分）
    pub key_node: Node<'a>,
    /// 翻訳キー引数のノード（引用符を含む）
    pub arg_key_node: Node<'a>,
    /// 明示的な名前空間（`t("key", { ns: "common" })` の `ns` 値）
    pub explicit_namespace: Option<String>,
}

/// Details about a `trans_fn`
#[derive(Debug, Clone, Default)]
pub struct GetTransFnDetail {
    /// 翻訳関数名（例: `t`, `i18n.t`）
    pub trans_fn_name: String,
    /// 名前空間（翻訳ファイルのグループ化に使用）
    pub namespace: Option<String>,
    /// 配列形式の名前空間（`useTranslation(["ns1", "ns2"])` の場合）
    pub namespaces: Option<Vec<String>>,
    /// キープレフィックス（翻訳キーの先頭に付加される文字列）
    pub key_prefix: Option<String>,
}

impl GetTransFnDetail {
    /// 新しい `GetTransFnDetail` を作成（デフォルト値で初期化）
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
