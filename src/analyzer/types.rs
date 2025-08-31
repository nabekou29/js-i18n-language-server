//! Types for the analyzer module

use serde::{
    Deserialize,
    Serialize,
};
use tree_sitter::{
    Node,
    Point,
};

/// TODO: doc
pub mod capture_names {
    /// TODO: doc
    pub const CALL_TRANS_FN: &str = "i18n.call_trans_fn";
    /// TODO: doc
    pub const TRANS_KEY: &str = "i18n.trans_key";
    /// TODO: doc
    pub const TRANS_KEY_ARG: &str = "i18n.trans_key_arg";
    /// TODO: doc
    pub const TRANS_FN_NAME: &str = "i18n.trans_fn_name";

    /// TODO: doc
    pub const GET_TRANS_FN: &str = "i18n.get_trans_fn";
    /// TODO: doc
    pub const NAMESPACE: &str = "i18n.namespace";
    /// TODO: doc
    pub const KEY_PREFIX: &str = "i18n.trans_key_prefix";
}

/// Information about translation function calls
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransFnCall {
    /// Translation key
    pub key: String,
    /// Translation arguments
    pub arg_key: String,
    /// Translation key node
    pub arg_key_node: Range,
}

/// Represents a position in the source code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Position {
    /// The line number (0-based).
    pub line: usize,
    /// The character index in the line (0-based).
    pub character: usize,
}

impl From<Point> for Position {
    fn from(point: Point) -> Self {
        Self { line: point.row, character: point.column }
    }
}

/// Represents a range in the source code, defined by a start and end position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Range {
    /// The start position of the range.
    pub start: Position,
    /// The end position of the range.
    pub end: Position,
}

impl From<Node<'_>> for Range {
    fn from(node: Node<'_>) -> Self {
        Self { start: node.start_position().into(), end: node.end_position().into() }
    }
}

/// Details about a `trans_fn` call
#[derive(Debug, Clone)]
pub struct CallTransFnDetail<'a> {
    /// The node representing the function call
    pub trans_fn_name: String,
    /// TODO: doc
    pub key: String,
    /// TODO: doc
    pub key_node: Node<'a>,
    /// TODO: doc
    pub arg_key_node: Node<'a>,
}

/// Details about a `trans_fn`
#[derive(Debug, Clone, Default)]
pub struct GetTransFnDetail {
    /// TODO: doc
    pub trans_fn_name: String,
    /// TODO: doc
    pub namespace: Option<String>,
    /// TODO: doc
    pub key_prefix: Option<String>,
}

impl GetTransFnDetail {
    /// 新しい `GetTransFnDetail` を作成
    #[must_use]
    pub fn new(trans_fn_name: impl Into<String>) -> Self {
        Self { trans_fn_name: trans_fn_name.into(), namespace: None, key_prefix: None }
    }

    /// namespace を設定（ビルダーパターン風）
    #[must_use]
    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = Some(namespace.into());
        self
    }

    /// `key_prefix` を設定（ビルダーパターン風）
    #[must_use]
    pub fn with_key_prefix(mut self, key_prefix: impl Into<String>) -> Self {
        self.key_prefix = Some(key_prefix.into());
        self
    }
}
