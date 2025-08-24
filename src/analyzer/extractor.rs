//! Extracts function calls to `trans_fn` from a given source code file using Tree-sitter.

use std::string::ToString;

use tree_sitter::{
    Language,
    Node,
    Parser,
    Query,
    QueryCursor,
    StreamingIteratorMut,
};

use crate::analyzer::error::AnalyzerError;
use crate::analyzer::scope::{
    ScopeInfo,
    Scopes,
};
use crate::analyzer::types::{
    CallTransFnDetail,
    GetTransFnDetail,
    TransFnCall,
    capture_names,
};

/// Extracts text content from a tree-sitter node
fn extract_node_text(node: Node<'_>, source_bytes: &[u8]) -> Option<String> {
    node.utf8_text(source_bytes).ok().map(ToString::to_string)
}

/// Finds the closest ancestor node of a given type
fn get_closest_node<'a>(node: Node<'a>, target_types: &[&str]) -> Option<Node<'a>> {
    let mut current_node = node;

    while let Some(parent) = current_node.parent() {
        println!("Checking parent node: {:?}", parent.kind());
        if target_types.contains(&parent.kind()) {
            return Some(parent);
        }
        current_node = parent;
    }

    None
}

/// Extracts translation function calls from a Tree-sitter syntax tree.
///
/// # Errors
/// Returns `AnalyzerError` if:
/// - Language setup fails
/// - Source code parsing fails
/// - Query execution encounters issues
pub fn analyze_trans_fn_calls(
    source: &str,
    language: &Language,
    queries: &[Query],
) -> Result<Vec<TransFnCall>, AnalyzerError> {
    let mut parser = Parser::new();
    parser.set_language(language).map_err(AnalyzerError::LanguageSetup)?;
    let tree = parser.parse(source, None).ok_or(AnalyzerError::ParseFailed)?;

    let source_bytes = source.as_bytes();

    let calls = Vec::new();
    let root_node = tree.root_node();

    let mut scopes = Scopes::new();

    // デフォルトのスコープを追加
    scopes.push_scope("t".to_string(), ScopeInfo::new(root_node, GetTransFnDetail::new("t")));

    for query in queries {
        let cap_names = query.capture_names();
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(query, root_node, source_bytes);

        while let Some(match_) = matches.next_mut() {
            for capture in match_.captures {
                let cap_name = cap_names.get(capture.index as usize);
                let Some(cap_name) = cap_name else {
                    continue;
                };

                match *cap_name {
                    capture_names::GET_TRANS_FN => {
                        let Ok(trans_fn) = parse_get_trans_fn_captures(
                            query,
                            capture.node,
                            source_bytes,
                            cap_names,
                        ) else {
                            continue;
                        };
                        println!("Found: {trans_fn:?}");

                        let scope_node =
                            get_closest_node(capture.node, &["statement_block", "jsx_element"]);
                        println!("Scope Node: {scope_node:?}");
                    }
                    capture_names::CALL_TRANS_FN => {
                        let Ok(call_trans_fn) = parse_call_trans_fn_captures(
                            query,
                            capture.node,
                            source_bytes,
                            cap_names,
                        ) else {
                            continue;
                        };
                        println!("Found: {call_trans_fn:?}");
                    }

                    _ => {}
                }
            }
        }
    }

    Ok(calls)
}

