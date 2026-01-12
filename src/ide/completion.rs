//! Completion implementation

use std::collections::HashMap;

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
use crate::types::{
    SourcePosition,
    SourceRange,
};

/// Quote context for completion
#[derive(Debug, Clone)]
pub enum QuoteContext {
    /// No quotes - cursor at argument start (e.g., `t(|)`)
    NoQuotes { position: Position },

    /// Inside quotes (e.g., `t("|")` or `t("com|mon")`)
    InsideQuotes { key_start: Position, key_end: Position, partial_key: String },
}

#[derive(Debug, Clone)]
pub struct CompletionContext {
    pub partial_key: String,
    pub quote_context: QuoteContext,
    pub key_prefix: Option<String>,
}

/// Generates completion items for translation keys.
pub fn generate_completions(
    db: &dyn I18nDatabase,
    translations: &[Translation],
    partial_key: Option<&str>,
    quote_context: &QuoteContext,
    key_prefix: Option<&str>,
    effective_language: Option<&str>,
    key_separator: &str,
) -> Vec<CompletionItem> {
    let mut completion_items = Vec::new();
    let mut key_translations: HashMap<String, Vec<(String, String)>> = HashMap::new();

    let full_partial = match (key_prefix, partial_key) {
        (Some(prefix), Some(partial)) if !partial.is_empty() => {
            Some(format!("{prefix}{key_separator}{partial}"))
        }
        (Some(prefix), _) => Some(prefix.to_string()),
        (None, Some(partial)) if !partial.is_empty() => Some(partial.to_string()),
        _ => None,
    };

    // Collect all translations for each key
    for translation in translations {
        let keys = translation.keys(db);
        let language = translation.language(db);

        for (key, value) in keys {
            if let Some(prefix) = key_prefix
                && !key.starts_with(prefix)
            {
                continue;
            }

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

    for (key, lang_values) in key_translations {
        if lang_values.is_empty() {
            continue;
        }

        // Remove key_prefix from insert key
        let insert_key = key_prefix.map_or_else(
            || key.clone(),
            |prefix| {
                key.strip_prefix(prefix)
                    .and_then(|s| s.strip_prefix(key_separator))
                    .unwrap_or(&key)
                    .to_string()
            },
        );

        let mut doc_lines = Vec::new();
        for (lang, value) in &lang_values {
            doc_lines.push(format!("- **{lang}**: {value}"));
        }
        let documentation_text = doc_lines.join("\n");

        let detail = effective_language.and_then(|eff_lang| {
            lang_values.iter().find(|(lang, _)| lang == eff_lang).map(|(_, value)| value.clone())
        });

        let mut item = CompletionItem {
            label: insert_key.clone(),
            kind: Some(CompletionItemKind::CONSTANT),
            detail,
            documentation: Some(Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: documentation_text,
            })),
            ..Default::default()
        };

        match quote_context {
            QuoteContext::NoQuotes { position } => {
                let new_text = format!("\"{insert_key}\"");
                item.text_edit = Some(CompletionTextEdit::Edit(TextEdit {
                    range: Range::new(*position, *position),
                    new_text,
                }));
            }
            QuoteContext::InsideQuotes { key_start, key_end, .. } => {
                item.text_edit = Some(CompletionTextEdit::Edit(TextEdit {
                    range: Range::new(*key_start, *key_end),
                    new_text: insert_key.clone(),
                }));
            }
        }

        completion_items.push(item);
    }

    completion_items.sort_by(|a, b| a.label.cmp(&b.label));

    completion_items
}

