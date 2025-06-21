//! i18n分析で使用する型定義
//!
//! このモジュールは、翻訳キーや関数呼び出しの情報を表現するための
//! 基本的な型を定義します。

use serde::{
    Deserialize,
    Serialize,
};

/// 翻訳キーを表す構造体
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TranslationKey {
    /// 翻訳キーの値（例: "common.hello", "errors.notFound"）
    pub key: String,
    /// 名前空間（オプション）
    pub namespace: Option<String>,
    /// keyPrefixがある場合の値
    pub key_prefix: Option<String>,
}

impl TranslationKey {
    /// `新しいTranslationKeyインスタンスを作成します`
    pub fn new(key: impl Into<String>) -> Self {
        Self { key: key.into(), namespace: None, key_prefix: None }
    }

    /// 名前空間を設定します
    #[must_use]
    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = Some(namespace.into());
        self
    }

    /// keyPrefixを設定します
    #[must_use]
    pub fn with_key_prefix(mut self, key_prefix: impl Into<String>) -> Self {
        self.key_prefix = Some(key_prefix.into());
        self
    }

    /// 完全な翻訳キーを取得します（namespace:prefix.key形式）
    #[must_use]
    pub fn full_key(&self) -> String {
        let mut parts = Vec::new();

        if let Some(ns) = &self.namespace {
            parts.push(ns.clone());
        }

        let key_part = self
            .key_prefix
            .as_ref()
            .map_or_else(|| self.key.clone(), |prefix| format!("{}.{}", prefix, self.key));

        if parts.is_empty() { key_part } else { format!("{}:{}", parts.join(":"), key_part) }
    }
}

/// 翻訳関数の呼び出し情報
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TranslationCall {
    /// 翻訳キー
    pub key: TranslationKey,
    /// ファイル内での位置（開始行）
    pub start_line: usize,
    /// ファイル内での位置（開始列）
    pub start_column: usize,
    /// ファイル内での位置（終了行）
    pub end_line: usize,
    /// ファイル内での位置（終了列）
    pub end_column: usize,
    /// 呼び出された関数名（例: "t", "i18n.t", "useTranslation"）
    pub function_name: String,
    /// 関数呼び出しのコンテキスト（例: "useTranslation"のオプション）
    pub context: Option<serde_json::Value>,
}

impl TranslationCall {
    /// `新しいTranslationCallインスタンスを作成します`
    pub fn new(
        key: TranslationKey,
        start_line: usize,
        start_column: usize,
        end_line: usize,
        end_column: usize,
        function_name: impl Into<String>,
    ) -> Self {
        Self {
            key,
            start_line,
            start_column,
            end_line,
            end_column,
            function_name: function_name.into(),
            context: None,
        }
    }

    /// コンテキスト情報を設定します
    #[must_use]
    pub fn with_context(mut self, context: serde_json::Value) -> Self {
        self.context = Some(context);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_translation_key_full_key() {
        // キーのみ
        let key = TranslationKey::new("hello");
        assert_eq!(key.full_key(), "hello");

        // 名前空間付き
        let key = TranslationKey::new("hello").with_namespace("common");
        assert_eq!(key.full_key(), "common:hello");

        // keyPrefix付き
        let key = TranslationKey::new("hello").with_key_prefix("greetings");
        assert_eq!(key.full_key(), "greetings.hello");

        // 両方
        let key =
            TranslationKey::new("hello").with_namespace("common").with_key_prefix("greetings");
        assert_eq!(key.full_key(), "common:greetings.hello");
    }
}
