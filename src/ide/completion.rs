//! Completion implementation

use tower_lsp::lsp_types::{
    CompletionItem,
    CompletionItemKind,
    CompletionTextEdit,
    Documentation,
    MarkupContent,
    MarkupKind,
    Position,
    Range,
    TextEdit,
};

use crate::db::I18nDatabase;
use crate::input::source::ProgrammingLanguage;
use crate::input::translation::Translation;
use crate::syntax::analyzer::{
    extractor::analyze_trans_fn_calls,
    query_loader::load_queries,
};

/// クォートのコンテキスト情報
#[derive(Debug, Clone)]
pub enum QuoteContext {
    /// クォートなし - カーソルが引数の開始位置（例: `t(|)`）
    NoQuotes { position: Position },

    /// クォート内 - カーソルがクォート内にある（例: `t("|")` or `t("com|mon")`）
    InsideQuotes {
        /// クォート内のキー開始位置（クォート記号の次の位置）
        key_start: Position,
        /// クォート内のキー終了位置（閉じクォート記号の位置）
        key_end: Position,
        /// 既に入力されたキー部分
        partial_key: String,
    },
}

/// 補完コンテキスト情報
#[derive(Debug, Clone)]
pub struct CompletionContext {
    /// 部分的に入力されたキー（空文字列の可能性あり）
    pub partial_key: String,
    /// クォートのコンテキスト
    pub quote_context: QuoteContext,
    /// Key prefix from useTranslation options
    pub key_prefix: Option<String>,
}

/// Generate completion items for translation keys
///
/// # Arguments
/// * `db` - Salsa database
/// * `translations` - All translation data
/// * `partial_key` - Partial key text at cursor position (e.g., "common." or "")
/// * `quote_context` - Quote context information for proper text editing
/// * `key_prefix` - Key prefix from useTranslation options
///
/// # Returns
/// List of completion items
pub fn generate_completions(
    db: &dyn I18nDatabase,
    translations: &[Translation],
    partial_key: Option<&str>,
    quote_context: &QuoteContext,
    key_prefix: Option<&str>,
) -> Vec<CompletionItem> {
    let mut completion_items = Vec::new();
    let mut key_translations: std::collections::HashMap<String, Vec<(String, String)>> =
        std::collections::HashMap::new();

    // key_prefix + partial_key で検索するフルパターンを構築
    let full_partial = match (key_prefix, partial_key) {
        (Some(prefix), Some(partial)) if !partial.is_empty() => Some(format!("{prefix}.{partial}")),
        (Some(prefix), _) => Some(prefix.to_string()),
        (None, Some(partial)) if !partial.is_empty() => Some(partial.to_string()),
        _ => None,
    };

    // Collect all translations for each key
    for translation in translations {
        let keys = translation.keys(db);
        let language = translation.language(db);

        for (key, value) in keys {
            // key_prefix がある場合、そのプレフィックスで始まるキーのみを候補に
            if let Some(prefix) = key_prefix
                && !key.starts_with(prefix)
            {
                continue;
            }

            // Filter by partial key if provided (部分一致)
            if let Some(ref full) = full_partial
                && !key.contains(full.as_str())
            {
                continue;
            }

            key_translations
                .entry(key.clone())
                .or_default()
                .push((language.clone(), value.to_owned()));
        }
    }

    // Create completion items for each unique key
    for (key, lang_values) in key_translations {
        // Skip if no translations found (should not happen)
        let Some((first_lang, first_value)) = lang_values.first() else {
            continue;
        };

        // key_prefix を除いた挿入用キーを計算
        let insert_key = key_prefix.map_or_else(
            || key.clone(),
            |prefix| {
                key.strip_prefix(prefix)
                    .and_then(|s| s.strip_prefix('.'))
                    .unwrap_or(&key)
                    .to_string()
            },
        );

        // Build documentation with all languages
        let mut doc_lines = Vec::new();
        for (lang, value) in &lang_values {
            doc_lines.push(format!("- **{lang}**: {value}"));
        }
        let documentation_text = doc_lines.join("\n");

        let mut item = CompletionItem {
            label: insert_key.clone(),
            kind: Some(CompletionItemKind::CONSTANT),
            detail: Some(format!("{first_value} ({first_lang})")),
            documentation: Some(Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: documentation_text,
            })),
            ..Default::default()
        };

        // Use textEdit based on quote context
        match quote_context {
            QuoteContext::NoQuotes { position } => {
                // t(|) → insert `"key"` with quotes
                let new_text = format!("\"{insert_key}\"");
                item.text_edit = Some(CompletionTextEdit::Edit(TextEdit {
                    range: Range::new(*position, *position),
                    new_text,
                }));
            }
            QuoteContext::InsideQuotes { key_start, key_end, .. } => {
                // t("|") → replace range with key (no quotes)
                item.text_edit = Some(CompletionTextEdit::Edit(TextEdit {
                    range: Range::new(*key_start, *key_end),
                    new_text: insert_key.clone(),
                }));
            }
        }

        completion_items.push(item);
    }

    // Sort by label for consistent ordering
    completion_items.sort_by(|a, b| a.label.cmp(&b.label));

    completion_items
}

