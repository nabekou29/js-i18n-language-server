//! Extracts function calls to `trans_fn` from a given source code file using Tree-sitter.

use std::string::ToString;

use tower_lsp::lsp_types::{
    Position,
    Range,
};
use tree_sitter::{
    Language,
    Node,
    Parser,
    Query,
    QueryCursor,
    StreamingIteratorMut,
};

use crate::syntax::analyzer::scope::{
    ScopeInfo,
    Scopes,
};
use crate::syntax::analyzer::types::{
    AnalyzerError,
    CallTransFnDetail,
    CaptureName,
    GetTransFnDetail,
    TransFnCall,
};

/// Extracts text content from a tree-sitter node
fn extract_node_text(node: Node<'_>, source_bytes: &[u8]) -> Option<String> {
    node.utf8_text(source_bytes).ok().map(ToString::to_string)
}

/// Extracts the base function name from a node
///
/// For member expressions like `t.rich("key")`, this returns "t".
/// For identifiers, this returns the identifier text.
fn extract_base_function_name(node: Node<'_>, source_bytes: &[u8]) -> Option<String> {
    match node.kind() {
        "identifier" => extract_node_text(node, source_bytes),
        "member_expression" => node
            .child_by_field_name("object")
            .and_then(|obj| extract_base_function_name(obj, source_bytes)),
        _ => None,
    }
}

/// Finds the closest ancestor node of a given type
fn get_closest_node<'a>(node: Node<'a>, target_types: &[&str]) -> Option<Node<'a>> {
    let mut current_node = node;

    while let Some(parent) = current_node.parent() {
        if target_types.contains(&parent.kind()) {
            return Some(parent);
        }
        current_node = parent;
    }

    None
}

