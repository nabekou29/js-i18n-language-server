//! 翻訳関数呼び出しの抽出機能
//!
//! tree-sitterのクエリを使用して翻訳関数の呼び出しを検出し、情報を抽出します。

use tree_sitter::{
    Query,
    QueryCursor,
    QueryMatch,
    StreamingIterator,
    Tree,
};

use crate::analyzer::query_loader::QuerySet;
use crate::analyzer::types::{
    TranslationCall,
    TranslationKey,
};

/// クエリベースでASTから翻訳関数呼び出しを抽出
///
/// # Errors
///
/// クエリの実行やテキスト抽出でエラーが発生した場合
pub fn extract_translation_calls_with_queries(
    tree: &Tree,
    source: &str,
    query_set: &QuerySet,
) -> Result<Vec<TranslationCall>, Box<dyn std::error::Error>> {
    let mut calls = Vec::new();
    let root_node = tree.root_node();
    let source_bytes = source.as_bytes();

    // 各クエリを実行
    for query in query_set.all_queries() {
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(query, root_node, source_bytes);

        while let Some(match_) = matches.next() {
            if let Some(call) = extract_call_from_match(match_, query, source_bytes)? {
                calls.push(call);
            }
        }
    }

    // 位置でソート
    calls.sort_by_key(|call| (call.start_line, call.start_column));

    Ok(calls)
}

/// `QueryMatchから翻訳呼び出し情報を抽出`
fn extract_call_from_match(
    match_: &QueryMatch<'_, '_>,
    query: &Query,
    source: &[u8],
) -> Result<Option<TranslationCall>, Box<dyn std::error::Error>> {
    let mut key_text: Option<String> = None;
    let mut namespace: Option<String> = None;
    let mut key_prefix: Option<String> = None;
    let mut function_name: Option<String> = None;
    let mut call_node = None;

    // キャプチャから情報を抽出
    for capture in match_.captures {
        let capture_name =
            query.capture_names().get(capture.index as usize).ok_or("Invalid capture index")?;
        let node = capture.node;

        match *capture_name {
            "i18n.key" => {
                key_text = Some(node.utf8_text(source)?.to_string());
            }
            "i18n.namespace" => {
                namespace = Some(node.utf8_text(source)?.to_string());
            }
            "i18n.key_prefix" => {
                key_prefix = Some(node.utf8_text(source)?.to_string());
            }
            "i18n.trans_func_name" => {
                function_name = Some(node.utf8_text(source)?.to_string());
            }
            "func_name" => {
                if function_name.is_none() {
                    function_name = Some(node.utf8_text(source)?.to_string());
                }
            }
            "i18n.call_trans_func" => {
                call_node = Some(node);
            }
            _ => {}
        }
    }

    // 翻訳キーが見つかった場合のみ処理
    if let (Some(key), Some(node)) = (key_text, call_node) {
        let mut translation_key = TranslationKey::new(key);

        if let Some(ns) = namespace {
            translation_key = translation_key.with_namespace(ns);
        }

        if let Some(prefix) = key_prefix {
            translation_key = translation_key.with_key_prefix(prefix);
        }

        let start_pos = node.start_position();
        let end_pos = node.end_position();

        return Ok(Some(TranslationCall::new(
            translation_key,
            start_pos.row + 1, // 1-indexed
            start_pos.column + 1,
            end_pos.row + 1,
            end_pos.column + 1,
            function_name.unwrap_or_else(|| "t".to_string()),
        )));
    }

    Ok(None)
}

/// 従来の単純な抽出（後方互換性のため）
///
/// # Errors
///
/// ASTの走査やテキスト抽出でエラーが発生した場合
pub fn extract_translation_calls(
    tree: &Tree,
    source: &str,
) -> Result<Vec<TranslationCall>, Box<dyn std::error::Error>> {
    let mut calls = Vec::new();
    let root_node = tree.root_node();

    extract_simple_calls(&root_node, source, &mut calls)?;

    Ok(calls)
}