/// tree-sitter を使用して補完コンテキストを抽出
///
/// リネームされた翻訳関数（例: `const { t: t2 } = useTranslation()`）や
/// 空の引数（例: `t()`）にも対応。
///
/// # Arguments
/// * `text` - Full text of the file
/// * `language` - Programming language of the source file
/// * `line` - Line number (0-indexed)
/// * `character` - Character position in line (0-indexed)
///
/// # Returns
/// `CompletionContext` if cursor is inside a translation function call, `None` otherwise
#[must_use]
pub fn extract_completion_context_tree_sitter(
    text: &str,
    language: ProgrammingLanguage,
    line: u32,
    character: u32,
) -> Option<CompletionContext> {
    // Parse source code with tree-sitter
    let tree_sitter_lang = language.tree_sitter_language();
    let queries = load_queries(language);

    let trans_fn_calls =
        analyze_trans_fn_calls(text, &tree_sitter_lang, &queries).unwrap_or_default();

    let cursor_position = Position::new(line, character);

    // Find translation function call that contains the cursor
    for call in &trans_fn_calls {
        let arg_range = call.arg_key_node;

        // Check if cursor is within the argument range
        if !position_in_range(cursor_position, arg_range) {
            continue;
        }

        // Extract the text of the argument to determine quote context
        let lines: Vec<&str> = text.lines().collect();

        // For simplicity, assume single-line argument (multi-line strings are rare)
        if arg_range.start.line != arg_range.end.line {
            continue;
        }

        let arg_start_line = lines.get(arg_range.start.line as usize)?;

        let arg_start_char = arg_range.start.character as usize;
        let arg_end_char = arg_range.end.character as usize;

        if arg_start_char >= arg_start_line.len() || arg_end_char > arg_start_line.len() {
            continue;
        }

        let arg_text = &arg_start_line[arg_start_char..arg_end_char];

        // Determine quote context
        // arg_text is typically something like `"common.hello"`, `'common.hello'`, or `()` for t(|)
        let first_char = arg_text.chars().next()?;

        // Case: t(|) - no quotes yet, arg_text is "()" or similar
        if first_char == '(' {
            // Cursor is inside empty arguments - return NoQuotes context
            #[allow(clippy::cast_possible_truncation)] // ソースファイルの列が42億を超えることはない
            let insert_position = Position::new(line, (arg_start_char + 1) as u32);

            return Some(CompletionContext {
                partial_key: String::new(),
                quote_context: QuoteContext::NoQuotes { position: insert_position },
                key_prefix: call.key_prefix.clone(),
            });
        }

        // Case: t("...") or t('...') - has quotes
        if first_char != '"' && first_char != '\'' {
            continue;
        }

        // Calculate positions relative to the quote
        let key_start_char = arg_start_char + 1; // After opening quote
        let key_end_char = arg_end_char.saturating_sub(1); // Before closing quote

        // Extract partial key
        let cursor_char = character as usize;
        let line_text = lines.get(line as usize)?;

        if cursor_char < key_start_char || cursor_char > arg_end_char {
            continue;
        }

        // Calculate partial key (from start of key to cursor)
        let partial_key = if cursor_char >= key_start_char && cursor_char <= key_end_char {
            &line_text[key_start_char..cursor_char]
        } else {
            ""
        };

        #[allow(clippy::cast_possible_truncation)] // ソースファイルの列が42億を超えることはない
        let key_start = Position::new(line, key_start_char as u32);
        #[allow(clippy::cast_possible_truncation)] // ソースファイルの列が42億を超えることはない
        let key_end = Position::new(line, key_end_char as u32);

        return Some(CompletionContext {
            partial_key: partial_key.to_string(),
            quote_context: QuoteContext::InsideQuotes {
                key_start,
                key_end,
                partial_key: partial_key.to_string(),
            },
            key_prefix: call.key_prefix.clone(),
        });
    }

    None
}

