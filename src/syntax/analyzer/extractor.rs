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

use crate::framework::FrameworkConfig;
use crate::input::source::ProgrammingLanguage;
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

/// Extracts the function name from a node
///
/// For member expressions like `i18next.t("key")`, this returns "i18next.t".
/// For identifiers, this returns the identifier text.
fn extract_function_name(node: Node<'_>, source_bytes: &[u8]) -> Option<String> {
    match node.kind() {
        "identifier" | "member_expression" => extract_node_text(node, source_bytes),
        _ => None,
    }
}

/// Determines whether a function name represents a translation function.
///
/// Returns true if:
/// - The name is `t` (always allowed — universal convention)
/// - The name is registered in the current scope
/// - The name is a known global function for this framework
/// - The name is a method call on `t` or a scoped function (e.g., `t.rich`, `myT.markup`)
fn is_trans_fn(trans_fn_name: &str, scopes: &Scopes<'_>, config: &FrameworkConfig) -> bool {
    if trans_fn_name == "t" {
        return true;
    }

    if scopes.has_scope(trans_fn_name) {
        return true;
    }

    if config.known_global_trans_fns.contains(&trans_fn_name) {
        return true;
    }

    if let Some((base, method)) = trans_fn_name.split_once('.')
        && config.allowed_trans_fn_methods.contains(&method)
    {
        return base == "t" || scopes.has_scope(base);
    }

    false
}

/// Preprocesses a function name for scope lookup.
///
/// Converts method calls like `t.rich` to their base name `t` for scope resolution.
/// Returns the original name if it's directly registered in scope or is a known global function.
fn preprocess_trans_fn_name_for_scope<'a>(
    trans_fn_name: &'a str,
    scopes: &Scopes<'_>,
    config: &FrameworkConfig,
) -> &'a str {
    if scopes.has_scope(trans_fn_name) {
        return trans_fn_name;
    }

    if let Some((base, method)) = trans_fn_name.split_once('.')
        && config.allowed_trans_fn_methods.contains(&method)
        && (base == "t" || scopes.has_scope(base))
    {
        return base;
    }

    trans_fn_name
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
#[allow(clippy::cast_possible_truncation)] // Source file lines/columns will never exceed 4 billion
fn get_node_range(node: Node<'_>) -> Range {
    let start_pos = node.start_position();
    let end_pos = node.end_position();
    Range::new(
        Position::new(start_pos.row as u32, start_pos.column as u32),
        Position::new(end_pos.row as u32, end_pos.column as u32),
    )
}