/// Parses get translation function captures from a tree-sitter node
///
/// # Arguments
/// * `query` - The tree-sitter query to execute
/// * `capture_node` - The node to analyze for captures
/// * `source_bytes` - Source code as bytes for text extraction
/// * `cap_names` - Capture names from the query
///
/// # Errors
/// Returns `AnalyzerError::ParseFailed` if required captures are missing
fn parse_get_trans_fn_captures(
    query: &Query,
    capture_node: Node<'_>,
    source_bytes: &[u8],
    cap_names: &[&str],
) -> Result<GetTransFnDetail, AnalyzerError> {
    let mut trans_fn_name: Option<String> = None;
    let mut namespace = None; // TODO: 将来的にキャプチャから取得
    let mut key_prefix = None; // TODO: 将来的にキャプチャから取得

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(query, capture_node, source_bytes);

    while let Some(match_) = matches.next_mut() {
        for capture in match_.captures {
            let Some(cap_name) = cap_names.get(capture.index as usize) else {
                continue; // 無効なインデックスの場合はスキップ
            };

            match *cap_name {
                capture_names::TRANS_FN_NAME => {
                    trans_fn_name = extract_node_text(capture.node, source_bytes);
                }
                capture_names::NAMESPACE => {
                    namespace = extract_node_text(capture.node, source_bytes);
                }
                capture_names::KEY_PREFIX => {
                    key_prefix = extract_node_text(capture.node, source_bytes);
                }
                _ => {} // その他のキャプチャは無視
            }
        }
    }

    let mut detail = GetTransFnDetail::new(trans_fn_name.ok_or(AnalyzerError::ParseFailed)?);

    if let Some(ns) = namespace {
        detail = detail.with_namespace(ns);
    }
    if let Some(prefix) = key_prefix {
        detail = detail.with_key_prefix(prefix);
    }

    Ok(detail)
}

/// Parses call translation function captures from a tree-sitter node
///
/// # Arguments
/// * `query` - The tree-sitter query to execute
/// * `capture_node` - The node to analyze for captures
/// * `source_bytes` - Source code as bytes for text extraction
/// * `cap_names` - Capture names from the query
///
/// # Errors
/// Returns `AnalyzerError::ParseFailed` if required captures are missing
fn parse_call_trans_fn_captures<'a>(
    query: &Query,
    capture_node: Node<'a>,
    source_bytes: &[u8],
    cap_names: &[&str],
) -> Result<CallTransFnDetail<'a>, AnalyzerError> {
    let mut trans_fn_name: Option<String> = None;
    let mut key: Option<String> = None;
    let mut key_node: Option<Node<'a>> = None;
    let mut key_arg_node: Option<Node<'a>> = None;
    let namespace = None;
    let key_prefix = None;

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(query, capture_node, source_bytes);

    while let Some(match_) = matches.next_mut() {
        for capture in match_.captures {
            let Some(cap_name) = cap_names.get(capture.index as usize) else {
                continue; // 無効なインデックスの場合はスキップ
            };

            match *cap_name {
                capture_names::TRANS_KEY => {
                    key = extract_node_text(capture.node, source_bytes);
                    key_node = Some(capture.node);
                }
                capture_names::TRANS_KEY_ARG => {
                    key_arg_node = Some(capture.node);
                }
                capture_names::TRANS_FN_NAME => {
                    trans_fn_name = extract_node_text(capture.node, source_bytes);
                }
                _ => {} // 予期しないキャプチャ名
            }
        }
    }

    Ok(CallTransFnDetail {
        trans_fn_name: trans_fn_name.ok_or(AnalyzerError::ParseFailed)?,
        key: key.ok_or(AnalyzerError::ParseFailed)?,
        key_node: key_node.ok_or(AnalyzerError::ParseFailed)?,
        key_arg_node: key_arg_node.ok_or(AnalyzerError::ParseFailed)?,
        namespace,
        key_prefix,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::expect_used)]
mod tests {
    use tree_sitter::Language;

    use super::*;

    fn javascript_language() -> Language {
        tree_sitter_javascript::LANGUAGE.into()
    }

    fn load_queries() -> Vec<Query> {
        let mut queries = Vec::new();

        let i18next_query = include_str!("../../queries/javascript/react-i18next.scm");
        queries.push(
            Query::new(&javascript_language(), i18next_query)
                .expect("Failed to parse i18next query"),
        );
        queries
    }

    #[test]
    fn test_analyze_simple_trans_fn_calls() {
        let queries = load_queries();

        let code = r#"
            const { t } = useTranslation();
            const message = t("key")
            "#;

        let calls = analyze_trans_fn_calls(code, &javascript_language(), &queries).unwrap();

        assert_eq!(calls.len(), 1);
    }

    #[test]
    fn test_analyze_nested_trans_fn_calls() {
        let queries = load_queries();

        let code = r#"
            function example() {
                const { t } = useTranslation();
                const message = t("key1");
                if (true) {
                    const nestedMessage = t("key2");
                }
            }
            "#;

        let calls = analyze_trans_fn_calls(code, &javascript_language(), &queries).unwrap();

        assert_eq!(calls.len(), 2);
    }
}
