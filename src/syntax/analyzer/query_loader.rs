//! Load Tree-sitter queries from files.

use tree_sitter::Query;

use crate::input::source::ProgrammingLanguage;

/// クエリをロード
///
/// # Errors
/// クエリのパースに失敗した場合、空の Vec を返す
#[must_use]
pub fn load_queries(language: ProgrammingLanguage) -> Vec<Query> {
    let mut queries = Vec::new();

    let tree_sitter_lang = language.tree_sitter_language();
    let i18next_query = include_str!("../../../queries/javascript/react-i18next.scm");

    match Query::new(&tree_sitter_lang, i18next_query) {
        Ok(query) => queries.push(query),
        Err(err) => {
            tracing::error!("Failed to parse i18next query: {err:?}");
        }
    }

    queries
}
