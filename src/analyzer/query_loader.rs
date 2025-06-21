//! tree-sitterクエリの読み込みと管理
//!
//! .scmファイルからクエリを読み込み、言語別に管理します。

use std::collections::HashMap;

use tree_sitter::{
    Language,
    Query,
};

/// クエリセット（複数のクエリをまとめたもの）
pub struct QuerySet {
    /// i18nextクエリ
    pub i18next: Option<Query>,
    /// react-i18nextクエリ
    pub react_i18next: Option<Query>,
    /// next-intlクエリ
    pub next_intl: Option<Query>,
}

impl std::fmt::Debug for QuerySet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QuerySet")
            .field("i18next", &self.i18next.as_ref().map(|_| "Query"))
            .field("react_i18next", &self.react_i18next.as_ref().map(|_| "Query"))
            .field("next_intl", &self.next_intl.as_ref().map(|_| "Query"))
            .finish()
    }
}

impl Default for QuerySet {
    fn default() -> Self {
        Self::new()
    }
}

impl QuerySet {
    /// `新しい空のQuerySetを作成`
    #[must_use]
    pub const fn new() -> Self {
        Self { i18next: None, react_i18next: None, next_intl: None }
    }

    /// すべての有効なクエリを取得
    #[must_use]
    pub fn all_queries(&self) -> Vec<&Query> {
        let mut queries = Vec::new();
        if let Some(q) = &self.i18next {
            queries.push(q);
        }
        if let Some(q) = &self.react_i18next {
            queries.push(q);
        }
        if let Some(q) = &self.next_intl {
            queries.push(q);
        }
        queries
    }
}

/// クエリローダー
#[derive(Debug)]
pub struct QueryLoader {
    /// 言語別のクエリセット
    query_sets: HashMap<String, QuerySet>,
}

impl QueryLoader {
    /// `新しいQueryLoaderインスタンスを作成`
    #[must_use]
    pub fn new() -> Self {
        Self { query_sets: HashMap::new() }
    }

    /// `JavaScript用のクエリを読み込み`
    ///
    /// # Errors
    ///
    /// クエリファイルの読み込みやパースに失敗した場合
    pub fn load_javascript_queries(
        &mut self,
        language: &Language,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut query_set = QuerySet::new();

        // プロジェクトルートからのパス
        let base_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        // i18nextクエリ
        let i18next_path = base_path.join("queries/javascript/i18next.scm");
        if let Ok(query_text) = std::fs::read_to_string(&i18next_path) {
            query_set.i18next = Some(Query::new(language, &query_text)?);
        }

        // react-i18nextクエリ
        let react_i18next_path = base_path.join("queries/javascript/react-i18next.scm");
        if let Ok(query_text) = std::fs::read_to_string(&react_i18next_path) {
            query_set.react_i18next = Some(Query::new(language, &query_text)?);
        }

        self.query_sets.insert("javascript".to_string(), query_set);
        Ok(())
    }

    /// `TypeScript用のクエリを読み込み`
    ///
    /// # Errors
    ///
    /// クエリファイルの読み込みやパースに失敗した場合
    pub fn load_typescript_queries(
        &mut self,
        language: &Language,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut query_set = QuerySet::new();

        // プロジェクトルートからのパス
        let base_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        // i18nextクエリ
        let i18next_path = base_path.join("queries/typescript/i18next.scm");
        if let Ok(query_text) = std::fs::read_to_string(&i18next_path) {
            query_set.i18next = Some(Query::new(language, &query_text)?);
        }

        self.query_sets.insert("typescript".to_string(), query_set);
        Ok(())
    }

    /// TSX用のクエリを読み込み
    ///
    /// # Errors
    ///
    /// クエリファイルの読み込みやパースに失敗した場合
    pub fn load_tsx_queries(
        &mut self,
        language: &Language,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut query_set = QuerySet::new();

        // プロジェクトルートからのパス
        let base_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        // i18nextクエリ
        let i18next_path = base_path.join("queries/tsx/i18next.scm");
        if let Ok(query_text) = std::fs::read_to_string(&i18next_path) {
            query_set.i18next = Some(Query::new(language, &query_text)?);
        }

        // react-i18nextクエリ（TSXはJSXと同じクエリを使用）
        let react_i18next_path = base_path.join("queries/tsx/react-i18next.scm");
        if let Ok(query_text) = std::fs::read_to_string(&react_i18next_path) {
            query_set.react_i18next = Some(Query::new(language, &query_text)?);
        }

        self.query_sets.insert("tsx".to_string(), query_set);
        Ok(())
    }

    /// 指定された言語のクエリセットを取得
    #[must_use]
    pub fn get_query_set(&self, language: &str) -> Option<&QuerySet> {
        self.query_sets.get(language)
    }
}

impl Default for QueryLoader {
    fn default() -> Self {
        Self::new()
    }
}
