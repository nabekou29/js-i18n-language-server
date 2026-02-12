//! Virtual text (inline translation display) for editor extensions.

use serde::{
    Deserialize,
    Serialize,
};
use tower_lsp::lsp_types::Range;

use crate::db::I18nDatabase;
use crate::ide::namespace::filter_translations_by_namespace;
use crate::ide::plural::find_plural_variants;
use crate::input::source::SourceFile;
use crate::input::translation::Translation;
use crate::syntax::analyzer::extractor::parse_key_with_namespace;

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
    key_separator: &str,
    namespace_separator: Option<&str>,
    default_namespace: Option<&str>,
) -> Vec<TranslationDecoration> {
    let key_usages = crate::syntax::analyze_source(db, source_file, key_separator.to_string());

    let mut decorations = Vec::new();

    for usage in key_usages {
        let key = usage.key(db);
        let full_key_text = key.text(db);
        let range: Range = usage.range(db).into();

        let (explicit_ns, key_part) = parse_key_with_namespace(full_key_text, namespace_separator);
        let declared_ns = usage.namespace(db);
        let declared_nss = usage.namespaces(db);

        let filtered = filter_translations_by_namespace(
            db,
            translations,
            explicit_ns.as_deref(),
            declared_ns.as_deref(),
            declared_nss.as_deref(),
            default_namespace,
        );

        let value = get_translation_value(db, &filtered, &key_part, language);

        if let Some(value) = value {
            decorations.push(TranslationDecoration { range, key: full_key_text.clone(), value });
        }
    }

    decorations
}

