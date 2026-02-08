//! Namespace filtering for translation lookups.

use crate::db::I18nDatabase;
use crate::input::translation::Translation;

/// Filters translations by namespace based on priority:
/// 1. `explicit_namespace` - from `t("ns:key")` or `t("key", {ns: "ns"})`
/// 2. `declared_namespaces[0]` - from `useTranslation(["ns1", "ns2"])`
/// 3. `declared_namespace` - from `useTranslation("ns")`
/// 4. `default_namespace` - from settings
/// 5. `None` - returns all translations (backward compatibility)
#[must_use]
pub fn filter_translations_by_namespace<'a>(
    db: &dyn I18nDatabase,
    translations: &'a [Translation],
    explicit_namespace: Option<&str>,
    declared_namespace: Option<&str>,
    declared_namespaces: Option<&[String]>,
    default_namespace: Option<&str>,
) -> Vec<&'a Translation> {
    let resolved_namespace = resolve_namespace(
        explicit_namespace,
        declared_namespace,
        declared_namespaces,
        default_namespace,
    );

    resolved_namespace.map_or_else(
        || translations.iter().collect(),
        |ns| {
            translations
                .iter()
                .filter(|t| t.namespace(db).as_ref().is_some_and(|n| n == ns))
                .collect()
        },
    )
}

/// Filters translations by a single explicit namespace.
///
/// Simpler API for callers that only have a namespace from `parse_key_with_namespace`.
#[must_use]
pub fn filter_by_namespace<'a>(
    db: &dyn I18nDatabase,
    translations: &'a [Translation],
    namespace: Option<&str>,
) -> Vec<&'a Translation> {
    filter_translations_by_namespace(db, translations, namespace, None, None, None)
}

#[must_use]
pub fn resolve_namespace<'a>(
    explicit_namespace: Option<&'a str>,
    declared_namespace: Option<&'a str>,
    declared_namespaces: Option<&'a [String]>,
    default_namespace: Option<&'a str>,
) -> Option<&'a str> {
    explicit_namespace
        .or_else(|| declared_namespaces.and_then(|ns| ns.first().map(String::as_str)))
        .or(declared_namespace)
        .or(default_namespace)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use std::collections::HashMap;

    use googletest::prelude::*;
    use rstest::*;

    use super::*;
    use crate::db::I18nDatabaseImpl;
    use crate::test_utils::create_translation_with_namespace;

    #[fixture]
    fn db() -> I18nDatabaseImpl {
        I18nDatabaseImpl::default()
    }

    #[rstest]
    fn resolve_namespace_explicit_first() {
        let namespaces = vec!["array1".to_string(), "array2".to_string()];
        let result = resolve_namespace(
            Some("explicit"),
            Some("declared"),
            Some(&namespaces),
            Some("default"),
        );
        assert_that!(result, some(eq("explicit")));
    }

    #[rstest]
    fn resolve_namespace_declared_array_second() {
        let namespaces = vec!["array1".to_string(), "array2".to_string()];
        let result = resolve_namespace(None, Some("declared"), Some(&namespaces), Some("default"));
        assert_that!(result, some(eq("array1")));
    }

    #[rstest]
    fn resolve_namespace_declared_single_third() {
        let result = resolve_namespace(None, Some("declared"), None, Some("default"));
        assert_that!(result, some(eq("declared")));
    }

    #[rstest]
    fn resolve_namespace_default_fourth() {
        let result = resolve_namespace(None, None, None, Some("default"));
        assert_that!(result, some(eq("default")));
    }

    #[rstest]
    fn resolve_namespace_none_when_all_empty() {
        let result = resolve_namespace(None, None, None, None);
        assert_that!(result, none());
    }

    #[rstest]
    fn filter_by_explicit_namespace(db: I18nDatabaseImpl) {
        let common = create_translation_with_namespace(
            &db,
            "en",
            Some("common"),
            "/locales/en/common.json",
            HashMap::from([("hello".to_string(), "Hello".to_string())]),
        );
        let errors = create_translation_with_namespace(
            &db,
            "en",
            Some("errors"),
            "/locales/en/errors.json",
            HashMap::from([("notFound".to_string(), "Not Found".to_string())]),
        );
        let translations = vec![common, errors];

        let filtered = filter_translations_by_namespace(
            &db,
            &translations,
            Some("common"), // explicit
            None,
            None,
            None,
        );

        assert_that!(filtered.len(), eq(1));
        assert_that!(filtered[0].namespace(&db).as_deref(), some(eq("common")));
    }

    #[rstest]
    fn filter_returns_all_when_no_namespace(db: I18nDatabaseImpl) {
        let common = create_translation_with_namespace(
            &db,
            "en",
            Some("common"),
            "/locales/en/common.json",
            HashMap::from([("hello".to_string(), "Hello".to_string())]),
        );
        let errors = create_translation_with_namespace(
            &db,
            "en",
            Some("errors"),
            "/locales/en/errors.json",
            HashMap::from([("notFound".to_string(), "Not Found".to_string())]),
        );
        let translations = vec![common, errors];

        let filtered = filter_translations_by_namespace(&db, &translations, None, None, None, None);

        assert_that!(filtered.len(), eq(2));
    }

    #[rstest]
    fn filter_by_namespace_some(db: I18nDatabaseImpl) {
        let common = create_translation_with_namespace(
            &db,
            "en",
            Some("common"),
            "/locales/en/common.json",
            HashMap::from([("hello".to_string(), "Hello".to_string())]),
        );
        let errors = create_translation_with_namespace(
            &db,
            "en",
            Some("errors"),
            "/locales/en/errors.json",
            HashMap::from([("notFound".to_string(), "Not Found".to_string())]),
        );
        let translations = vec![common, errors];

        let filtered = filter_by_namespace(&db, &translations, Some("common"));
        assert_that!(filtered.len(), eq(1));
        assert_that!(filtered[0].namespace(&db).as_deref(), some(eq("common")));
    }

    #[rstest]
    fn filter_by_namespace_none_returns_all(db: I18nDatabaseImpl) {
        let common = create_translation_with_namespace(
            &db,
            "en",
            Some("common"),
            "/locales/en/common.json",
            HashMap::from([("hello".to_string(), "Hello".to_string())]),
        );
        let errors = create_translation_with_namespace(
            &db,
            "en",
            Some("errors"),
            "/locales/en/errors.json",
            HashMap::from([("notFound".to_string(), "Not Found".to_string())]),
        );
        let translations = vec![common, errors];

        let filtered = filter_by_namespace(&db, &translations, None);
        assert_that!(filtered.len(), eq(2));
    }

    #[rstest]
    fn filter_by_default_namespace(db: I18nDatabaseImpl) {
        let common = create_translation_with_namespace(
            &db,
            "en",
            Some("common"),
            "/locales/en/common.json",
            HashMap::from([("hello".to_string(), "Hello".to_string())]),
        );
        let translation = create_translation_with_namespace(
            &db,
            "en",
            Some("translation"),
            "/locales/en/translation.json",
            HashMap::from([("world".to_string(), "World".to_string())]),
        );
        let translations = vec![common, translation];

        let filtered = filter_translations_by_namespace(
            &db,
            &translations,
            None,
            None,
            None,
            Some("translation"), // default
        );

        assert_that!(filtered.len(), eq(1));
        assert_that!(filtered[0].namespace(&db).as_deref(), some(eq("translation")));
    }
}
