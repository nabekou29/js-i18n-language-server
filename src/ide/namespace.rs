//! Namespace filtering for translation lookups.

use crate::db::I18nDatabase;
use crate::input::translation::Translation;
use crate::ir::key_usage::KeyUsage;
use crate::syntax::analyzer::extractor::parse_key_with_namespace;

/// Filters translations by a resolved namespace.
///
/// When `namespace` is `Some`, only translations with a matching namespace are returned.
/// When `None`, all translations are returned (backward compatibility for non-namespaced setups).
#[must_use]
pub fn filter_by_namespace<'a>(
    db: &dyn I18nDatabase,
    translations: &'a [Translation],
    namespace: Option<&str>,
) -> Vec<&'a Translation> {
    namespace.map_or_else(
        || translations.iter().collect(),
        |ns| {
            translations
                .iter()
                .filter(|t| t.namespace(db).as_ref().is_some_and(|n| n == ns))
                .collect()
        },
    )
}

/// Resolves namespace and key part from a `KeyUsage`.
///
/// Combines explicit namespace (from key text), declared namespace (from `useTranslation`),
/// and default namespace into a single resolved result.
/// Returns `(resolved_namespace, key_part)`.
#[must_use]
pub fn resolve_usage_namespace(
    db: &dyn I18nDatabase,
    usage: KeyUsage<'_>,
    namespace_separator: Option<&str>,
    default_namespace: Option<&str>,
) -> (Option<String>, String) {
    let full_key = usage.key(db).text(db);
    let (explicit_ns, key_part) = parse_key_with_namespace(full_key, namespace_separator);
    let declared_ns = usage.namespace(db);
    let declared_nss = usage.namespaces(db);

    let ns = resolve_namespace(
        explicit_ns.as_deref(),
        declared_ns.as_deref(),
        declared_nss.as_deref(),
        default_namespace,
    )
    .map(str::to_owned);

    (ns, key_part)
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
}