fn get_translation_value(
    db: &dyn I18nDatabase,
    translations: &[&Translation],
    key_text: &str,
    language: Option<&str>,
) -> Option<String> {
    translations.iter().filter(|t| language.is_none_or(|lang| t.language(db) == lang)).find_map(
        |t| {
            let keys = t.keys(db);

            if let Some(value) = keys.get(key_text) {
                return Some(value.clone());
            }

            // Plural fallback: prefer _other variant, then first available
            let variants = find_plural_variants(key_text, keys);
            variants
                .iter()
                .find(|(k, _)| k.ends_with("_other"))
                .or_else(|| variants.first())
                .map(|(_, value)| value.to_string())
        },
    )
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
    use crate::test_utils::{
        create_translation,
        create_translation_with_namespace,
    };

    fn create_source_file(db: &I18nDatabaseImpl, content: &str) -> SourceFile {
        SourceFile::new(
            db,
            "file:///test/app.tsx".to_string(),
            content.to_string(),
            ProgrammingLanguage::Tsx,
        )
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
            ".",
            None,
            None,
        );

        assert_that!(decorations, len(eq(1)));
        assert_that!(decorations[0].key, eq("common.hello"));
        assert_that!(decorations[0].value, eq("こんにちは"));
    }

    #[rstest]
    fn get_decorations_returns_full_value() {
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
            ".",
            None,
            None,
        );

        assert_that!(decorations, len(eq(1)));
        assert_that!(
            decorations[0].value,
            eq("これは非常に長いメッセージで切り詰める必要があります")
        );
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
            ".",
            None,
            None,
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
            get_translation_decorations(&db, source_file, &[translation], None, ".", None, None);

        assert_that!(decorations, len(eq(1)));
        assert_that!(decorations[0].value, eq("Hello"));
    }

    #[rstest]
    fn get_decorations_plural_fallback_other() {
        let db = I18nDatabaseImpl::default();

        let source_file = create_source_file(&db, r#"const msg = t("items");"#);

        let translation = create_translation(
            &db,
            "en",
            "/test/locales/en.json",
            HashMap::from([
                ("items_one".to_string(), "{{count}} item".to_string()),
                ("items_other".to_string(), "{{count}} items".to_string()),
            ]),
        );

        let decorations = get_translation_decorations(
            &db,
            source_file,
            &[translation],
            Some("en"),
            ".",
            None,
            None,
        );

        assert_that!(decorations, len(eq(1)));
        assert_that!(decorations[0].key, eq("items"));
        // Prefers _other as representative value
        assert_that!(decorations[0].value, eq("{{count}} items"));
    }

    #[rstest]
    fn get_decorations_plural_fallback_ordinal() {
        let db = I18nDatabaseImpl::default();

        let source_file = create_source_file(&db, r#"const msg = t("place");"#);

        let translation = create_translation(
            &db,
            "en",
            "/test/locales/en.json",
            HashMap::from([
                ("place_ordinal_one".to_string(), "{{count}}st".to_string()),
                ("place_ordinal_two".to_string(), "{{count}}nd".to_string()),
                ("place_ordinal_few".to_string(), "{{count}}rd".to_string()),
                ("place_ordinal_other".to_string(), "{{count}}th".to_string()),
            ]),
        );

        let decorations = get_translation_decorations(
            &db,
            source_file,
            &[translation],
            Some("en"),
            ".",
            None,
            None,
        );

        assert_that!(decorations, len(eq(1)));
        assert_that!(decorations[0].key, eq("place"));
        // Falls back to _ordinal_other
        assert_that!(decorations[0].value, eq("{{count}}th"));
    }

    #[rstest]
    fn get_decorations_plural_fallback_no_other() {
        let db = I18nDatabaseImpl::default();

        let source_file = create_source_file(&db, r#"const msg = t("items");"#);

        // Only _one and _few exist (no _other)
        let translation = create_translation(
            &db,
            "en",
            "/test/locales/en.json",
            HashMap::from([
                ("items_one".to_string(), "{{count}} item".to_string()),
                ("items_few".to_string(), "{{count}} items (few)".to_string()),
            ]),
        );

        let decorations = get_translation_decorations(
            &db,
            source_file,
            &[translation],
            Some("en"),
            ".",
            None,
            None,
        );

        assert_that!(decorations, len(eq(1)));
        assert_that!(decorations[0].key, eq("items"));
        // Falls back to first available variant
        assert_that!(decorations[0].value, eq("{{count}} item"));
    }

    #[rstest]
    fn get_decorations_filters_by_namespace() {
        let db = I18nDatabaseImpl::default();

        // Source code uses useTranslation with namespace (simulated by explicit ns in key)
        let source_file = create_source_file(&db, r#"const msg = t("common:hello");"#);

        let common = create_translation_with_namespace(
            &db,
            "ja",
            Some("common"),
            "/locales/ja/common.json",
            HashMap::from([("hello".to_string(), "こんにちは".to_string())]),
        );
        let errors = create_translation_with_namespace(
            &db,
            "ja",
            Some("errors"),
            "/locales/ja/errors.json",
            HashMap::from([("hello".to_string(), "エラー".to_string())]),
        );

        let decorations = get_translation_decorations(
            &db,
            source_file,
            &[common, errors],
            Some("ja"),
            ".",
            Some(":"),
            None,
        );

        assert_that!(decorations, len(eq(1)));
        assert_that!(decorations[0].key, eq("common:hello"));
        // Should show common namespace value, not errors
        assert_that!(decorations[0].value, eq("こんにちは"));
    }

    #[rstest]
    fn get_decorations_no_namespace_returns_first_match() {
        let db = I18nDatabaseImpl::default();

        let source_file = create_source_file(&db, r#"const msg = t("hello");"#);

        let common = create_translation_with_namespace(
            &db,
            "ja",
            Some("common"),
            "/locales/ja/common.json",
            HashMap::from([("hello".to_string(), "こんにちは".to_string())]),
        );
        let errors = create_translation_with_namespace(
            &db,
            "ja",
            Some("errors"),
            "/locales/ja/errors.json",
            HashMap::from([("hello".to_string(), "エラー".to_string())]),
        );

        // Without namespace separator, no filtering occurs
        let decorations = get_translation_decorations(
            &db,
            source_file,
            &[common, errors],
            Some("ja"),
            ".",
            None,
            None,
        );

        assert_that!(decorations, len(eq(1)));
        // Returns first match (common comes first)
        assert_that!(decorations[0].value, eq("こんにちは"));
    }
}