/// Collects, sorts, and deduplicates captures from all queries.
///
/// Returns captures sorted by source position with `GetTransFn` before `CallTransFn` at equal positions.
fn collect_and_sort_captures<'a>(
    queries: &[Query],
    root_node: Node<'a>,
    source_bytes: &'a [u8],
) -> Vec<(usize, CaptureName, Node<'a>, usize)> {
    let mut all_captures: Vec<(usize, CaptureName, Node<'a>, usize)> = Vec::new();

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

    // Sort by source position; when positions are equal, process GetTransFn before CallTransFn
    all_captures.sort_by(|a, b| {
        a.3.cmp(&b.3).then_with(|| match (&a.1, &b.1) {
            (CaptureName::GetTransFn, CaptureName::CallTransFn) => std::cmp::Ordering::Less,
            (CaptureName::CallTransFn, CaptureName::GetTransFn) => std::cmp::Ordering::Greater,
            _ => std::cmp::Ordering::Equal,
        })
    });

    // Deduplicate captures at the same position with the same type from the same query.
    // Multiple query patterns (e.g., string-based and selector-based t() calls) can match
    // the same node, producing duplicate CallTransFn entries.
    all_captures.dedup_by(|a, b| a.1 == b.1 && a.3 == b.3 && a.0 == b.0);

    all_captures
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
    programming_language: ProgrammingLanguage,
    queries: &[Query],
    key_separator: &str,
) -> Result<Vec<TransFnCall>, AnalyzerError> {
    let config = FrameworkConfig::for_language(programming_language);
    let mut parser = Parser::new();
    parser.set_language(language).map_err(AnalyzerError::LanguageSetup)?;
    let tree = parser.parse(source, None).ok_or(AnalyzerError::ParseFailed)?;

    let source_bytes = source.as_bytes();

    let mut calls = Vec::new();
    let root_node = tree.root_node();

    let mut scopes = Scopes::new();

    // Add default scope for bare `t` function
    scopes.push_scope("t".to_string(), ScopeInfo::new(root_node, GetTransFnDetail::new("t")));

    let all_captures = collect_and_sort_captures(queries, root_node, source_bytes);

    for (query_idx, capture_name, node, _) in all_captures {
        let Some(query) = queries.get(query_idx) else {
            continue;
        };
        let cap_names = query.capture_names();

        match capture_name {
            CaptureName::GetTransFn => {
                let Ok(trans_fns) =
                    parse_get_trans_fn_captures(query, node, source_bytes, cap_names, config)
                else {
                    continue;
                };

                for trans_fn in trans_fns {
                    cleanup_out_of_scopes(&mut scopes, &trans_fn.trans_fn_name, node);

                    let scope_node = get_closest_node(node, &["statement_block", "jsx_element"])
                        .unwrap_or(root_node);

                    let trans_fn_name = trans_fn.trans_fn_name.clone();
                    scopes.push_scope(trans_fn_name, ScopeInfo::new(scope_node, trans_fn));
                }
            }
            CaptureName::CallTransFn => {
                let Ok(call_trans_fn) = parse_call_trans_fn_captures(
                    query,
                    node,
                    source_bytes,
                    cap_names,
                    key_separator,
                ) else {
                    continue;
                };

                if !is_trans_fn(&call_trans_fn.trans_fn_name, &scopes, config) {
                    continue;
                }

                // Preprocess function name for scope lookup (e.g., t.rich -> t)
                let scope_name = preprocess_trans_fn_name_for_scope(
                    &call_trans_fn.trans_fn_name,
                    &scopes,
                    config,
                );

                cleanup_out_of_scopes(&mut scopes, scope_name, node);

                let current_scope = scopes.current_scope(scope_name);
                let key_prefix = current_scope.and_then(|s| s.trans_fn.key_prefix.clone());

                // Namespace priority:
                // 1. explicit_namespace: t("key", { ns: "common" })
                // 2. scope namespace: useTranslation("ns")
                let namespace = call_trans_fn
                    .explicit_namespace
                    .clone()
                    .or_else(|| current_scope.and_then(|s| s.trans_fn.namespace.clone()));

                let namespaces = current_scope.and_then(|s| s.trans_fn.namespaces.clone());

                let arg_key_node = call_trans_fn.arg_key_node;

                calls.push(TransFnCall {
                    key: key_prefix.as_ref().map_or_else(
                        || call_trans_fn.key.clone(),
                        |prefix| format!("{}{}{}", prefix, key_separator, &call_trans_fn.key),
                    ),
                    arg_key: call_trans_fn.key.clone(),
                    arg_key_node: call_trans_fn
                        .arg_key_range
                        .unwrap_or_else(|| get_node_range(arg_key_node)),
                    key_prefix,
                    namespace,
                    namespaces,
                });
            }
            _ => {}
        }
    }

    Ok(calls)
}

/// Pops scopes that the current node has exited from
fn cleanup_out_of_scopes(scopes: &mut Scopes<'_>, trans_fn_name: &str, current_node: Node<'_>) {
    while scopes.current_scope(trans_fn_name).is_some()
        && !scopes.is_node_in_current_scope(trans_fn_name, current_node)
    {
        scopes.pop_scope(trans_fn_name);
    }
}

/// Parses a namespace from a translation key using the namespace separator.
///
/// # Examples
/// - `parse_key_with_namespace("common:hello", Some(":"))` -> `(Some("common"), "hello")`
/// - `parse_key_with_namespace("hello", Some(":"))` -> `(None, "hello")`
/// - `parse_key_with_namespace("common:hello", None)` -> `(None, "common:hello")`
#[must_use]
pub fn parse_key_with_namespace(
    key: &str,
    namespace_separator: Option<&str>,
) -> (Option<String>, String) {
    namespace_separator.and_then(|sep| key.split_once(sep)).map_or_else(
        || (None, key.to_string()),
        |(ns, key_part)| (Some(ns.to_string()), key_part.to_string()),
    )
}

/// Extracts the parameter name from an arrow function node.
///
/// Handles both `$ => ...` (`parameter` field) and `($) => ...` (`parameters`/`formal_parameters` field).
/// In TypeScript/TSX, parameters are wrapped in `required_parameter` nodes.
fn extract_arrow_param_name(arrow_fn: Node<'_>, source_bytes: &[u8]) -> Option<String> {
    // `$ => ...` — single identifier parameter (JS only)
    if let Some(param) = arrow_fn.child_by_field_name("parameter")
        && param.kind() == "identifier"
    {
        return extract_node_text(param, source_bytes);
    }
    // `($) => ...` — formal_parameters wrapping an identifier or required_parameter
    if let Some(params) = arrow_fn.child_by_field_name("parameters")
        && params.kind() == "formal_parameters"
    {
        for i in 0..params.named_child_count() {
            #[allow(clippy::cast_possible_truncation)]
            if let Some(child) = params.named_child(i as u32) {
                match child.kind() {
                    // JavaScript: (identifier)
                    "identifier" => return extract_node_text(child, source_bytes),
                    // TypeScript/TSX: (required_parameter pattern: (identifier))
                    "required_parameter" => {
                        if let Some(pattern) = child.child_by_field_name("pattern")
                            && pattern.kind() == "identifier"
                        {
                            return extract_node_text(pattern, source_bytes);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    None
}

/// Recursively extracts key segments from a selector expression body.
///
/// Walks the `member_expression` / `subscript_expression` chain and collects property names.
/// Returns `None` if the chain doesn't start with the expected parameter.
fn extract_selector_key_parts(
    node: Node<'_>,
    param_name: &str,
    source_bytes: &[u8],
) -> Option<Vec<String>> {
    match node.kind() {
        "identifier" => {
            // Terminal: should be the parameter (e.g., `$`)
            let name = node.utf8_text(source_bytes).ok()?;
            if name == param_name { Some(Vec::new()) } else { None }
        }
        "member_expression" => {
            let object = node.child_by_field_name("object")?;
            let property = node.child_by_field_name("property")?;
            let prop_text = property.utf8_text(source_bytes).ok()?.to_string();
            let mut parts = extract_selector_key_parts(object, param_name, source_bytes)?;
            parts.push(prop_text);
            Some(parts)
        }
        "subscript_expression" => {
            let object = node.child_by_field_name("object")?;
            let index = node.child_by_field_name("index")?;
            let index_text = match index.kind() {
                "number" => index.utf8_text(source_bytes).ok()?.to_string(),
                "string" => {
                    // Extract the fragment inside quotes
                    index.named_child(0)?.utf8_text(source_bytes).ok()?.to_string()
                }
                _ => return None,
            };
            let mut parts = extract_selector_key_parts(object, param_name, source_bytes)?;
            parts.push(index_text);
            Some(parts)
        }
        _ => None,
    }
}

/// Extends a node's range to include trailing accessor operators (`.` or `[`).
///
/// Tree-sitter excludes trailing accessors from incomplete expressions like `$.common.`,
/// so we scan the source bytes and extend the range to cover them.
#[allow(clippy::cast_possible_truncation)]
fn extend_range_past_accessors(node: Node<'_>, source_bytes: &[u8]) -> Range {
    let mut range = get_node_range(node);
    let mut byte = node.end_byte();
    while source_bytes.get(byte).is_some_and(|&b| matches!(b, b'.' | b'[')) {
        byte += 1;
    }
    if byte > node.end_byte() {
        range.end.character += (byte - node.end_byte()) as u32;
    }
    range
}

/// Extracts a translation key from a selector arrow function node.
///
/// Parses `$ => $.a.b.c` into the key `"a.b.c"` using the given key separator.
fn extract_selector_key(
    arrow_fn: Node<'_>,
    source_bytes: &[u8],
    key_separator: &str,
) -> Option<String> {
    let param_name = extract_arrow_param_name(arrow_fn, source_bytes)?;
    let body = arrow_fn.child_by_field_name("body")?;
    let parts = extract_selector_key_parts(body, &param_name, source_bytes)?;
    Some(parts.join(key_separator))
}

/// Extracts string arguments from an arguments node
///
/// Returns a vector of string values from the arguments, in order.
/// Only string literals are extracted; other argument types are represented as None.
fn extract_string_arguments(args_node: Node<'_>, source_bytes: &[u8]) -> Vec<Option<String>> {
    let mut strings = Vec::new();

    for i in 0..args_node.named_child_count() {
        #[allow(clippy::cast_possible_truncation)] // Argument count will never exceed 4 billion
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
    config: &FrameworkConfig,
) -> Result<Vec<GetTransFnDetail>, AnalyzerError> {
    let mut trans_fn_names: Vec<String> = Vec::new();
    let mut namespace = None;
    let mut namespace_items: Vec<String> = Vec::new();
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
                    if let Some(name) = extract_node_text(capture.node, source_bytes)
                        && !trans_fn_names.contains(&name)
                    {
                        trans_fn_names.push(name);
                    }
                }
                CaptureName::Namespace => {
                    namespace = extract_node_text(capture.node, source_bytes);
                }
                CaptureName::NamespaceItem => {
                    if let Some(ns_item) = extract_node_text(capture.node, source_bytes) {
                        namespace_items.push(ns_item);
                    }
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

    // Delegate library-specific argument parsing to the framework config
    if let (Some(args), Some(func)) = (args_node, &func_name) {
        let string_args = extract_string_arguments(args, source_bytes);

        if let Some(parsed) = config.parse_get_trans_fn_args(func, &string_args) {
            if parsed.namespace.is_some() {
                namespace = parsed.namespace;
            }
            if parsed.key_prefix.is_some() {
                key_prefix = parsed.key_prefix;
            }
        }
    }

    if trans_fn_names.is_empty() {
        return Err(AnalyzerError::ParseFailed);
    }

    let namespaces = if namespace_items.is_empty() { None } else { Some(namespace_items) };

    Ok(trans_fn_names
        .into_iter()
        .map(|name| GetTransFnDetail {
            trans_fn_name: name,
            namespace: namespace.clone(),
            namespaces: namespaces.clone(),
            key_prefix: key_prefix.clone(),
        })
        .collect())
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
    key_separator: &str,
) -> Result<CallTransFnDetail<'a>, AnalyzerError> {
    let mut trans_fn_name: Option<String> = None;
    let mut key: Option<String> = None;
    let mut key_node: Option<Node<'a>> = None;
    let mut key_arg_node: Option<Node<'a>> = None;
    let mut trans_args_node: Option<Node<'a>> = None;
    let mut explicit_namespace: Option<String> = None;
    let mut selector_fn_node: Option<Node<'a>> = None;

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(query, capture_node, source_bytes);

    while let Some(match_) = matches.next_mut() {
        for capture in match_.captures {
            let Some(cap_name) = cap_names.get(capture.index as usize) else {
                continue;
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
                    // Extract function name (e.g., t, t.rich, i18next.t)
                    trans_fn_name = extract_function_name(capture.node, source_bytes);
                }
                CaptureName::TransArgs => {
                    trans_args_node = Some(capture.node);
                }
                CaptureName::ExplicitNamespace => {
                    // The ns value from t("key", { ns: "common" })
                    explicit_namespace = extract_node_text(capture.node, source_bytes);
                }
                CaptureName::SelectorFn => {
                    selector_fn_node = Some(capture.node);
                }
                _ => {}
            }
        }
    }

    // Handle Selector API: t($ => $.a.b.c)
    // Use unwrap_or_default for incomplete selectors (e.g., `$ => $.common.`) to support completion
    if let Some(selector_node) = selector_fn_node {
        let selector_key =
            extract_selector_key(selector_node, source_bytes, key_separator).unwrap_or_default();

        // Use the body node (e.g., `$.common.hello`) for the range instead of the full
        // arrow function, so that go-to-definition and rename target only the key expression.
        // Extend past trailing accessor operators for incomplete expressions like `$.common.`
        let body_node = selector_node.child_by_field_name("body").unwrap_or(selector_node);
        let extended_range = extend_range_past_accessors(body_node, source_bytes);

        return Ok(CallTransFnDetail {
            trans_fn_name: trans_fn_name.unwrap_or_else(|| "t".to_string()),
            key: selector_key,
            key_node: selector_node,
            arg_key_node: selector_node,
            explicit_namespace,
            arg_key_range: Some(extended_range),
        });
    }

    // Determine the argument node: use string argument if available, otherwise check for empty args
    let arg_key_node = if let Some(node) = key_arg_node {
        node
    } else if let Some(args_node) = trans_args_node {
        let args_text =
            args_node.utf8_text(source_bytes).map_err(|_| AnalyzerError::ParseFailed)?;
        let inner = args_text.trim_start_matches('(').trim_end_matches(')').trim();

        if inner.is_empty() {
            args_node
        } else {
            // Non-string arguments like t(someVar) are invalid
            return Err(AnalyzerError::ParseFailed);
        }
    } else {
        return Err(AnalyzerError::ParseFailed);
    };

    Ok(CallTransFnDetail {
        trans_fn_name: trans_fn_name.unwrap_or_else(|| "t".to_string()),
        key: key.unwrap_or_default(),
        key_node: key_node.unwrap_or(arg_key_node),
        arg_key_node,
        explicit_namespace,
        arg_key_range: None,
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

    #[fixture]
    fn js_lang() -> Language {
        tree_sitter_javascript::LANGUAGE.into()
    }

    #[fixture]
    fn tsx_lang() -> Language {
        tree_sitter_typescript::LANGUAGE_TSX.into()
    }

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

    #[fixture]
    fn tsx_queries(tsx_lang: Language) -> Vec<Query> {
        let query_files = [
            ("react-i18next", include_str!("../../../queries/tsx/react-i18next.scm")),
            ("i18next", include_str!("../../../queries/tsx/i18next.scm")),
            ("next-intl", include_str!("../../../queries/tsx/next-intl.scm")),
        ];

        query_files
            .iter()
            .map(|(name, content)| {
                Query::new(&tsx_lang, content)
                    .unwrap_or_else(|e| panic!("Failed to parse {name} query: {e}"))
            })
            .collect()
    }

    #[fixture]
    fn ts_lang() -> Language {
        tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
    }

    #[fixture]
    fn svelte_queries(ts_lang: Language) -> Vec<Query> {
        let query_files = [
            ("react-i18next", include_str!("../../../queries/typescript/react-i18next.scm")),
            ("i18next", include_str!("../../../queries/typescript/i18next.scm")),
            ("next-intl", include_str!("../../../queries/typescript/next-intl.scm")),
            ("svelte-i18n", include_str!("../../../queries/svelte-i18n.scm")),
        ];

        query_files
            .iter()
            .map(|(name, content)| {
                Query::new(&ts_lang, content)
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

        let calls =
            analyze_trans_fn_calls(code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap();

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

        let calls =
            analyze_trans_fn_calls(code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap();

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

        let calls =
            analyze_trans_fn_calls(code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap();

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

        let calls =
            analyze_trans_fn_calls(code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap();

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

        let calls =
            analyze_trans_fn_calls(code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap();

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

        let calls =
            analyze_trans_fn_calls(code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap();

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

        let calls =
            analyze_trans_fn_calls(code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap();

        assert_that!(
            calls,
            elements_are![
                field!(TransFnCall.key, eq("original.key")),
                field!(TransFnCall.key, eq("shadowed.key")),
                field!(TransFnCall.key, eq("original.key2"))
            ]
        );
    }

    // key_prefix tests

    #[rstest]
    fn test_key_prefix_application(queries: Vec<Query>, js_lang: Language) {
        let code = r#"
            const { t } = useTranslation("translation", { keyPrefix: "common" });
            const message = t("button.save");
            "#;

        let calls =
            analyze_trans_fn_calls(code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap();

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

        let calls =
            analyze_trans_fn_calls(code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap();

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

        let calls =
            analyze_trans_fn_calls(code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap();

        assert_that!(
            calls,
            elements_are![all![
                field!(TransFnCall.key, eq("simple.key")),
                field!(TransFnCall.arg_key, eq("simple.key"))
            ]]
        );
    }

    // Edge case tests

    #[rstest]
    fn test_empty_code(queries: Vec<Query>, js_lang: Language) {
        let code = "";

        let calls =
            analyze_trans_fn_calls(code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap();

        assert_that!(calls, is_empty());
    }

    #[rstest]
    fn test_undefined_trans_fn(queries: Vec<Query>, js_lang: Language) {
        let code = r#"
            // Call without explicit useTranslation declaration
            const message = t("undefined.key");
            "#;

        let calls =
            analyze_trans_fn_calls(code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap();

        // Default scope "t" exists, so the call is detected
        assert_that!(calls, elements_are![field!(TransFnCall.key, eq("undefined.key"))]);
    }

    #[rstest]
    fn test_invalid_arguments(queries: Vec<Query>, js_lang: Language) {
        let code = r#"
            const { t } = useTranslation();

            // Valid call
            t("valid.key");

            // Invalid: numeric argument
            t(123);

            // Invalid: variable argument
            const key = "variable.key";
            t(key);

            // Invalid: template string
            t(`template.${key}`);
            "#;

        let calls =
            analyze_trans_fn_calls(code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap();

        // Only string literals are valid
        assert_that!(calls, elements_are![field!(TransFnCall.key, eq("valid.key"))]);
    }

    // Table-driven tests for argument patterns

    #[rstest]
    #[case::double_quotes(r#"t("double.quotes")"#, "double.quotes")]
    #[case::single_quotes(r"t('single.quotes')", "single.quotes")]
    #[case::no_spaces(r#"t("no.spaces")"#, "no.spaces")]
    #[case::spaces_around(r#"t( "spaces.around" )"#, "spaces.around")]
    #[case::multiple_spaces(r#"t(  "multiple.spaces"  )"#, "multiple.spaces")]
    #[case::newlines("t(\n  \"newlines\"\n)", "newlines")]
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

        let calls =
            analyze_trans_fn_calls(&code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap_or_else(|_| panic!("Failed to parse code for test case"));

        assert_that!(
            calls,
            elements_are![all![
                field!(TransFnCall.key, eq(expected_key)),
                field!(TransFnCall.arg_key, eq(expected_key))
            ]]
        );
    }

    #[rstest]
    #[case::with_object(r#"t("key.with.object", { count: 1 })"#)]
    #[case::with_number(r#"t("key.with.number", 42)"#)]
    #[case::with_variable(r#"t("key.with.variable", someVariable)"#)]
    #[case::with_multiple_args(r#"t("key.multiple.args", arg1, arg2, arg3)"#)]
    #[case::with_multiline_object(
        r"t('key.multiline', {
  postProcess: 'interval',
})"
    )]
    #[case::with_single_quotes(r"t('key.single.quotes', { count: 1 })")]
    #[case::with_trailing_comma(r#"t("key.trailing.comma", { count: 1, })"#)]
    #[case::with_string_value(r#"t("key.string.value", { postProcess: "interval" })"#)]
    #[case::with_nested_object(
        r"t('key.nested', {
  interpolation: { escapeValue: false },
  start: formatDate(data.startAt),
  end: formatDate(data.endAt),
  timezone,
})"
    )]
    #[case::with_nested_object_simple(r#"t("key.nested.simple", { nested: { a: 1 } })"#)]
    #[case::with_function_call_value(r#"t("key.func", { value: getData() })"#)]
    #[case::with_shorthand_property(r#"t("key.shorthand", { count, name })"#)]
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

        let calls =
            analyze_trans_fn_calls(&code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap_or_else(|_| panic!("Failed to parse code for test case"));

        assert_that!(calls, elements_are![field!(TransFnCall.key, starts_with("key."))]);
    }

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

        let calls =
            analyze_trans_fn_calls(&code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap_or_else(|_| panic!("Failed to parse code for test case"));

        assert_that!(calls, is_empty());
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

        let calls =
            analyze_trans_fn_calls(code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap();

        // Actual parse order: function definitions are parsed first
        assert_that!(
            calls,
            elements_are![
                field!(TransFnCall.key, eq("header.navigation.home")), /* Header scope (with keyPrefix) */
                field!(TransFnCall.key, eq("header.navigation.about")),
                field!(TransFnCall.key, eq("home.welcome")), // Content scope (no keyPrefix)
                field!(TransFnCall.key, eq("home.section.features.title")), /* Nested scope (with keyPrefix) */
                field!(TransFnCall.key, eq("home.section.features.description")),
                field!(TransFnCall.key, eq("home.footer")), // Back to Content scope
                field!(TransFnCall.key, eq("global.loading")), // Main execution global scope
                field!(TransFnCall.key, eq("global.error"))
            ]
        );
    }

    // ===== i18next tests =====

    #[rstest]
    fn test_i18next_get_fixed_t(queries: Vec<Query>, js_lang: Language) {
        let code = r#"
            const t = i18n.getFixedT(null, "common");
            const message = t("hello.world");
        "#;

        let calls =
            analyze_trans_fn_calls(code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap();

        assert_that!(calls, elements_are![field!(TransFnCall.key, eq("hello.world"))]);
    }

    #[rstest]
    fn test_i18next_get_fixed_t_with_key_prefix(queries: Vec<Query>, js_lang: Language) {
        let code = r#"
            const t = i18n.getFixedT(null, "common", "buttons");
            const message = t("save");
        "#;

        let calls =
            analyze_trans_fn_calls(code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap();

        assert_that!(calls, elements_are![field!(TransFnCall.key, eq("buttons.save"))]);
    }

    // ===== next-intl tests =====

    #[rstest]
    fn test_next_intl_use_translations(queries: Vec<Query>, js_lang: Language) {
        let code = r#"
            const t = useTranslations("common");
            const message = t("hello");
        "#;

        let calls =
            analyze_trans_fn_calls(code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap();

        assert_that!(calls, elements_are![field!(TransFnCall.key, eq("common.hello"))]);
    }

    #[rstest]
    fn test_next_intl_use_translations_without_prefix(queries: Vec<Query>, js_lang: Language) {
        let code = r#"
            const t = useTranslations();
            const message = t("hello");
        "#;

        let calls =
            analyze_trans_fn_calls(code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap();

        assert_that!(calls, elements_are![field!(TransFnCall.key, eq("hello"))]);
    }

    #[rstest]
    fn test_next_intl_t_rich(queries: Vec<Query>, js_lang: Language) {
        let code = r#"
            const t = useTranslations("common");
            const message = t.rich("hello", { strong: (chunks) => chunks });
        "#;

        let calls =
            analyze_trans_fn_calls(code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap();

        assert_that!(calls, elements_are![field!(TransFnCall.key, eq("common.hello"))]);
    }

    // ===== react-i18next Trans/Translation component tests =====

    #[rstest]
    fn test_trans_component_self_closing(queries: Vec<Query>, js_lang: Language) {
        let code = r#"
            const { t } = useTranslation();
            return <Trans i18nKey="welcome" t={t} />;
        "#;

        let calls =
            analyze_trans_fn_calls(code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap();

        assert_that!(calls, elements_are![field!(TransFnCall.key, eq("welcome"))]);
    }

    #[rstest]
    fn test_trans_component_without_t_attr(queries: Vec<Query>, js_lang: Language) {
        let code = r#"
            const { t } = useTranslation();
            return <Trans i18nKey="welcome" />;
        "#;

        let calls =
            analyze_trans_fn_calls(code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap();

        // Even without t attribute, matches using "t" in scope
        assert_that!(calls, elements_are![field!(TransFnCall.key, eq("welcome"))]);
    }

    #[rstest]
    fn test_trans_component_opening(queries: Vec<Query>, js_lang: Language) {
        let code = r#"
            const { t } = useTranslation();
            return <Trans i18nKey="greeting" t={t}>Hello <strong>World</strong></Trans>;
        "#;

        let calls =
            analyze_trans_fn_calls(code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap();

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

        let calls =
            analyze_trans_fn_calls(code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap();

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

        let calls =
            analyze_trans_fn_calls(code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap();

        assert_that!(calls, elements_are![field!(TransFnCall.key, eq("hello"))]);
    }

    // ===== Global translation function tests =====

    #[rstest]
    fn test_i18next_t_global(queries: Vec<Query>, js_lang: Language) {
        // Direct i18next.t call without scope setup
        let code = r#"
            const message = i18next.t("global.key");
        "#;

        let calls =
            analyze_trans_fn_calls(code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap();

        assert_that!(
            calls,
            elements_are![all![
                field!(TransFnCall.key, eq("global.key")),
                field!(TransFnCall.key_prefix, none())
            ]]
        );
    }

    #[rstest]
    fn test_i18n_t_global(queries: Vec<Query>, js_lang: Language) {
        // i18n.t is also allowed
        let code = r#"
            const message = i18n.t("another.key");
        "#;

        let calls =
            analyze_trans_fn_calls(code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap();

        assert_that!(calls, elements_are![field!(TransFnCall.key, eq("another.key"))]);
    }

    #[rstest]
    fn test_bare_t_without_scope(queries: Vec<Query>, js_lang: Language) {
        // Bare t() without scope is allowed (as default translation function)
        let code = r#"
            const message = t("bare.key");
        "#;

        let calls =
            analyze_trans_fn_calls(code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap();

        assert_that!(
            calls,
            elements_are![all![
                field!(TransFnCall.key, eq("bare.key")),
                field!(TransFnCall.key_prefix, none())
            ]]
        );
    }

    #[rstest]
    fn test_t_rich_without_scope(queries: Vec<Query>, js_lang: Language) {
        // Bare t.rich() without scope is also allowed
        let code = r#"
            const message = t.rich("rich.key", { strong: (chunks) => chunks });
        "#;

        let calls =
            analyze_trans_fn_calls(code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap();

        assert_that!(calls, elements_are![field!(TransFnCall.key, eq("rich.key"))]);
    }

    #[rstest]
    fn test_scoped_t_rich(queries: Vec<Query>, js_lang: Language) {
        // Scoped t.rich()
        let code = r#"
            const t = useTranslations("namespace");
            const message = t.rich("scoped.key", { strong: (chunks) => chunks });
        "#;

        let calls =
            analyze_trans_fn_calls(code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap();

        assert_that!(calls, elements_are![field!(TransFnCall.key, eq("namespace.scoped.key"))]);
    }

    #[rstest]
    fn test_unknown_member_expression_ignored(queries: Vec<Query>, js_lang: Language) {
        // Unknown function calls are ignored
        let code = r#"
            const message = foo.bar("ignored.key");
        "#;

        let calls =
            analyze_trans_fn_calls(code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap();

        assert_that!(calls, is_empty());
    }

    // ===== Namespace separator tests =====

    #[rstest]
    #[case("common:hello", Some(":"), Some("common"), "hello")]
    #[case("errors:notFound", Some(":"), Some("errors"), "notFound")]
    #[case("hello", Some(":"), None, "hello")]
    #[case("common:nested:key", Some(":"), Some("common"), "nested:key")]
    #[case("common:hello", None, None, "common:hello")]
    #[case("ns/key", Some("/"), Some("ns"), "key")]
    fn test_parse_key_with_namespace(
        #[case] key: &str,
        #[case] separator: Option<&str>,
        #[case] expected_ns: Option<&str>,
        #[case] expected_key: &str,
    ) {
        let (ns, parsed_key) = parse_key_with_namespace(key, separator);
        assert_that!(ns.as_deref(), eq(expected_ns));
        assert_that!(parsed_key.as_str(), eq(expected_key));
    }

    // ===== Array namespace tests =====

    #[rstest]
    fn test_array_namespace(queries: Vec<Query>, js_lang: Language) {
        let code = r#"
            const { t } = useTranslation(['common', 'errors']);
            const message = t("hello");
        "#;

        let calls =
            analyze_trans_fn_calls(code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap();

        assert_that!(calls, len(eq(1)));
        assert_that!(calls[0].key.as_str(), eq("hello"));
        assert_that!(
            calls[0].namespaces.as_ref().unwrap(),
            eq(&vec!["common".to_string(), "errors".to_string()])
        );
    }

    #[rstest]
    fn test_array_namespace_single_item(queries: Vec<Query>, js_lang: Language) {
        let code = r#"
            const { t } = useTranslation(['common']);
            const message = t("hello");
        "#;

        let calls =
            analyze_trans_fn_calls(code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap();

        assert_that!(calls, len(eq(1)));
        assert_that!(calls[0].key.as_str(), eq("hello"));
        assert_that!(calls[0].namespaces.as_ref().unwrap(), eq(&vec!["common".to_string()]));
    }

    // ===== Explicit namespace (ns option) tests =====

    #[rstest]
    fn test_explicit_namespace_ns_option(queries: Vec<Query>, js_lang: Language) {
        let code = r#"
            const { t } = useTranslation('common');
            const message = t("hello", { ns: "errors" });
        "#;

        let calls =
            analyze_trans_fn_calls(code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap();

        // explicit_namespace overrides the scope namespace
        assert_that!(
            calls,
            elements_are![all![
                field!(TransFnCall.key, eq("hello")),
                field!(TransFnCall.namespace, some(eq("errors")))
            ]]
        );
    }

    #[rstest]
    fn test_explicit_namespace_without_declared_namespace(queries: Vec<Query>, js_lang: Language) {
        let code = r#"
            const { t } = useTranslation();
            const message = t("hello", { ns: "common" });
        "#;

        let calls =
            analyze_trans_fn_calls(code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap();

        assert_that!(
            calls,
            elements_are![all![
                field!(TransFnCall.key, eq("hello")),
                field!(TransFnCall.namespace, some(eq("common")))
            ]]
        );
    }

    #[rstest]
    fn test_explicit_namespace_with_other_options(queries: Vec<Query>, js_lang: Language) {
        let code = r#"
            const { t } = useTranslation('common');
            const message = t("hello", { count: 5, ns: "errors" });
        "#;

        let calls =
            analyze_trans_fn_calls(code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap();

        assert_that!(
            calls,
            elements_are![all![
                field!(TransFnCall.key, eq("hello")),
                field!(TransFnCall.namespace, some(eq("errors")))
            ]]
        );
    }

    #[rstest]
    fn test_tsx_complex_object_options(tsx_queries: Vec<Query>, tsx_lang: Language) {
        let code = r"
function Component() {
    const { t } = useTranslation();
    return (
        <div>
            {t('key.complex', {
                interpolation: { escapeValue: false },
                value: someFunc(a.b),
                shorthand,
            })}
        </div>
    );
}
";

        let calls =
            analyze_trans_fn_calls(code, &tsx_lang, ProgrammingLanguage::Tsx, &tsx_queries, ".")
                .unwrap();

        assert_that!(calls, elements_are![field!(TransFnCall.key, eq("key.complex"))]);
    }

    #[rstest]
    fn test_mixed_patterns_same_file(queries: Vec<Query>, js_lang: Language) {
        let code = r#"
            const { t } = useTranslation('common');
            const msg1 = t("key1");
            const msg2 = t("key2", { count: 1 });
            const msg3 = t("key3", { ns: "errors" });
            const msg4 = t("key4", { count: 5, ns: "other" });
            const msg5 = t("key5", { postProcess: "interval" });
        "#;

        let calls =
            analyze_trans_fn_calls(code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap();

        assert_that!(
            calls,
            elements_are![
                all![
                    field!(TransFnCall.key, eq("key1")),
                    field!(TransFnCall.namespace, some(eq("common")))
                ],
                all![
                    field!(TransFnCall.key, eq("key2")),
                    field!(TransFnCall.namespace, some(eq("common")))
                ],
                all![
                    field!(TransFnCall.key, eq("key3")),
                    field!(TransFnCall.namespace, some(eq("errors")))
                ],
                all![
                    field!(TransFnCall.key, eq("key4")),
                    field!(TransFnCall.namespace, some(eq("other")))
                ],
                all![
                    field!(TransFnCall.key, eq("key5")),
                    field!(TransFnCall.namespace, some(eq("common")))
                ]
            ]
        );
    }

    // --- svelte-i18n: global store functions ---

    #[rstest]
    fn svelte_i18n_dollar_underscore(queries: Vec<Query>, js_lang: Language) {
        let code = r#"$_("common.hello")"#;
        let calls =
            analyze_trans_fn_calls(code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap();

        assert_that!(
            calls,
            elements_are![all![
                field!(TransFnCall.key, eq("common.hello")),
                field!(TransFnCall.namespace, none()),
                field!(TransFnCall.key_prefix, none())
            ]]
        );
    }

    #[rstest]
    fn svelte_i18n_dollar_t(queries: Vec<Query>, js_lang: Language) {
        let code = r#"$t("common.goodbye")"#;
        let calls =
            analyze_trans_fn_calls(code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap();

        assert_that!(calls, elements_are![all![field!(TransFnCall.key, eq("common.goodbye"))]]);
    }

    #[rstest]
    fn svelte_i18n_dollar_format(queries: Vec<Query>, js_lang: Language) {
        let code = r#"$format("format.example")"#;
        let calls =
            analyze_trans_fn_calls(code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap();

        assert_that!(calls, elements_are![all![field!(TransFnCall.key, eq("format.example"))]]);
    }

    #[rstest]
    fn svelte_i18n_dollar_json(queries: Vec<Query>, js_lang: Language) {
        let code = r#"$json("json_data.colors")"#;
        let calls =
            analyze_trans_fn_calls(code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap();

        assert_that!(calls, elements_are![all![field!(TransFnCall.key, eq("json_data.colors"))]]);
    }

    #[rstest]
    fn svelte_i18n_multiple_calls(queries: Vec<Query>, js_lang: Language) {
        let code = r#"
            $_("key1");
            $t("key2");
            $format("key3");
        "#;
        let calls =
            analyze_trans_fn_calls(code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap();

        assert_that!(
            calls,
            elements_are![
                all![field!(TransFnCall.key, eq("key1"))],
                all![field!(TransFnCall.key, eq("key2"))],
                all![field!(TransFnCall.key, eq("key3"))]
            ]
        );
    }

    // --- svelte-i18n: object form ---

    #[rstest]
    fn svelte_i18n_object_form_basic(svelte_queries: Vec<Query>, ts_lang: Language) {
        let code = r#"$_({ id: "common.hello" })"#;
        let calls = analyze_trans_fn_calls(
            code,
            &ts_lang,
            ProgrammingLanguage::Svelte,
            &svelte_queries,
            ".",
        )
        .unwrap();

        assert_that!(calls, elements_are![all![field!(TransFnCall.key, eq("common.hello"))]]);
    }

    #[rstest]
    fn svelte_i18n_object_form_with_values(svelte_queries: Vec<Query>, ts_lang: Language) {
        let code = r#"$_({ id: "common.welcome", values: { name: "Alice" } })"#;
        let calls = analyze_trans_fn_calls(
            code,
            &ts_lang,
            ProgrammingLanguage::Svelte,
            &svelte_queries,
            ".",
        )
        .unwrap();

        assert_that!(calls, elements_are![all![field!(TransFnCall.key, eq("common.welcome"))]]);
    }

    #[rstest]
    fn svelte_i18n_object_form_with_locale(svelte_queries: Vec<Query>, ts_lang: Language) {
        let code = r#"$_({ id: "common.hello", locale: "en" })"#;
        let calls = analyze_trans_fn_calls(
            code,
            &ts_lang,
            ProgrammingLanguage::Svelte,
            &svelte_queries,
            ".",
        )
        .unwrap();

        assert_that!(calls, elements_are![all![field!(TransFnCall.key, eq("common.hello"))]]);
    }

    #[rstest]
    fn svelte_i18n_object_form_dollar_t(svelte_queries: Vec<Query>, ts_lang: Language) {
        let code = r#"$t({ id: "home.title" })"#;
        let calls = analyze_trans_fn_calls(
            code,
            &ts_lang,
            ProgrammingLanguage::Svelte,
            &svelte_queries,
            ".",
        )
        .unwrap();

        assert_that!(calls, elements_are![all![field!(TransFnCall.key, eq("home.title"))]]);
    }

    // --- svelte-i18n: unwrapFunctionStore ---

    #[rstest]
    fn svelte_i18n_unwrap_function_store(svelte_queries: Vec<Query>, ts_lang: Language) {
        let code = r#"
            const $format = unwrapFunctionStore(format);
            $format("some.key");
        "#;
        let calls = analyze_trans_fn_calls(
            code,
            &ts_lang,
            ProgrammingLanguage::Svelte,
            &svelte_queries,
            ".",
        )
        .unwrap();

        assert_that!(calls, elements_are![all![field!(TransFnCall.key, eq("some.key"))]]);
    }

    #[rstest]
    fn svelte_i18n_unwrap_custom_name(svelte_queries: Vec<Query>, ts_lang: Language) {
        let code = r#"
            const translate = unwrapFunctionStore(_);
            translate("hello.world");
        "#;
        let calls = analyze_trans_fn_calls(
            code,
            &ts_lang,
            ProgrammingLanguage::Svelte,
            &svelte_queries,
            ".",
        )
        .unwrap();

        assert_that!(calls, elements_are![all![field!(TransFnCall.key, eq("hello.world"))]]);
    }

    // --- svelte-i18n: defineMessages ---

    #[rstest]
    fn svelte_i18n_define_messages(svelte_queries: Vec<Query>, ts_lang: Language) {
        let code = r#"
            const messages = defineMessages({
                greeting: { id: "greeting" },
                farewell: { id: "farewell" },
            })
        "#;
        let calls = analyze_trans_fn_calls(
            code,
            &ts_lang,
            ProgrammingLanguage::Svelte,
            &svelte_queries,
            ".",
        )
        .unwrap();

        assert_that!(
            calls,
            elements_are![
                all![field!(TransFnCall.key, eq("greeting"))],
                all![field!(TransFnCall.key, eq("farewell"))]
            ]
        );
    }

    // --- Framework isolation tests ---

    #[rstest]
    fn tsx_does_not_recognize_svelte_globals(tsx_queries: Vec<Query>, tsx_lang: Language) {
        let code = r#"$_("key")"#;
        let calls =
            analyze_trans_fn_calls(code, &tsx_lang, ProgrammingLanguage::Tsx, &tsx_queries, ".")
                .unwrap();
        assert_that!(calls, is_empty());
    }

    #[rstest]
    fn svelte_does_not_recognize_i18next_globals(svelte_queries: Vec<Query>, ts_lang: Language) {
        // i18next.t is a member expression; Svelte's FrameworkConfig excludes it
        let code = r#"i18next.t("key")"#;
        let calls = analyze_trans_fn_calls(
            code,
            &ts_lang,
            ProgrammingLanguage::Svelte,
            &svelte_queries,
            ".",
        )
        .unwrap();
        assert_that!(calls, is_empty());
    }

    #[rstest]
    fn svelte_does_not_recognize_method_calls(svelte_queries: Vec<Query>, ts_lang: Language) {
        // t.rich is allowed for i18next/next-intl but not for Svelte
        let code = r#"
            const { t } = useTranslation();
            t.rich("key");
        "#;
        let calls = analyze_trans_fn_calls(
            code,
            &ts_lang,
            ProgrammingLanguage::Svelte,
            &svelte_queries,
            ".",
        )
        .unwrap();
        assert_that!(calls, is_empty());
    }

    // ===== Selector API tests =====

    #[rstest]
    fn test_selector_api_incomplete_trailing_dot(queries: Vec<Query>, js_lang: Language) {
        // Incomplete selector: trailing dot (mid-typing scenario for completion)
        let code = r"
            const { t } = useTranslation();
            const msg = t($ => $.common.);
        ";

        let calls =
            analyze_trans_fn_calls(code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap();

        // Should produce a TransFnCall (possibly with partial key) for completion support
        assert_that!(calls.len(), ge(1));
    }

    #[rstest]
    fn test_selector_api_basic(queries: Vec<Query>, js_lang: Language) {
        let code = r"
            const { t } = useTranslation();
            const message = t($ => $.my.nested.key);
        ";

        let calls =
            analyze_trans_fn_calls(code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap();

        assert_that!(
            calls,
            elements_are![all![
                field!(TransFnCall.key, eq("my.nested.key")),
                field!(TransFnCall.arg_key, eq("my.nested.key"))
            ]]
        );
    }

    #[rstest]
    fn test_selector_api_with_parens(queries: Vec<Query>, js_lang: Language) {
        let code = r"
            const { t } = useTranslation();
            const message = t(($) => $.my.key);
        ";

        let calls =
            analyze_trans_fn_calls(code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap();

        assert_that!(calls, elements_are![field!(TransFnCall.key, eq("my.key"))]);
    }

    #[rstest]
    fn test_selector_api_single_key(queries: Vec<Query>, js_lang: Language) {
        let code = r"
            const { t } = useTranslation();
            const message = t($ => $.hello);
        ";

        let calls =
            analyze_trans_fn_calls(code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap();

        assert_that!(calls, elements_are![field!(TransFnCall.key, eq("hello"))]);
    }

    #[rstest]
    fn test_selector_api_with_namespace(queries: Vec<Query>, js_lang: Language) {
        let code = r#"
            const { t } = useTranslation("common");
            const message = t($ => $.greeting);
        "#;

        let calls =
            analyze_trans_fn_calls(code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap();

        assert_that!(
            calls,
            elements_are![all![
                field!(TransFnCall.key, eq("greeting")),
                field!(TransFnCall.namespace, some(eq("common")))
            ]]
        );
    }

    #[rstest]
    fn test_selector_api_with_explicit_namespace(queries: Vec<Query>, js_lang: Language) {
        let code = r#"
            const { t } = useTranslation("common");
            const message = t($ => $.hello, { ns: "errors" });
        "#;

        let calls =
            analyze_trans_fn_calls(code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap();

        assert_that!(
            calls,
            elements_are![all![
                field!(TransFnCall.key, eq("hello")),
                field!(TransFnCall.namespace, some(eq("errors")))
            ]]
        );
    }

    #[rstest]
    fn test_selector_api_with_key_prefix(queries: Vec<Query>, js_lang: Language) {
        let code = r#"
            const { t } = useTranslation("translation", { keyPrefix: "user.settings" });
            const message = t($ => $.title);
        "#;

        let calls =
            analyze_trans_fn_calls(code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap();

        assert_that!(
            calls,
            elements_are![all![
                field!(TransFnCall.key, eq("user.settings.title")),
                field!(TransFnCall.arg_key, eq("title"))
            ]]
        );
    }

    #[rstest]
    fn test_selector_api_with_interpolation(queries: Vec<Query>, js_lang: Language) {
        let code = r#"
            const { t } = useTranslation();
            const message = t($ => $.greeting, { name: "World" });
        "#;

        let calls =
            analyze_trans_fn_calls(code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap();

        assert_that!(calls, elements_are![field!(TransFnCall.key, eq("greeting"))]);
    }

    #[rstest]
    fn test_selector_api_subscript_expression(queries: Vec<Query>, js_lang: Language) {
        let code = r"
            const { t } = useTranslation();
            const message = t($ => $[0][1][2]);
        ";

        let calls =
            analyze_trans_fn_calls(code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap();

        assert_that!(calls, elements_are![field!(TransFnCall.key, eq("0.1.2"))]);
    }

    #[rstest]
    fn test_selector_api_mixed_with_string(queries: Vec<Query>, js_lang: Language) {
        let code = r#"
            const { t } = useTranslation();
            const msg1 = t("string.key");
            const msg2 = t($ => $.selector.key);
        "#;

        let calls =
            analyze_trans_fn_calls(code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap();

        assert_that!(
            calls,
            elements_are![
                field!(TransFnCall.key, eq("string.key")),
                field!(TransFnCall.key, eq("selector.key"))
            ]
        );
    }

    #[rstest]
    fn test_selector_api_global_t(queries: Vec<Query>, js_lang: Language) {
        let code = r"
            const message = i18next.t($ => $.global.key);
        ";

        let calls =
            analyze_trans_fn_calls(code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap();

        assert_that!(calls, elements_are![field!(TransFnCall.key, eq("global.key"))]);
    }

    #[rstest]
    fn test_selector_api_get_fixed_t(queries: Vec<Query>, js_lang: Language) {
        let code = r#"
            const t = i18n.getFixedT(null, "common", "buttons");
            const message = t($ => $.save);
        "#;

        let calls =
            analyze_trans_fn_calls(code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap();

        assert_that!(
            calls,
            elements_are![all![
                field!(TransFnCall.key, eq("buttons.save")),
                field!(TransFnCall.arg_key, eq("save"))
            ]]
        );
    }

    #[rstest]
    fn test_selector_api_trans_component(tsx_queries: Vec<Query>, tsx_lang: Language) {
        let code = r"
            const { t } = useTranslation();
            return <Trans i18nKey={($) => $.foo.bar} />;
        ";

        let calls =
            analyze_trans_fn_calls(code, &tsx_lang, ProgrammingLanguage::Tsx, &tsx_queries, ".")
                .unwrap();

        assert_that!(calls, elements_are![field!(TransFnCall.key, eq("foo.bar"))]);
    }

    #[rstest]
    fn test_selector_api_subscript_string_index(queries: Vec<Query>, js_lang: Language) {
        // $["key"] subscript with string index
        let code = r#"
            const { t } = useTranslation();
            const msg = t($ => $["hello"]);
        "#;

        let calls =
            analyze_trans_fn_calls(code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap();

        assert_that!(calls, elements_are![field!(TransFnCall.key, eq("hello"))]);
    }

    #[rstest]
    fn test_selector_api_subscript_unknown_index(queries: Vec<Query>, js_lang: Language) {
        // $[variable] — unsupported subscript type, should produce a call with partial key
        let code = r"
            const { t } = useTranslation();
            const msg = t($ => $[someVar]);
        ";

        let calls =
            analyze_trans_fn_calls(code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap();

        // extract_selector_key fails → unwrap_or_default → empty key
        assert_that!(calls, elements_are![field!(TransFnCall.key, eq(""))]);
    }

    #[rstest]
    fn test_selector_api_tsx_parens(tsx_queries: Vec<Query>, tsx_lang: Language) {
        // TSX with ($) — exercises required_parameter path in extract_arrow_param_name
        let code = r"
            const { t } = useTranslation();
            const msg = t(($) => $.nested.key);
        ";

        let calls =
            analyze_trans_fn_calls(code, &tsx_lang, ProgrammingLanguage::Tsx, &tsx_queries, ".")
                .unwrap();

        assert_that!(calls, elements_are![field!(TransFnCall.key, eq("nested.key"))]);
    }

    #[rstest]
    fn test_selector_api_custom_key_separator(queries: Vec<Query>, js_lang: Language) {
        // Non-dot key separator: member expression `.` is joined with `_`
        let code = r"
            const { t } = useTranslation();
            const msg = t($ => $.common.hello);
        ";

        let calls =
            analyze_trans_fn_calls(code, &js_lang, ProgrammingLanguage::JavaScript, &queries, "_")
                .unwrap();

        assert_that!(calls, elements_are![field!(TransFnCall.key, eq("common_hello"))]);
    }

    #[rstest]
    fn test_selector_api_param_only(queries: Vec<Query>, js_lang: Language) {
        // `$ => $` — just the parameter, no property access
        let code = r"
            const { t } = useTranslation();
            const msg = t($ => $);
        ";

        let calls =
            analyze_trans_fn_calls(code, &js_lang, ProgrammingLanguage::JavaScript, &queries, ".")
                .unwrap();

        // Empty key (root)
        assert_that!(calls, elements_are![field!(TransFnCall.key, eq(""))]);
    }
}
