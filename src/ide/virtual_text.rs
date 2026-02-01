//! Virtual text (inline translation display) for editor extensions.

use serde::{
    Deserialize,
    Serialize,
};
use tower_lsp::lsp_types::Range;

use crate::db::I18nDatabase;
use crate::input::source::SourceFile;
use crate::input::translation::Translation;

/// Translation decoration info for a key usage in the document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationDecoration {
    pub range: Range,
    pub key: String,
    pub value: String,
}

/// Generates translation decorations for all key usages in a source file.
#[must_use]
pub fn get_translation_decorations(
    db: &dyn I18nDatabase,
    source_file: SourceFile,
    translations: &[Translation],
    language: Option<&str>,
    max_length: Option<usize>,
    max_width: usize,
    key_separator: &str,
) -> Vec<TranslationDecoration> {
    let key_usages = crate::syntax::analyze_source(db, source_file, key_separator.to_string());

    let mut decorations = Vec::new();

    for usage in key_usages {
        let key = usage.key(db);
        let key_text = key.text(db);
        let range: Range = usage.range(db).into();

        let value = get_translation_value(db, translations, key_text, language);

        if let Some(value) = value {
            let truncated_value = truncate_value(&value, max_length, max_width);
            decorations.push(TranslationDecoration {
                range,
                key: key_text.clone(),
                value: truncated_value,
            });
        }
    }

    decorations
}

fn get_translation_value(
    db: &dyn I18nDatabase,
    translations: &[Translation],
    key_text: &str,
    language: Option<&str>,
) -> Option<String> {
    translations
        .iter()
        .filter(|t| language.is_none_or(|lang| t.language(db) == lang))
        .find_map(|t| t.keys(db).get(key_text).cloned())
}

fn truncate_value(value: &str, max_length: Option<usize>, max_width: usize) -> String {
    max_length.map_or_else(
        || truncate_by_width(value, max_width),
        |max_l| truncate_by_length(value, max_l),
    )
}

fn truncate_by_length(value: &str, max_length: usize) -> String {
    let char_count = value.chars().count();
    if char_count <= max_length {
        value.to_string()
    } else {
        let truncated: String = value.chars().take(max_length.saturating_sub(1)).collect();
        format!("{truncated}…")
    }
}