/// Check if a position is within a range
const fn position_in_range(position: Position, range: Range) -> bool {
    // Before range start
    if position.line < range.start.line {
        return false;
    }
    if position.line == range.start.line && position.character < range.start.character {
        return false;
    }

    // After range end
    if position.line > range.end.line {
        return false;
    }
    if position.line == range.end.line && position.character > range.end.character {
        return false;
    }

    true
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::expect_used, clippy::panic)]
mod tests {
    use std::collections::HashMap;

    use googletest::prelude::*;
    use rstest::*;

    use super::*;
    use crate::db::I18nDatabaseImpl;

    #[rstest]
    fn generate_completions_all_keys() {
        let db = I18nDatabaseImpl::default();

        // Create test translations
        let en_translation = Translation::new(
            &db,
            "en".to_string(),
            "/test/en.json".to_string(),
            HashMap::from([
                ("common.hello".to_string(), "Hello".to_string()),
                ("common.goodbye".to_string(), "Goodbye".to_string()),
                ("errors.notFound".to_string(), "Not Found".to_string()),
            ]),
            "{}".to_string(),
            HashMap::new(),
            HashMap::new(),
        );

        let translations = vec![en_translation];
        let quote_context = QuoteContext::InsideQuotes {
            key_start: Position::new(0, 0),
            key_end: Position::new(0, 0),
            partial_key: String::new(),
        };
        let items = generate_completions(&db, &translations, None, &quote_context, None);

        assert_that!(items.len(), eq(3));
        assert_that!(items[0].label, eq("common.goodbye"));
        assert_that!(items[1].label, eq("common.hello"));
        assert_that!(items[2].label, eq("errors.notFound"));
    }

    #[rstest]
    fn generate_completions_with_partial_key() {
        let db = I18nDatabaseImpl::default();

        let en_translation = Translation::new(
            &db,
            "en".to_string(),
            "/test/en.json".to_string(),
            HashMap::from([
                ("common.hello".to_string(), "Hello".to_string()),
                ("common.goodbye".to_string(), "Goodbye".to_string()),
                ("errors.notFound".to_string(), "Not Found".to_string()),
            ]),
            "{}".to_string(),
            HashMap::new(),
            HashMap::new(),
        );

        let translations = vec![en_translation];
        let quote_context = QuoteContext::InsideQuotes {
            key_start: Position::new(0, 0),
            key_end: Position::new(0, 0),
            partial_key: "common.".to_string(),
        };
        let items = generate_completions(&db, &translations, Some("common."), &quote_context, None);

        assert_that!(items.len(), eq(2));
        assert_that!(items[0].label, eq("common.goodbye"));
        assert_that!(items[1].label, eq("common.hello"));
    }

    #[rstest]
    fn generate_completions_multiple_languages() {
        let db = I18nDatabaseImpl::default();

        // Same key in multiple languages
        let en_translation = Translation::new(
            &db,
            "en".to_string(),
            "/test/en.json".to_string(),
            HashMap::from([("common.hello".to_string(), "Hello".to_string())]),
            "{}".to_string(),
            HashMap::new(),
            HashMap::new(),
        );

        let ja_translation = Translation::new(
            &db,
            "ja".to_string(),
            "/test/ja.json".to_string(),
            HashMap::from([("common.hello".to_string(), "Konnichiwa".to_string())]),
            "{}".to_string(),
            HashMap::new(),
            HashMap::new(),
        );

        let translations = vec![en_translation, ja_translation];
        let quote_context = QuoteContext::InsideQuotes {
            key_start: Position::new(0, 0),
            key_end: Position::new(0, 0),
            partial_key: String::new(),
        };
        let items = generate_completions(&db, &translations, None, &quote_context, None);

        // Should have one item with both languages
        assert_that!(items.len(), eq(1));
        assert_that!(items[0].label, eq("common.hello"));

        // Documentation should contain both languages
        if let Some(Documentation::MarkupContent(content)) = &items[0].documentation {
            assert_that!(content.value, contains_substring("en"));
            assert_that!(content.value, contains_substring("ja"));
            assert_that!(content.value, contains_substring("Hello"));
            assert_that!(content.value, contains_substring("Konnichiwa"));
        }
    }