/// Gets the range of a tree-sitter node
#[allow(clippy::cast_possible_truncation)] // ソースファイルの行・列が42億を超えることはない
fn get_node_range(node: Node<'_>) -> Range {
    let start_pos = node.start_position();
    let end_pos = node.end_position();
    Range::new(
        Position::new(start_pos.row as u32, start_pos.column as u32),
        Position::new(end_pos.row as u32, end_pos.column as u32),
    )
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
    key_separator: &str,
) -> Result<Vec<TransFnCall>, AnalyzerError> {
    let mut parser = Parser::new();
    parser.set_language(language).map_err(AnalyzerError::LanguageSetup)?;
    let tree = parser.parse(source, None).ok_or(AnalyzerError::ParseFailed)?;

    let source_bytes = source.as_bytes();

    let mut calls = Vec::new();
    let root_node = tree.root_node();

    let mut scopes = Scopes::new();

    // デフォルトのスコープを追加
    scopes.push_scope("t".to_string(), ScopeInfo::new(root_node, GetTransFnDetail::new("t")));

    // 複数のクエリからのマッチを収集し、ソースコードの位置順で処理する。
    // これにより、i18next.scm と react-i18next.scm など別々のクエリファイルでも、
    // 翻訳関数の定義が関数呼び出しより前に処理されることが保証される。

    // 全てのクエリからキャプチャを収集
    let mut all_captures: Vec<(usize, CaptureName, Node<'_>, usize)> = Vec::new();

    for (query_idx, query) in queries.iter().enumerate() {
        let cap_names = query.capture_names();
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(query, root_node, source_bytes);

        while let Some(match_) = matches.next_mut() {
            for capture in match_.captures {
                let cap_name = cap_names.get(capture.index as usize);
                let Some(cap_name) = cap_name else {
                    continue;
                };

                let Ok(capture_name) = cap_name.parse::<CaptureName>() else {
                    continue;
                };

                // GetTransFn と CallTransFn のみを収集
                if matches!(capture_name, CaptureName::GetTransFn | CaptureName::CallTransFn) {
                    all_captures.push((
                        query_idx,
                        capture_name,
                        capture.node,
                        capture.node.start_byte(),
                    ));
                }
            }
        }
    }

    // ソースコードの位置順でソート（開始位置が同じ場合は GetTransFn を先に）
    all_captures.sort_by(|a, b| {
        a.3.cmp(&b.3).then_with(|| {
            // GetTransFn を CallTransFn より先に処理する
            match (&a.1, &b.1) {
                (CaptureName::GetTransFn, CaptureName::CallTransFn) => std::cmp::Ordering::Less,
                (CaptureName::CallTransFn, CaptureName::GetTransFn) => std::cmp::Ordering::Greater,
                _ => std::cmp::Ordering::Equal,
            }
        })
    });

    // 位置順で処理
    for (query_idx, capture_name, node, _) in all_captures {
        let Some(query) = queries.get(query_idx) else {
            continue;
        };
        let cap_names = query.capture_names();

        match capture_name {
            CaptureName::GetTransFn => {
                let Ok(trans_fn) =
                    parse_get_trans_fn_captures(query, node, source_bytes, cap_names)
                else {
                    continue;
                };

                cleanup_out_of_scopes(&mut scopes, &trans_fn.trans_fn_name, node);

                let scope_node = get_closest_node(node, &["statement_block", "jsx_element"])
                    .unwrap_or(root_node);

                let trans_fn_name = trans_fn.trans_fn_name.clone();
                scopes.push_scope(trans_fn_name, ScopeInfo::new(scope_node, trans_fn));
            }
            CaptureName::CallTransFn => {
                let Ok(call_trans_fn) =
                    parse_call_trans_fn_captures(query, node, source_bytes, cap_names)
                else {
                    continue;
                };

                // 現在のスコープに存在しない翻訳関数は無視
                if !scopes.has_scope(&call_trans_fn.trans_fn_name) {
                    continue;
                }

                cleanup_out_of_scopes(&mut scopes, &call_trans_fn.trans_fn_name, node);

                // スコープ情報を取得
                // Trans コンポーネントで t 属性がない場合はデフォルトのスコープを使用
                let scope_info = if call_trans_fn.use_default_scope {
                    scopes.default_scope(&call_trans_fn.trans_fn_name)
                } else {
                    scopes.current_scope(&call_trans_fn.trans_fn_name)
                };
                let Some(scope_info) = scope_info else {
                    continue;
                };

                let arg_key_node = call_trans_fn.arg_key_node;

                calls.push(TransFnCall {
                    key: scope_info.trans_fn.key_prefix.as_ref().map_or_else(
                        || call_trans_fn.key.clone(),
                        |prefix| format!("{}{}{}", prefix, key_separator, &call_trans_fn.key),
                    ),
                    arg_key: call_trans_fn.key.clone(),
                    arg_key_node: get_node_range(arg_key_node),
                    key_prefix: scope_info.trans_fn.key_prefix.clone(),
                });
            }
            _ => {}
        }
    }

    Ok(calls)
}

/// スコープから外れた場合に自動的にポップする
fn cleanup_out_of_scopes(scopes: &mut Scopes<'_>, trans_fn_name: &str, current_node: Node<'_>) {
    while scopes.current_scope(trans_fn_name).is_some()
        && !scopes.is_node_in_current_scope(trans_fn_name, current_node)
    {
        scopes.pop_scope(trans_fn_name);
    }
}

/// Extracts string arguments from an arguments node
///
/// Returns a vector of string values from the arguments, in order.
/// Only string literals are extracted; other argument types are represented as None.
fn extract_string_arguments(args_node: Node<'_>, source_bytes: &[u8]) -> Vec<Option<String>> {
    let mut strings = Vec::new();

    for i in 0..args_node.named_child_count() {
        #[allow(clippy::cast_possible_truncation)] // 引数の数が 42 億を超えることはない
        if let Some(child) = args_node.named_child(i as u32) {
            if child.kind() == "string"
                && let Some(fragment) = child.named_child(0)
                && fragment.kind() == "string_fragment"
            {
                strings.push(extract_node_text(fragment, source_bytes));
                continue;
            }
            // Non-string argument (null, undefined, variable, etc.)
            strings.push(None);
        }
    }

    strings
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
    let mut namespace = None;
    let mut key_prefix = None;
    let mut args_node: Option<Node<'_>> = None;
    let mut func_name: Option<String> = None;

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(query, capture_node, source_bytes);

    while let Some(match_) = matches.next_mut() {
        for capture in match_.captures {
            let Some(cap_name) = cap_names.get(capture.index as usize) else {
                continue;
            };

            let Ok(capture_name) = cap_name.parse::<CaptureName>() else {
                // Check for function-specific captures that aren't CaptureName variants
                if *cap_name == "get_fixed_t_func" || *cap_name == "use_translations" {
                    func_name = extract_node_text(capture.node, source_bytes);
                }
                continue;
            };

            match capture_name {
                CaptureName::GetTransFnName => {
                    trans_fn_name = extract_node_text(capture.node, source_bytes);
                }
                CaptureName::Namespace => {
                    namespace = extract_node_text(capture.node, source_bytes);
                }
                CaptureName::KeyPrefix => {
                    key_prefix = extract_node_text(capture.node, source_bytes);
                }
                CaptureName::GetTransFnArgs => {
                    args_node = Some(capture.node);
                }
                _ => {}
            }
        }
    }

    // If we have args_node and func_name, extract library-specific arguments
    if let (Some(args), Some(func)) = (args_node, &func_name) {
        let string_args = extract_string_arguments(args, source_bytes);

        if func.ends_with("getFixedT") {
            // i18next: getFixedT(lang, ns?, keyPrefix?)
            // - Index 0: lang (ignored)
            // - Index 1: namespace
            // - Index 2: keyPrefix
            if string_args.len() >= 2 {
                namespace = string_args.get(1).and_then(Option::clone);
            }
            if string_args.len() >= 3 {
                key_prefix = string_args.get(2).and_then(Option::clone);
            }
        } else if func == "useTranslations" {
            // next-intl: useTranslations(namespace?)
            // The namespace in next-intl acts as a key prefix
            if let Some(Some(ns)) = string_args.first() {
                key_prefix = Some(ns.clone());
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
    let mut trans_args_node: Option<Node<'a>> = None;

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(query, capture_node, source_bytes);

    while let Some(match_) = matches.next_mut() {
        for capture in match_.captures {
            let Some(cap_name) = cap_names.get(capture.index as usize) else {
                continue; // 無効なインデックスの場合はスキップ
            };

            let Ok(capture_name) = cap_name.parse::<CaptureName>() else {
                continue;
            };

            match capture_name {
                CaptureName::TransKey => {
                    key = extract_node_text(capture.node, source_bytes);
                    key_node = Some(capture.node);
                }
                CaptureName::TransKeyArg => {
                    key_arg_node = Some(capture.node);
                }
                CaptureName::CallTransFnName => {
                    // member_expression の場合、base 関数名を抽出 (e.g., t.rich -> t)
                    trans_fn_name = extract_base_function_name(capture.node, source_bytes);
                }
                CaptureName::TransArgs => {
                    trans_args_node = Some(capture.node);
                }
                _ => {}
            }
        }
    }

    // 引数ノードの決定: 文字列引数があればそれを使用、なければ空の引数かチェック
    let arg_key_node = if let Some(node) = key_arg_node {
        node
    } else if let Some(args_node) = trans_args_node {
        let args_text =
            args_node.utf8_text(source_bytes).map_err(|_| AnalyzerError::ParseFailed)?;
        let inner = args_text.trim_start_matches('(').trim_end_matches(')').trim();

        if inner.is_empty() {
            args_node
        } else {
            // t(someVar) など文字列以外の引数は無効
            return Err(AnalyzerError::ParseFailed);
        }
    } else {
        return Err(AnalyzerError::ParseFailed);
    };

    // Trans コンポーネントで t 属性がない場合、デフォルトのスコープの "t" を使用
    let use_default_scope = trans_fn_name.is_none();

    Ok(CallTransFnDetail {
        trans_fn_name: trans_fn_name.unwrap_or_else(|| "t".to_string()),
        key: key.unwrap_or_default(),
        key_node: key_node.unwrap_or(arg_key_node),
        arg_key_node,
        use_default_scope,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::expect_used, clippy::panic)]
mod tests {

    use googletest::prelude::*;
    use rstest::*;
    use tree_sitter::{
        Language,
        Query,
    };

    use super::*;

    /// JavaScript 言語パーサー
    #[fixture]
    fn js_lang() -> Language {
        tree_sitter_javascript::LANGUAGE.into()
    }

    /// Tree-sitter クエリ（すべてのライブラリ対応）
    #[fixture]
    fn queries(js_lang: Language) -> Vec<Query> {
        let query_files = [
            ("react-i18next", include_str!("../../../queries/javascript/react-i18next.scm")),
            ("i18next", include_str!("../../../queries/javascript/i18next.scm")),
            ("next-intl", include_str!("../../../queries/javascript/next-intl.scm")),
        ];

        query_files
            .iter()
            .map(|(name, content)| {
                Query::new(&js_lang, content)
                    .unwrap_or_else(|e| panic!("Failed to parse {name} query: {e}"))
            })
            .collect()
    }

    #[rstest]
    fn test_simple_translation(queries: Vec<Query>, js_lang: Language) {
        let code = r#"
            const { t } = useTranslation();
            const message = t("hello.world");
            "#;

        let calls = analyze_trans_fn_calls(code, &js_lang, &queries, ".").unwrap();

        assert_that!(
            calls,
            elements_are![all![
                field!(TransFnCall.key, eq("hello.world")),
                field!(TransFnCall.arg_key, eq("hello.world"))
            ]]
        );
    }

    #[rstest]
    fn test_multiple_translations(queries: Vec<Query>, js_lang: Language) {
        let code = r#"
            const { t } = useTranslation();
            const message1 = t("key1");
            const message2 = t("key2");
            const message3 = t("key3");
            "#;

        let calls = analyze_trans_fn_calls(code, &js_lang, &queries, ".").unwrap();

        assert_that!(
            calls,
            elements_are![
                field!(TransFnCall.key, eq("key1")),
                field!(TransFnCall.key, eq("key2")),
                field!(TransFnCall.key, eq("key3"))
            ]
        );
    }

    #[rstest]
    fn test_custom_variable_name(queries: Vec<Query>, js_lang: Language) {
        let code = r#"
            const { t: translate } = useTranslation();
            const message = translate("custom.key");
            "#;

        let calls = analyze_trans_fn_calls(code, &js_lang, &queries, ".").unwrap();

        assert_that!(
            calls,
            elements_are![all![
                field!(TransFnCall.key, eq("custom.key")),
                field!(TransFnCall.arg_key, eq("custom.key"))
            ]]
        );
    }

    #[rstest]
    fn test_function_scope_isolation(queries: Vec<Query>, js_lang: Language) {
        let code = r#"
            function funcA() {
                const { t } = useTranslation();
                t("funcA.key");
            }

            function funcB() {
                const { t } = useTranslation();
                t("funcB.key");
            }
            "#;

        let calls = analyze_trans_fn_calls(code, &js_lang, &queries, ".").unwrap();

        assert_that!(
            calls,
            elements_are![
                field!(TransFnCall.key, eq("funcA.key")),
                field!(TransFnCall.key, eq("funcB.key"))
            ]
        );
    }

    #[rstest]
    fn test_block_scope_isolation(queries: Vec<Query>, js_lang: Language) {
        let code = r#"
            const { t } = useTranslation();
            t("outer.key");

            if (true) {
                const { t } = useTranslation();
                t("block.key");
            }

            t("outer.key2");
            "#;

        let calls = analyze_trans_fn_calls(code, &js_lang, &queries, ".").unwrap();

        assert_that!(
            calls,
            elements_are![
                field!(TransFnCall.key, eq("outer.key")),
                field!(TransFnCall.key, eq("block.key")),
                field!(TransFnCall.key, eq("outer.key2"))
            ]
        );
    }

    #[rstest]
    fn test_nested_scopes(queries: Vec<Query>, js_lang: Language) {
        let code = r#"
            function outer() {
                const { t } = useTranslation();
                t("outer.key");

                if (true) {
                    const { t } = useTranslation();
                    t("nested.key");

                    if (true) {
                        t("deeply.nested.key");
                    }
                }

                t("outer.key2");
            }
            "#;

        let calls = analyze_trans_fn_calls(code, &js_lang, &queries, ".").unwrap();

        assert_that!(
            calls,
            elements_are![
                field!(TransFnCall.key, eq("outer.key")),
                field!(TransFnCall.key, eq("nested.key")),
                field!(TransFnCall.key, eq("deeply.nested.key")),
                field!(TransFnCall.key, eq("outer.key2"))
            ]
        );
    }

    #[rstest]
    fn test_scope_shadowing(queries: Vec<Query>, js_lang: Language) {
        let code = r#"
            const { t } = useTranslation();
            t("original.key");

            {
                const { t } = useTranslation();
                t("shadowed.key");
            }

            t("original.key2");
            "#;

        let calls = analyze_trans_fn_calls(code, &js_lang, &queries, ".").unwrap();

        assert_that!(
            calls,
            elements_are![
                field!(TransFnCall.key, eq("original.key")),
                field!(TransFnCall.key, eq("shadowed.key")),
                field!(TransFnCall.key, eq("original.key2"))
            ]
        );
    }

    // 3. key_prefix機能テスト

    #[rstest]
    fn test_key_prefix_application(queries: Vec<Query>, js_lang: Language) {
        let code = r#"
            const { t } = useTranslation("translation", { keyPrefix: "common" });
            const message = t("button.save");
            "#;

        let calls = analyze_trans_fn_calls(code, &js_lang, &queries, ".").unwrap();

        // プレフィックスが適用されたkeyと、元のarg_keyをチェック
        assert_that!(
            calls,
            elements_are![all![
                field!(TransFnCall.key, eq("common.button.save")),
                field!(TransFnCall.arg_key, eq("button.save"))
            ]]
        );
    }

    #[rstest]
    fn test_mixed_key_prefix(queries: Vec<Query>, js_lang: Language) {
        let code = r#"
            const { t } = useTranslation();
            t("no.prefix");

            {
                const { t } = useTranslation("translation", { keyPrefix: "form" });
                t("field.name");
                t("field.email");
            }

            t("no.prefix.again");
            "#;

        let calls = analyze_trans_fn_calls(code, &js_lang, &queries, ".").unwrap();

        assert_that!(
            calls,
            elements_are![
                field!(TransFnCall.key, eq("no.prefix")),
                field!(TransFnCall.key, eq("form.field.name")),
                field!(TransFnCall.key, eq("form.field.email")),
                field!(TransFnCall.key, eq("no.prefix.again"))
            ]
        );
    }

    #[rstest]
    fn test_no_key_prefix(queries: Vec<Query>, js_lang: Language) {
        let code = r#"
            const { t } = useTranslation();
            const message = t("simple.key");
            "#;

        let calls = analyze_trans_fn_calls(code, &js_lang, &queries, ".").unwrap();

        assert_that!(
            calls,
            elements_are![all![
                field!(TransFnCall.key, eq("simple.key")),
                field!(TransFnCall.arg_key, eq("simple.key"))
            ]]
        );
    }

    // 4. エッジケーステスト

    #[rstest]
    fn test_empty_code(queries: Vec<Query>, js_lang: Language) {
        let code = "";

        let calls = analyze_trans_fn_calls(code, &js_lang, &queries, ".").unwrap();

        assert_that!(calls, is_empty()); // 空チェックに最適
    }

    #[rstest]
    fn test_undefined_trans_fn(queries: Vec<Query>, js_lang: Language) {
        let code = r#"
            // 翻訳関数が定義されていない状態で呼び出し
            const message = t("undefined.key");
            "#;

        let calls = analyze_trans_fn_calls(code, &js_lang, &queries, ".").unwrap();

        // デフォルトスコープ "t" が存在するため、呼び出しは検出される
        assert_that!(calls, elements_are![field!(TransFnCall.key, eq("undefined.key"))]);
    }

    #[rstest]
    fn test_invalid_arguments(queries: Vec<Query>, js_lang: Language) {
        let code = r#"
            const { t } = useTranslation();

            // 有効な呼び出し
            t("valid.key");

            // 無効な呼び出し（数値引数）
            t(123);

            // 無効な呼び出し（変数引数）
            const key = "variable.key";
            t(key);

            // 無効な呼び出し（テンプレート文字列）
            t(`template.${key}`);
            "#;

        let calls = analyze_trans_fn_calls(code, &js_lang, &queries, ".").unwrap();

        // 文字列リテラルのみが有効
        assert_that!(calls, elements_are![field!(TransFnCall.key, eq("valid.key"))]);
    }

    // 4.5. テーブルドリブンテスト - 引数パターン

    /// 様々な引数パターンのテスト
    #[rstest]
    // 引用符のパターン
    #[case::double_quotes(r#"t("double.quotes")"#, "double.quotes")]
    #[case::single_quotes(r"t('single.quotes')", "single.quotes")]
    // 空白のパターン
    #[case::no_spaces(r#"t("no.spaces")"#, "no.spaces")]
    #[case::spaces_around(r#"t( "spaces.around" )"#, "spaces.around")]
    #[case::multiple_spaces(r#"t(  "multiple.spaces"  )"#, "multiple.spaces")]
    #[case::newlines("t(\n  \"newlines\"\n)", "newlines")]
    // 複雑なキー名
    #[case::dots_in_key(r#"t("section.subsection.item")"#, "section.subsection.item")]
    #[case::underscores(r#"t("snake_case_key")"#, "snake_case_key")]
    #[case::numbers(r#"t("item123.section456")"#, "item123.section456")]
    #[case::special_chars(r#"t("special-chars_key.item")"#, "special-chars_key.item")]
    fn test_various_argument_patterns(
        queries: Vec<Query>,
        js_lang: Language,
        #[case] t_call: &str,
        #[case] expected_key: &str,
    ) {
        let code = format!(
            "
            const {{ t }} = useTranslation();
            const message = {t_call};
            "
        );

        let calls = analyze_trans_fn_calls(&code, &js_lang, &queries, ".")
            .unwrap_or_else(|_| panic!("Failed to parse code for test case"));

        // keyとarg_keyの両方をチェック
        assert_that!(
            calls,
            elements_are![all![
                field!(TransFnCall.key, eq(expected_key)),
                field!(TransFnCall.arg_key, eq(expected_key))
            ]]
        );
    }

    /// 複数引数を持つ翻訳関数呼び出しのテスト
    #[rstest]
    #[case::with_object(r#"t("key.with.object", { count: 1 })"#)]
    #[case::with_number(r#"t("key.with.number", 42)"#)]
    #[case::with_variable(r#"t("key.with.variable", someVariable)"#)]
    #[case::with_multiple_args(r#"t("key.multiple.args", arg1, arg2, arg3)"#)]
    fn test_multiple_arguments_ignored(
        queries: Vec<Query>,
        js_lang: Language,
        #[case] t_call: &str,
    ) {
        let code = format!(
            "
            const {{ t }} = useTranslation();
            const message = {t_call}
            "
        );

        let calls = analyze_trans_fn_calls(&code, &js_lang, &queries, ".")
            .unwrap_or_else(|_| panic!("Failed to parse code for test case"));

        // 期待される検出数と、最初のキーが"key."で始まることを確認
        assert_that!(calls, elements_are![field!(TransFnCall.key, starts_with("key."))]);
    }

    /// 無効な引数パターンのテスト
    #[rstest]
    #[case::template_literal(r"t(`template.${variable}`)")]
    #[case::variable(r"t(someVariable)")]
    #[case::number(r"t(123)")]
    #[case::object(r#"t({ key: "value" })"#)]
    #[case::array(r#"t(["array", "item"])"#)]
    #[case::function_call(r"t(getKey())")]
    #[case::expression(r#"t("prefix" + "suffix")"#)]
    fn test_invalid_first_argument_patterns(
        queries: Vec<Query>,
        js_lang: Language,
        #[case] t_call: &str,
    ) {
        let code = format!(
            r"
            const {{ t }} = useTranslation();
            const message = {t_call};
            "
        );

        let calls = analyze_trans_fn_calls(&code, &js_lang, &queries, ".")
            .unwrap_or_else(|_| panic!("Failed to parse code for test case"));

        // 無効な引数パターンは検出されない
        assert_that!(calls, is_empty()); // 空チェック
    }

    #[rstest]
    fn test_complex_nested_structure(queries: Vec<Query>, js_lang: Language) {
        let code = r#"
            function App() {
                const { t } = useTranslation();

                function Header() {
                    const { t } = useTranslation("common", { keyPrefix: "header" });
                    t("navigation.home");
                    t("navigation.about");
                }

                function Content() {
                    const { t } = useTranslation("pages");
                    t("home.welcome");

                    if (true) {
                        const { t } = useTranslation("pages", { keyPrefix: "home.section" });
                        t("features.title");
                        t("features.description");
                    }

                    t("home.footer");
                }

                t("global.loading");
                Header();
                Content();
                t("global.error");

                return null;
            }
            "#;

        let calls = analyze_trans_fn_calls(code, &js_lang, &queries, ".").unwrap();

        // 実際の解析順序（関数定義が先に解析される）
        assert_that!(
            calls,
            elements_are![
                field!(TransFnCall.key, eq("header.navigation.home")), /* ヘッダースコープ（keyPrefixあり） */
                field!(TransFnCall.key, eq("header.navigation.about")), //
                field!(TransFnCall.key, eq("home.welcome")), // コンテンツスコープ（keyPrefixなし）
                field!(TransFnCall.key, eq("home.section.features.title")), /* ネストしたスコープ（keyPrefixあり） */
                field!(TransFnCall.key, eq("home.section.features.description")),
                field!(TransFnCall.key, eq("home.footer")), // コンテンツスコープに戻る
                field!(TransFnCall.key, eq("global.loading")), // メイン実行部分のグローバルスコープ
                field!(TransFnCall.key, eq("global.error"))
            ]
        );
    }

    // ===== i18next テスト =====

    #[rstest]
    fn test_i18next_get_fixed_t(queries: Vec<Query>, js_lang: Language) {
        let code = r#"
            const t = i18n.getFixedT(null, "common");
            const message = t("hello.world");
        "#;

        let calls = analyze_trans_fn_calls(code, &js_lang, &queries, ".").unwrap();

        assert_that!(calls, elements_are![field!(TransFnCall.key, eq("hello.world"))]);
    }

    #[rstest]
    fn test_i18next_get_fixed_t_with_key_prefix(queries: Vec<Query>, js_lang: Language) {
        let code = r#"
            const t = i18n.getFixedT(null, "common", "buttons");
            const message = t("save");
        "#;

        let calls = analyze_trans_fn_calls(code, &js_lang, &queries, ".").unwrap();

        assert_that!(calls, elements_are![field!(TransFnCall.key, eq("buttons.save"))]);
    }

    // ===== next-intl テスト =====

    #[rstest]
    fn test_next_intl_use_translations(queries: Vec<Query>, js_lang: Language) {
        let code = r#"
            const t = useTranslations("common");
            const message = t("hello");
        "#;

        let calls = analyze_trans_fn_calls(code, &js_lang, &queries, ".").unwrap();

        assert_that!(calls, elements_are![field!(TransFnCall.key, eq("common.hello"))]);
    }

    #[rstest]
    fn test_next_intl_use_translations_without_prefix(queries: Vec<Query>, js_lang: Language) {
        let code = r#"
            const t = useTranslations();
            const message = t("hello");
        "#;

        let calls = analyze_trans_fn_calls(code, &js_lang, &queries, ".").unwrap();

        assert_that!(calls, elements_are![field!(TransFnCall.key, eq("hello"))]);
    }

    #[rstest]
    fn test_next_intl_t_rich(queries: Vec<Query>, js_lang: Language) {
        let code = r#"
            const t = useTranslations("common");
            const message = t.rich("hello", { strong: (chunks) => chunks });
        "#;

        let calls = analyze_trans_fn_calls(code, &js_lang, &queries, ".").unwrap();

        assert_that!(calls, elements_are![field!(TransFnCall.key, eq("common.hello"))]);
    }

    // ===== react-i18next Trans/Translation コンポーネントテスト =====

    #[rstest]
    fn test_trans_component_self_closing(queries: Vec<Query>, js_lang: Language) {
        let code = r#"
            const { t } = useTranslation();
            return <Trans i18nKey="welcome" t={t} />;
        "#;

        let calls = analyze_trans_fn_calls(code, &js_lang, &queries, ".").unwrap();

        assert_that!(calls, elements_are![field!(TransFnCall.key, eq("welcome"))]);
    }

    #[rstest]
    fn test_trans_component_without_t_attr(queries: Vec<Query>, js_lang: Language) {
        let code = r#"
            const { t } = useTranslation();
            return <Trans i18nKey="welcome" />;
        "#;

        let calls = analyze_trans_fn_calls(code, &js_lang, &queries, ".").unwrap();

        // t 属性がなくても、スコープ内の "t" を使用してマッチする
        assert_that!(calls, elements_are![field!(TransFnCall.key, eq("welcome"))]);
    }

    #[rstest]
    fn test_trans_component_opening(queries: Vec<Query>, js_lang: Language) {
        let code = r#"
            const { t } = useTranslation();
            return <Trans i18nKey="greeting" t={t}>Hello <strong>World</strong></Trans>;
        "#;

        let calls = analyze_trans_fn_calls(code, &js_lang, &queries, ".").unwrap();

        assert_that!(calls, elements_are![field!(TransFnCall.key, eq("greeting"))]);
    }

    #[rstest]
    fn test_translation_component(queries: Vec<Query>, js_lang: Language) {
        let code = r#"
            return (
                <Translation keyPrefix="common">
                    {(t) => <p>{t("hello")}</p>}
                </Translation>
            );
        "#;

        let calls = analyze_trans_fn_calls(code, &js_lang, &queries, ".").unwrap();

        assert_that!(calls, elements_are![field!(TransFnCall.key, eq("common.hello"))]);
    }

    #[rstest]
    fn test_translation_component_without_key_prefix(queries: Vec<Query>, js_lang: Language) {
        let code = r#"
            return (
                <Translation>
                    {(t) => <p>{t("hello")}</p>}
                </Translation>
            );
        "#;

        let calls = analyze_trans_fn_calls(code, &js_lang, &queries, ".").unwrap();

        assert_that!(calls, elements_are![field!(TransFnCall.key, eq("hello"))]);
    }
}