fn truncate_by_width(value: &str, max_width: usize) -> String {
    use unicode_width::UnicodeWidthChar;

    let total_width: usize = value.chars().filter_map(UnicodeWidthChar::width).sum();
    if total_width <= max_width {
        return value.to_string();
    }

    // Reserve width 1 for ellipsis "…"
    let target_width = max_width.saturating_sub(1);
    let mut current_width = 0;
    let truncated: String = value
        .chars()
        .take_while(|c| {
            let w = UnicodeWidthChar::width(*c).unwrap_or(0);
            if current_width + w > target_width {
                return false;
            }
            current_width += w;
            true
        })
        .collect();

    format!("{truncated}…")
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use std::collections::HashMap;

    use googletest::prelude::*;
    use rstest::*;

    use super::*;
    use crate::db::I18nDatabaseImpl;
    use crate::input::source::ProgrammingLanguage;
    use crate::test_utils::create_translation;

    fn create_source_file(db: &I18nDatabaseImpl, content: &str) -> SourceFile {
        SourceFile::new(
            db,
            "file:///test/app.tsx".to_string(),
            content.to_string(),
            ProgrammingLanguage::Tsx,
        )
    }

    #[rstest]
    fn truncate_value_short_text() {
        let result = truncate_value("Hello", Some(30), 32);
        assert_that!(result, eq("Hello"));
    }

    #[rstest]
    fn truncate_value_exact_length() {
        let result = truncate_value("Hello World", Some(11), 32);
        assert_that!(result, eq("Hello World"));
    }

    #[rstest]
    fn truncate_value_long_text() {
        let result =
            truncate_value("This is a very long message that should be truncated", Some(20), 32);
        assert_that!(result, eq("This is a very long…"));
    }

    #[rstest]
    fn truncate_value_japanese_text() {
        let result = truncate_value("これは長いメッセージです", Some(10), 32);
        assert_that!(result, eq("これは長いメッセー…"));
    }

    #[rstest]
    fn truncate_by_width_ascii_short() {
        let result = truncate_by_width("Hello", 30);
        assert_that!(result, eq("Hello"));
    }

    #[rstest]
    fn truncate_by_width_ascii_exact() {
        // "Hello" = width 5
        let result = truncate_by_width("Hello", 5);
        assert_that!(result, eq("Hello"));
    }

    #[rstest]
    fn truncate_by_width_ascii_truncated() {
        // "Hello World" = width 11, max_width 8 → 7 chars + "…"
        let result = truncate_by_width("Hello World", 8);
        assert_that!(result, eq("Hello W…"));
    }

    #[rstest]
    fn truncate_by_width_cjk_fits() {
        // "こんにちは" = 5 chars × width 2 = width 10
        let result = truncate_by_width("こんにちは", 10);
        assert_that!(result, eq("こんにちは"));
    }

    #[rstest]
    fn truncate_by_width_cjk_truncated() {
        // "こんにちは" = width 10, max_width 8 → target 7 → 3 CJK chars (width 6) + "…"
        let result = truncate_by_width("こんにちは", 8);
        assert_that!(result, eq("こんに…"));
    }

    #[rstest]
    fn truncate_by_width_mixed() {
        // "Hello世界" = 5 (ASCII) + 4 (CJK) = width 9
        let result = truncate_by_width("Hello世界", 9);
        assert_that!(result, eq("Hello世界"));
    }

    #[rstest]
    fn truncate_by_width_mixed_truncated() {
        // "Hello世界test" = 5 + 4 + 4 = width 13, max_width 10 → target 9
        // "Hello世界" = width 9, fits
        let result = truncate_by_width("Hello世界test", 10);
        assert_that!(result, eq("Hello世界…"));
    }

    #[rstest]
    fn truncate_by_width_cjk_boundary() {
        // max_width 7, target 6 → "こんに" = width 6
        let result = truncate_by_width("こんにちは", 7);
        assert_that!(result, eq("こんに…"));
    }

    #[rstest]
    fn truncate_by_width_cjk_odd_boundary() {
        // max_width 6, target 5 → "こん" = width 4 (next CJK would be 6, exceeds 5)
        let result = truncate_by_width("こんにちは", 6);
        assert_that!(result, eq("こん…"));
    }

    #[rstest]
    fn truncate_value_falls_back_to_max_width() {
        // max_length=None, max_width=8; CJK should be truncated by width
        let result = truncate_value("こんにちは世界", None, 8);
        // width 14, max_width 8, target 7 → "こんに" = width 6 + "…"
        assert_that!(result, eq("こんに…"));
    }

    #[rstest]
    fn truncate_value_max_length_overrides_max_width() {
        // max_length=5 takes priority over max_width=32
        let result = truncate_value("こんにちは世界", Some(5), 32);
        // 7 chars, max_length 5 → 4 chars + "…"
        assert_that!(result, eq("こんにち…"));
    }

    #[rstest]
    fn get_decorations_basic() {
        let db = I18nDatabaseImpl::default();

        let source_file = create_source_file(&db, r#"const msg = t("common.hello");"#);

        let translation = create_translation(
            &db,
            "ja",
            "/test/locales/ja.json",
            HashMap::from([("common.hello".to_string(), "こんにちは".to_string())]),
        );

        let decorations = get_translation_decorations(
            &db,
            source_file,
            &[translation],
            Some("ja"),
            None,
            30,
            ".",
        );

        assert_that!(decorations, len(eq(1)));
        assert_that!(decorations[0].key, eq("common.hello"));
        assert_that!(decorations[0].value, eq("こんにちは"));
    }

    #[rstest]
    fn get_decorations_with_truncation() {
        let db = I18nDatabaseImpl::default();

        let source_file = create_source_file(&db, r#"const msg = t("common.hello");"#);

        let translation = create_translation(
            &db,
            "ja",
            "/test/locales/ja.json",
            HashMap::from([(
                "common.hello".to_string(),
                "これは非常に長いメッセージで切り詰める必要があります".to_string(),
            )]),
        );

        let decorations = get_translation_decorations(
            &db,
            source_file,
            &[translation],
            Some("ja"),
            Some(10),
            32,
            ".",
        );

        assert_that!(decorations, len(eq(1)));
        assert_that!(decorations[0].value, eq("これは非常に長いメ…"));
    }

    #[rstest]
    fn get_decorations_no_language_match() {
        let db = I18nDatabaseImpl::default();

        let source_file = create_source_file(&db, r#"const msg = t("common.hello");"#);

        let translation = create_translation(
            &db,
            "en",
            "/test/locales/en.json",
            HashMap::from([("common.hello".to_string(), "Hello".to_string())]),
        );

        let decorations = get_translation_decorations(
            &db,
            source_file,
            &[translation],
            Some("fr"),
            None,
            30,
            ".",
        );

        assert_that!(decorations, is_empty());
    }

    #[rstest]
    fn get_decorations_no_language_specified() {
        let db = I18nDatabaseImpl::default();

        let source_file = create_source_file(&db, r#"const msg = t("common.hello");"#);

        let translation = create_translation(
            &db,
            "en",
            "/test/locales/en.json",
            HashMap::from([("common.hello".to_string(), "Hello".to_string())]),
        );

        let decorations =
            get_translation_decorations(&db, source_file, &[translation], None, None, 30, ".");

        assert_that!(decorations, len(eq(1)));
        assert_that!(decorations[0].value, eq("Hello"));
    }
}
