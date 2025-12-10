//! Completion implementation

use tower_lsp::lsp_types::{
    CompletionItem,
    CompletionItemKind,
    Documentation,
    MarkupContent,
    MarkupKind,
};

use crate::db::I18nDatabase;
use crate::input::translation::Translation;

/// Generate completion items for translation keys
///
/// # Arguments
/// * `db` - Salsa database
/// * `translations` - All translation data
/// * `partial_key` - Partial key text at cursor position (e.g., "common." or "")
///
/// # Returns
/// List of completion items
pub fn generate_completions(
    db: &dyn I18nDatabase,
    translations: &[Translation],
    partial_key: Option<&str>,
) -> Vec<CompletionItem> {
    let mut completion_items = Vec::new();
    let mut seen_keys = std::collections::HashSet::new();

    // Collect all unique keys from all translations
    for translation in translations {
        let keys = translation.keys(db);
        let language = translation.language(db);

        for (key, value) in keys {
            // Skip if we've already seen this key
            if seen_keys.contains(key.as_str()) {
                continue;
            }

            // Filter by partial key if provided
            if let Some(partial) = partial_key
                && !key.starts_with(partial)
            {
                continue;
            }

            seen_keys.insert(key.clone());

            // Create completion item
            let mut item = CompletionItem {
                label: key.clone(),
                kind: Some(CompletionItemKind::CONSTANT),
                detail: Some(format!("{value} ({language})")),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: format!("**{language}**: {value}"),
                })),
                ..Default::default()
            };

            // If we have a partial key, set insert text to only the remaining part
            if let Some(partial) = partial_key
                && !partial.is_empty()
                && key.starts_with(partial)
            {
                item.insert_text = Some(key[partial.len()..].to_string());
            }

            completion_items.push(item);
        }
    }

    // Sort by label for consistent ordering
    completion_items.sort_by(|a, b| a.label.cmp(&b.label));

    completion_items
}

/// Extract partial key from text at cursor position
///
/// # Arguments
/// * `text` - Full text of the file
/// * `line` - Line number (0-indexed)
/// * `character` - Character position in line (0-indexed)
///
/// # Returns
/// Partial key if found (e.g., "common." or "common.hel")
#[must_use]
pub fn extract_partial_key(text: &str, line: u32, character: u32) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let line_text = lines.get(line as usize)?;

    // Get text before cursor (clamp to line length)
    let char_pos = character as usize;
    let line_len = line_text.len();

    if char_pos > line_len {
        return None;
    }

    let before_cursor = &line_text[..char_pos];

    // Find the last opening quote before cursor
    let key_start = before_cursor.rfind('"')?;

    // Extract the partial key (everything between the opening quote and cursor)
    let partial = &before_cursor[key_start + 1..];

    // Empty string is valid (represents start of key)
    if partial.is_empty() {
        return Some(String::new());
    }

    // Only return if it looks like a valid key (alphanumeric, dots, underscores, hyphens)
    if partial.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '_' || c == '-') {
        Some(partial.to_string())
    } else {
        None
    }
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
        );

        let translations = vec![en_translation];
        let items = generate_completions(&db, &translations, None);

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
        );

        let translations = vec![en_translation];
        let items = generate_completions(&db, &translations, Some("common."));

        assert_that!(items.len(), eq(2));
        assert_that!(items[0].label, eq("common.goodbye"));
        assert_that!(items[1].label, eq("common.hello"));
    }

    #[rstest]
    fn generate_completions_deduplicates_keys() {
        let db = I18nDatabaseImpl::default();

        // Same key in multiple languages
        let en_translation = Translation::new(
            &db,
            "en".to_string(),
            "/test/en.json".to_string(),
            HashMap::from([("common.hello".to_string(), "Hello".to_string())]),
            "{}".to_string(),
            HashMap::new(),
        );

        let ja_translation = Translation::new(
            &db,
            "ja".to_string(),
            "/test/ja.json".to_string(),
            HashMap::from([("common.hello".to_string(), "Hello in Japanese".to_string())]),
            "{}".to_string(),
            HashMap::new(),
        );

        let translations = vec![en_translation, ja_translation];
        let items = generate_completions(&db, &translations, None);

        // Should only have one item (deduplicated)
        assert_that!(items.len(), eq(1));
        assert_that!(items[0].label, eq("common.hello"));
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
        );

        let translations = vec![en_translation];
        let items = generate_completions(&db, &translations, Some("nonexistent."));

        assert_that!(items, is_empty());
    }

    #[rstest]
    #[case(r#"const msg = t(""#, 0, 15, Some(""))]
    #[case(r#"const msg = t("common"#, 0, 21, Some("common"))]
    #[case(r#"const msg = t("common."#, 0, 22, Some("common."))]
    #[case(r#"const msg = t("common.hel"#, 0, 25, Some("common.hel"))]
    #[case(r#"const msg = t("common-key"#, 0, 25, Some("common-key"))]
    #[case(r#"const msg = t("common_key"#, 0, 25, Some("common_key"))]
    fn extract_partial_key_cases(
        #[case] text: &str,
        #[case] line: u32,
        #[case] character: u32,
        #[case] expected: Option<&str>,
    ) {
        let result = extract_partial_key(text, line, character);
        assert_that!(result.as_deref(), eq(expected));
    }

    #[rstest]
    fn extract_partial_key_multiline() {
        let text = r#"const msg1 = t("common.hello");
const msg2 = t("errors."#;

        let result = extract_partial_key(text, 1, 23);
        assert_that!(result.as_deref(), eq(Some("errors.")));
    }
}