    #[rstest]
    fn generate_completions_no_match() {
        let db = I18nDatabaseImpl::default();

        let en_translation = Translation::new(
            &db,
            "en".to_string(),
            "/test/en.json".to_string(),
            HashMap::from([("common.hello".to_string(), "Hello".to_string())]),
            "{}".to_string(),
            HashMap::new(),
            HashMap::new(),
        );

        let translations = vec![en_translation];
        let quote_context = QuoteContext::InsideQuotes {
            key_start: Position::new(0, 0),
            key_end: Position::new(0, 0),
            partial_key: "nonexistent.".to_string(),
        };
        let items =
            generate_completions(&db, &translations, Some("nonexistent."), &quote_context, None);

        assert_that!(items, is_empty());
    }

    // Tests for tree-sitter based extraction with renamed functions

    #[rstest]
    fn extract_completion_context_tree_sitter_renamed_t2() {
        let text = r#"
const { t: t2 } = useTranslation();
const msg = t2("common.hello");
"#;
        let language = ProgrammingLanguage::JavaScript;

        // Cursor inside "common.hello" at position after "common."
        let result = extract_completion_context_tree_sitter(text, language, 2, 23);

        assert_that!(result.is_some(), eq(true));
        let context = result.unwrap();
        assert_that!(context.partial_key, eq("common."));
    }

    #[rstest]
    fn extract_completion_context_tree_sitter_renamed_translate() {
        let text = r#"
const { t: translate } = useTranslation();
const msg = translate("errors.notFound");
"#;
        let language = ProgrammingLanguage::JavaScript;

        // Line 2: const msg = translate("errors.notFound");
        // Position 22 = " (opening quote)
        // Position 23 = e (key starts)
        // Position 29 = .
        // Position 30 = n (after the dot)
        // Cursor at position 30 should give partial_key = "errors."
        let result = extract_completion_context_tree_sitter(text, language, 2, 30);

        assert_that!(result.is_some(), eq(true));
        let context = result.unwrap();
        assert_that!(context.partial_key, eq("errors."));
    }

    #[rstest]
    fn extract_completion_context_tree_sitter_renamed_at_start() {
        let text = r#"
const { t: myT } = useTranslation();
const msg = myT("hello");
"#;
        let language = ProgrammingLanguage::JavaScript;

        // Line 2: const msg = myT("hello");
        // Position 16 = " (opening quote)
        // Position 17 = h (key starts)
        // Cursor at position 17 (right after quote) should give partial_key = ""
        let result = extract_completion_context_tree_sitter(text, language, 2, 17);

        assert_that!(result.is_some(), eq(true));
        let context = result.unwrap();
        assert_that!(context.partial_key, eq(""));
    }

    #[rstest]
    fn extract_completion_context_tree_sitter_scoped_rename() {
        let text = r#"
function Component() {
    const { t: t2 } = useTranslation();
    return t2("scoped.key");
}
"#;
        let language = ProgrammingLanguage::JavaScript;

        // Line 3:     return t2("scoped.key");
        // Position 14 = " (opening quote)
        // Position 15 = s (key starts)
        // Position 21 = .
        // Position 22 = k (after the dot)
        // Cursor at position 22 should give partial_key = "scoped."
        let result = extract_completion_context_tree_sitter(text, language, 3, 22);

        assert_that!(result.is_some(), eq(true));
        let context = result.unwrap();
        assert_that!(context.partial_key, eq("scoped."));
    }