/// シンプルな関数呼び出しパターンを抽出（後方互換性のため）
fn extract_simple_calls(
    node: &tree_sitter::Node<'_>,
    source: &str,
    calls: &mut Vec<TranslationCall>,
) -> Result<(), Box<dyn std::error::Error>> {
    if node.kind() == "call_expression" {
        if let Some(call) = extract_single_call(node, source)? {
            calls.push(call);
        }
    }

    for child in node.children(&mut node.walk()) {
        extract_simple_calls(&child, source, calls)?;
    }

    Ok(())
}

/// 単一の関数呼び出しから情報を抽出（後方互換性のため）
fn extract_single_call(
    node: &tree_sitter::Node<'_>,
    source: &str,
) -> Result<Option<TranslationCall>, Box<dyn std::error::Error>> {
    let function_node = node.child_by_field_name("function");
    let arguments_node = node.child_by_field_name("arguments");

    if let (Some(func), Some(args)) = (function_node, arguments_node) {
        let function_name = match func.kind() {
            "identifier" => func.utf8_text(source.as_bytes())?.to_string(),
            "member_expression" => {
                if let (Some(obj), Some(prop)) =
                    (func.child_by_field_name("object"), func.child_by_field_name("property"))
                {
                    format!(
                        "{}.{}",
                        obj.utf8_text(source.as_bytes())?,
                        prop.utf8_text(source.as_bytes())?
                    )
                } else {
                    return Ok(None);
                }
            }
            _ => return Ok(None),
        };

        if !is_translation_function(&function_name) {
            return Ok(None);
        }

        if let Some(first_arg) = args.children(&mut args.walk()).nth(1) {
            if first_arg.kind() == "string" {
                if let Some(string_content) =
                    first_arg.child(1).filter(|n| n.kind() == "string_fragment")
                {
                    let key_text = string_content.utf8_text(source.as_bytes())?;

                    let start_pos = node.start_position();
                    let end_pos = node.end_position();

                    return Ok(Some(TranslationCall::new(
                        TranslationKey::new(key_text),
                        start_pos.row + 1,
                        start_pos.column + 1,
                        end_pos.row + 1,
                        end_pos.column + 1,
                        function_name,
                    )));
                }
            }
        }
    }

    Ok(None)
}

/// 翻訳関数かどうかを判定
fn is_translation_function(name: &str) -> bool {
    matches!(name, "t" | "i18n.t" | "i18next.t" | "translate" | "i18n.translate")
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use tree_sitter::{
        Language,
        Parser,
    };

    use super::*;

    fn javascript_language() -> Language {
        tree_sitter_javascript::LANGUAGE.into()
    }

    #[test]
    fn test_extract_simple_t_call() {
        let mut parser = Parser::new();
        parser.set_language(&javascript_language()).unwrap();

        let code = r#"const message = t("hello.world");"#;
        let tree = parser.parse(code, None).unwrap();

        let calls = extract_translation_calls(&tree, code).unwrap();

        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].key.key, "hello.world");
        assert_eq!(calls[0].function_name, "t");
    }

    #[test]
    fn test_extract_i18n_t_call() {
        let mut parser = Parser::new();
        parser.set_language(&javascript_language()).unwrap();

        let code = r#"const message = i18n.t("common.greeting");"#;
        let tree = parser.parse(code, None).unwrap();

        let calls = extract_translation_calls(&tree, code).unwrap();

        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].key.key, "common.greeting");
        assert_eq!(calls[0].function_name, "i18n.t");
    }

    #[test]
    fn test_multiple_calls() {
        let mut parser = Parser::new();
        parser.set_language(&javascript_language()).unwrap();

        let code = r#"
            const a = t("first.key");
            const b = i18n.t("second.key");
            const c = t("third.key");
        "#;
        let tree = parser.parse(code, None).unwrap();

        let calls = extract_translation_calls(&tree, code).unwrap();

        assert_eq!(calls.len(), 3);
        assert_eq!(calls[0].key.key, "first.key");
        assert_eq!(calls[1].key.key, "second.key");
        assert_eq!(calls[2].key.key, "third.key");
    }
}
