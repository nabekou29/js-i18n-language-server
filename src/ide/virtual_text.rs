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
    max_length: usize,
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
            let truncated_value = truncate_value(&value, max_length);
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

fn truncate_value(value: &str, max_length: usize) -> String {
    let char_count = value.chars().count();
    if char_count <= max_length {
        value.to_string()
    } else {
        let truncated: String = value.chars().take(max_length.saturating_sub(1)).collect();
        format!("{truncated}…")
    }
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
        let result = truncate_value("Hello", 30);
        assert_that!(result, eq("Hello"));
    }

    #[rstest]
    fn truncate_value_exact_length() {
        let result = truncate_value("Hello World", 11);
        assert_that!(result, eq("Hello World"));
    }

    #[rstest]
    fn truncate_value_long_text() {
        let result = truncate_value("This is a very long message that should be truncated", 20);
        assert_that!(result, eq("This is a very long…"));
    }

    #[rstest]
    fn truncate_value_japanese_text() {
        let result = truncate_value("これは長いメッセージです", 10);
        assert_that!(result, eq("これは長いメッセー…"));
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

        let decorations =
            get_translation_decorations(&db, source_file, &[translation], Some("ja"), 30, ".");

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

        let decorations =
            get_translation_decorations(&db, source_file, &[translation], Some("ja"), 10, ".");

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

        let decorations =
            get_translation_decorations(&db, source_file, &[translation], Some("fr"), 30, ".");

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
            get_translation_decorations(&db, source_file, &[translation], None, 30, ".");

        assert_that!(decorations, len(eq(1)));
        assert_that!(decorations[0].value, eq("Hello"));
    }
}