    #[rstest]
    fn extract_completion_context_tree_sitter_not_in_function_call() {
        let text = r#"
const { t: t2 } = useTranslation();
const msg = "not a function call";
"#;
        let language = ProgrammingLanguage::JavaScript;

        // Cursor in regular string (not a translation function)
        let result = extract_completion_context_tree_sitter(text, language, 2, 15);

        assert_that!(result.is_none(), eq(true));
    }

    #[rstest]
    fn extract_completion_context_tree_sitter_tsx_renamed() {
        let text = r#"
const { t: translate } = useTranslation();
return <div>{translate("ui.button.save")}</div>;
"#;
        let language = ProgrammingLanguage::Tsx;

        // Line 2: return <div>{translate("ui.button.save")}</div>;
        // Position 23 = " (opening quote)
        // Position 24 = u
        // Position 26 = .
        // Position 27 = b (after "ui.")
        // Cursor at position 27 should give partial_key = "ui."
        let result = extract_completion_context_tree_sitter(text, language, 2, 27);

        assert_that!(result.is_some(), eq(true));
        let context = result.unwrap();
        assert_that!(context.partial_key, eq("ui."));
    }

    #[rstest]
    fn extract_completion_context_tree_sitter_in_comment_should_not_trigger() {
        let text = r#"
const { t } = useTranslation();
// t("comment.key")
const msg = t("real.key");
"#;
        let language = ProgrammingLanguage::JavaScript;

        // Line 2 is a comment: // t("comment.key")
        // tree-sitter should NOT detect this as a translation call
        let result = extract_completion_context_tree_sitter(text, language, 2, 10);

        // Should be None because it's inside a comment
        assert_that!(result.is_none(), eq(true));
    }

    #[rstest]
    fn extract_completion_context_tree_sitter_empty_string() {
        // Test t("") case - empty string
        let text = r#"
const { t } = useTranslation();
const msg = t("");
"#;
        let language = ProgrammingLanguage::JavaScript;

        // Line 2: const msg = t("");
        // Position 14 = " (opening quote)
        // Position 15 = " (closing quote)
        // Cursor at position 15 (between quotes)
        let result = extract_completion_context_tree_sitter(text, language, 2, 15);

        assert_that!(result.is_some(), eq(true));
        let context = result.unwrap();
        assert_that!(context.partial_key, eq(""));
    }

    #[rstest]
    fn extract_completion_context_tree_sitter_no_quotes() {
        // Test t(|) case - no quotes, cursor inside empty arguments
        let text = r"
const { t } = useTranslation();
const msg = t();
";
        let language = ProgrammingLanguage::JavaScript;

        // Line 2: const msg = t();
        // Position 14 = ( (opening paren)
        // Position 15 = ) (closing paren)
        // Cursor at position 14 (inside empty parentheses)
        let result = extract_completion_context_tree_sitter(text, language, 2, 14);

        assert_that!(result.is_some(), eq(true));
        let context = result.unwrap();
        assert_that!(context.partial_key, eq(""));
        assert_that!(matches!(context.quote_context, QuoteContext::NoQuotes { .. }), eq(true));
    }

    #[rstest]
    fn extract_completion_context_tree_sitter_no_quotes_renamed() {
        // Test t2(|) case - renamed function with no quotes
        let text = r"
const { t: t2 } = useTranslation();
const msg = t2();
";
        let language = ProgrammingLanguage::JavaScript;

        // Line 2: const msg = t2();
        // Cursor inside the empty parentheses
        let result = extract_completion_context_tree_sitter(text, language, 2, 15);

        assert_that!(result.is_some(), eq(true));
        let context = result.unwrap();
        assert_that!(context.partial_key, eq(""));
        assert_that!(matches!(context.quote_context, QuoteContext::NoQuotes { .. }), eq(true));
    }

    #[rstest]
    fn extract_completion_context_tree_sitter_no_quotes_not_trans_fn() {
        // Test foo(|) case - not a translation function, should not trigger
        let text = r"
const msg = foo();
";
        let language = ProgrammingLanguage::JavaScript;

        // Cursor inside the empty parentheses of foo()
        let result = extract_completion_context_tree_sitter(text, language, 1, 16);

        // Should be None because foo is not a translation function
        assert_that!(result.is_none(), eq(true));
    }
}
