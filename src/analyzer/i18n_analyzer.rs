//! i18n分析器の実装
//!
//! JavaScript/TypeScriptコードを解析し、i18n関連の情報を抽出します。

use tree_sitter::{Language, Parser};

use crate::analyzer::extractor::{
    extract_translation_calls, extract_translation_calls_with_queries,
};
use crate::analyzer::query_loader::QueryLoader;
use crate::analyzer::types::TranslationCall;

/// `JavaScript言語を取得`
fn javascript_language() -> Language {
    tree_sitter_javascript::LANGUAGE.into()
}

/// `TypeScript言語を取得`
fn typescript_language() -> Language {
    tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
}

/// TSX言語を取得
fn tsx_language() -> Language {
    tree_sitter_typescript::LANGUAGE_TSX.into()
}

/// i18n分析器
pub struct I18nAnalyzer {
    /// `JavaScript用パーサー`
    js_parser: Parser,
    /// `TypeScript用パーサー`
    ts_parser: Parser,
    /// TSX用パーサー
    tsx_parser: Parser,
    /// クエリローダー
    query_loader: QueryLoader,
}

impl I18nAnalyzer {
    /// `新しいI18nAnalyzerインスタンスを作成`
    ///
    /// # Errors
    ///
    /// パーサーの初期化やクエリの読み込みに失敗した場合
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let mut js_parser = Parser::new();
        js_parser.set_language(&javascript_language())?;

        let mut ts_parser = Parser::new();
        ts_parser.set_language(&typescript_language())?;

        let mut tsx_parser = Parser::new();
        tsx_parser.set_language(&tsx_language())?;

        let mut query_loader = QueryLoader::new();
        // クエリをロード
        query_loader.load_javascript_queries(&javascript_language())?;
        query_loader.load_typescript_queries(&typescript_language())?;
        query_loader.load_tsx_queries(&tsx_language())?;

        Ok(Self { js_parser, ts_parser, tsx_parser, query_loader })
    }

    /// ファイルを解析して翻訳関数呼び出しを抽出
    ///
    /// # Errors
    ///
    /// ファイルの解析や翻訳関数の抽出に失敗した場合
    pub fn analyze_file(
        &mut self,
        content: &str,
        file_extension: &str,
    ) -> Result<Vec<TranslationCall>, Box<dyn std::error::Error>> {
        let (parser, query_lang) = match file_extension {
            "js" | "jsx" => (&mut self.js_parser, "javascript"),
            "ts" => (&mut self.ts_parser, "typescript"),
            "tsx" => (&mut self.tsx_parser, "tsx"),
            _ => return Err("Unsupported file extension".into()),
        };

        let tree = parser.parse(content, None).ok_or("Failed to parse file")?;

        // クエリベースの抽出を試みる
        self.query_loader.get_query_set(query_lang).map_or_else(
            || extract_translation_calls(&tree, content),
            |query_set| extract_translation_calls_with_queries(&tree, content, query_set),
        )
    }
}

impl std::fmt::Debug for I18nAnalyzer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("I18nAnalyzer")
            .field("js_parser", &"Parser<JavaScript>")
            .field("ts_parser", &"Parser<TypeScript>")
            .field("tsx_parser", &"Parser<TSX>")
            .field("query_loader", &self.query_loader)
            .finish()
    }
}

impl Default for I18nAnalyzer {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| {
            // デフォルトパーサーを作成
            Self {
                js_parser: Parser::new(),
                ts_parser: Parser::new(),
                tsx_parser: Parser::new(),
                query_loader: QueryLoader::default(),
            }
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn test_analyzer_creation() {
        let analyzer = I18nAnalyzer::new();
        assert!(analyzer.is_ok());
    }

    #[test]
    fn test_simple_javascript() {
        let mut analyzer = I18nAnalyzer::new().unwrap();
        let content = r#"
            import { t } from "i18next";
            const message = t("hello.world");
        "#;

        let result = analyzer.analyze_file(content, "js");
        assert!(result.is_ok());

        let calls = result.unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].key.key, "hello.world");
        assert_eq!(calls[0].function_name, "t");
    }
}