/// Extracts completion context using tree-sitter.
///
/// Supports renamed translation functions (e.g., `const { t: t2 } = useTranslation()`)
/// and empty arguments (e.g., `t()`).
#[must_use]
pub fn extract_completion_context_tree_sitter(
    text: &str,
    language: ProgrammingLanguage,
    line: u32,
    character: u32,
    key_separator: &str,
) -> Option<CompletionContext> {
    let tree_sitter_lang = language.tree_sitter_language();
    let queries = load_queries(language);

    let trans_fn_calls =
        analyze_trans_fn_calls(text, &tree_sitter_lang, queries, key_separator).unwrap_or_default();

    let cursor_position = Position::new(line, character);

    for call in &trans_fn_calls {
        let arg_range = call.arg_key_node;

        if !SourceRange::from(arg_range).contains(SourcePosition::from(cursor_position)) {
            continue;
        }

        let lines: Vec<&str> = text.lines().collect();

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

        let first_char = arg_text.chars().next()?;

        // t(|) - no quotes
        if first_char == '(' {
            #[allow(clippy::cast_possible_truncation)] // Column count won't exceed u32::MAX
            let insert_position = Position::new(line, (arg_start_char + 1) as u32);

            return Some(CompletionContext {
                partial_key: String::new(),
                quote_context: QuoteContext::NoQuotes { position: insert_position },
                key_prefix: call.key_prefix.clone(),
            });
        }

        // t("...") or t('...')
        if first_char != '"' && first_char != '\'' {
            continue;
        }

        let key_start_char = arg_start_char + 1;
        let key_end_char = arg_end_char.saturating_sub(1);

        let cursor_char = character as usize;
        let line_text = lines.get(line as usize)?;

        if cursor_char < key_start_char || cursor_char > arg_end_char {
            continue;
        }

        let partial_key = if cursor_char >= key_start_char && cursor_char <= key_end_char {
            &line_text[key_start_char..cursor_char]
        } else {
            ""
        };

        #[allow(clippy::cast_possible_truncation)] // Column count won't exceed u32::MAX
        let key_start = Position::new(line, key_start_char as u32);
        #[allow(clippy::cast_possible_truncation)] // Column count won't exceed u32::MAX
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
            None,
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
        let items = generate_completions(&db, &translations, None, &quote_context, None, None, ".");

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
            None,
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
        let items = generate_completions(
            &db,
            &translations,
            Some("common."),
            &quote_context,
            None,
            None,
            ".",
        );

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
            None,
            "/test/en.json".to_string(),
            HashMap::from([("common.hello".to_string(), "Hello".to_string())]),
            "{}".to_string(),
            HashMap::new(),
            HashMap::new(),
        );

        let ja_translation = Translation::new(
            &db,
            "ja".to_string(),
            None,
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
        let items = generate_completions(&db, &translations, None, &quote_context, None, None, ".");

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
            None,
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
        let items = generate_completions(
            &db,
            &translations,
            Some("nonexistent."),
            &quote_context,
            None,
            None,
            ".",
        );

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
        let result = extract_completion_context_tree_sitter(text, language, 2, 23, ".");

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
        let result = extract_completion_context_tree_sitter(text, language, 2, 30, ".");

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
        let result = extract_completion_context_tree_sitter(text, language, 2, 17, ".");

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
        let result = extract_completion_context_tree_sitter(text, language, 3, 22, ".");

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
        let result = extract_completion_context_tree_sitter(text, language, 2, 15, ".");

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
        let result = extract_completion_context_tree_sitter(text, language, 2, 27, ".");

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
        let result = extract_completion_context_tree_sitter(text, language, 2, 10, ".");

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
        let result = extract_completion_context_tree_sitter(text, language, 2, 15, ".");

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
        let result = extract_completion_context_tree_sitter(text, language, 2, 14, ".");

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
        let result = extract_completion_context_tree_sitter(text, language, 2, 15, ".");

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
        let result = extract_completion_context_tree_sitter(text, language, 1, 16, ".");

        // Should be None because foo is not a translation function
        assert_that!(result.is_none(), eq(true));
    }

    #[rstest]
    fn generate_completions_with_key_prefix_only() {
        let db = I18nDatabaseImpl::default();
        let en_translation = Translation::new(
            &db,
            "en".to_string(),
            None,
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

        let items = generate_completions(
            &db,
            &translations,
            None,
            &quote_context,
            Some("common"),
            None,
            ".",
        );

        assert_eq!(items.len(), 2);
        assert!(items.iter().any(|i| i.label == "hello"));
        assert!(items.iter().any(|i| i.label == "goodbye"));
    }

    #[rstest]
    fn generate_completions_with_key_prefix_and_partial() {
        let db = I18nDatabaseImpl::default();
        let en_translation = Translation::new(
            &db,
            "en".to_string(),
            None,
            "/test/en.json".to_string(),
            HashMap::from([
                ("common.hello".to_string(), "Hello".to_string()),
                ("common.help".to_string(), "Help".to_string()),
                ("common.goodbye".to_string(), "Goodbye".to_string()),
            ]),
            "{}".to_string(),
            HashMap::new(),
            HashMap::new(),
        );

        let translations = vec![en_translation];
        let quote_context = QuoteContext::InsideQuotes {
            key_start: Position::new(0, 0),
            key_end: Position::new(0, 0),
            partial_key: "hel".to_string(),
        };

        let items = generate_completions(
            &db,
            &translations,
            Some("hel"),
            &quote_context,
            Some("common"),
            None,
            ".",
        );

        assert_eq!(items.len(), 2);
    }

    #[rstest]
    fn generate_completions_key_prefix_filters_out_non_matching() {
        let db = I18nDatabaseImpl::default();
        let en_translation = Translation::new(
            &db,
            "en".to_string(),
            None,
            "/test/en.json".to_string(),
            HashMap::from([
                ("common.hello".to_string(), "Hello".to_string()),
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

        let items = generate_completions(
            &db,
            &translations,
            None,
            &quote_context,
            Some("errors"),
            None,
            ".",
        );

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].label, "notFound");
    }

    #[rstest]
    fn generate_completions_with_effective_language() {
        let db = I18nDatabaseImpl::default();
        let en_translation = Translation::new(
            &db,
            "en".to_string(),
            None,
            "/test/en.json".to_string(),
            HashMap::from([("hello".to_string(), "Hello".to_string())]),
            "{}".to_string(),
            HashMap::new(),
            HashMap::new(),
        );
        let ja_translation = Translation::new(
            &db,
            "ja".to_string(),
            None,
            "/test/ja.json".to_string(),
            HashMap::from([("hello".to_string(), "こんにちは".to_string())]),
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

        let items =
            generate_completions(&db, &translations, None, &quote_context, None, Some("ja"), ".");

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].detail, Some("こんにちは".to_string()));
    }

    #[rstest]
    fn generate_completions_effective_language_not_found() {
        let db = I18nDatabaseImpl::default();
        let en_translation = Translation::new(
            &db,
            "en".to_string(),
            None,
            "/test/en.json".to_string(),
            HashMap::from([("hello".to_string(), "Hello".to_string())]),
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

        let items =
            generate_completions(&db, &translations, None, &quote_context, None, Some("fr"), ".");

        assert_eq!(items.len(), 1);
        assert!(items[0].detail.is_none());
    }

    #[rstest]
    fn generate_completions_item_fields() {
        let db = I18nDatabaseImpl::default();
        let en_translation = Translation::new(
            &db,
            "en".to_string(),
            None,
            "/test/en.json".to_string(),
            HashMap::from([("hello".to_string(), "Hello World".to_string())]),
            "{}".to_string(),
            HashMap::new(),
            HashMap::new(),
        );

        let translations = vec![en_translation];
        let quote_context = QuoteContext::InsideQuotes {
            key_start: Position::new(0, 5),
            key_end: Position::new(0, 10),
            partial_key: String::new(),
        };

        let items =
            generate_completions(&db, &translations, None, &quote_context, None, Some("en"), ".");

        assert_eq!(items.len(), 1);
        let item = &items[0];

        assert_eq!(item.kind, Some(CompletionItemKind::CONSTANT));
        assert_eq!(item.detail, Some("Hello World".to_string()));
        assert!(matches!(
            &item.documentation,
            Some(Documentation::MarkupContent(c)) if c.kind == MarkupKind::Markdown
        ));
        assert!(item.text_edit.is_some());
    }

    #[rstest]
    fn generate_completions_no_quotes_text_edit() {
        let db = I18nDatabaseImpl::default();
        let en_translation = Translation::new(
            &db,
            "en".to_string(),
            None,
            "/test/en.json".to_string(),
            HashMap::from([("hello".to_string(), "Hello".to_string())]),
            "{}".to_string(),
            HashMap::new(),
            HashMap::new(),
        );

        let translations = vec![en_translation];
        let position = Position::new(1, 5);
        let quote_context = QuoteContext::NoQuotes { position };

        let items = generate_completions(&db, &translations, None, &quote_context, None, None, ".");

        assert_eq!(items.len(), 1);

        // NoQuotes inserts with quotes ("hello")
        if let Some(CompletionTextEdit::Edit(edit)) = &items[0].text_edit {
            assert_eq!(edit.new_text, "\"hello\"");
            assert_eq!(edit.range.start, position);
            assert_eq!(edit.range.end, position);
        } else {
            panic!("Expected TextEdit");
        }
    }

    #[rstest]
    fn generate_completions_inside_quotes_text_edit() {
        let db = I18nDatabaseImpl::default();
        let en_translation = Translation::new(
            &db,
            "en".to_string(),
            None,
            "/test/en.json".to_string(),
            HashMap::from([("hello".to_string(), "Hello".to_string())]),
            "{}".to_string(),
            HashMap::new(),
            HashMap::new(),
        );

        let translations = vec![en_translation];
        let key_start = Position::new(1, 5);
        let key_end = Position::new(1, 10);
        let quote_context =
            QuoteContext::InsideQuotes { key_start, key_end, partial_key: "hel".to_string() };

        let items =
            generate_completions(&db, &translations, Some("hel"), &quote_context, None, None, ".");

        assert_eq!(items.len(), 1);

        // InsideQuotes replaces without quotes (hello)
        if let Some(CompletionTextEdit::Edit(edit)) = &items[0].text_edit {
            assert_eq!(edit.new_text, "hello");
            assert_eq!(edit.range.start, key_start);
            assert_eq!(edit.range.end, key_end);
        } else {
            panic!("Expected TextEdit");
        }
    }

    #[rstest]
    fn generate_completions_empty_translations() {
        let db = I18nDatabaseImpl::default();
        let translations: Vec<Translation> = vec![];
        let quote_context = QuoteContext::InsideQuotes {
            key_start: Position::new(0, 0),
            key_end: Position::new(0, 0),
            partial_key: String::new(),
        };

        let items = generate_completions(&db, &translations, None, &quote_context, None, None, ".");

        assert!(items.is_empty());
    }
}
