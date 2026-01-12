//! Namespace フィルタリングモジュール
//!
//! 翻訳キーの使用箇所から適切な翻訳ファイルを特定するための
//! namespace 解決ロジックを提供します。

use crate::db::I18nDatabase;
use crate::input::translation::Translation;

/// Namespace 解決の優先度に従って翻訳をフィルタリングする
///
/// # Namespace 解決優先度
/// 1. `explicit_namespace` - t("ns:key") または t("key", {ns: "ns"}) から
/// 2. `declared_namespaces` の最初 - `useTranslation(["ns1", "ns2"])` から
/// 3. `declared_namespace` - useTranslation("ns") から
/// 4. `default_namespace` - 設定から
/// 5. None → 全翻訳を返す（後方互換性）
///
/// # Arguments
/// * `db` - Salsa データベース
/// * `translations` - フィルタリング対象の翻訳リスト
/// * `explicit_namespace` - キーから解析された明示的 namespace
/// * `declared_namespace` - useTranslation から宣言された単一 namespace
/// * `declared_namespaces` - useTranslation から宣言された複数 namespace
/// * `default_namespace` - 設定からのデフォルト namespace
///
/// # Returns
/// フィルタリングされた翻訳のリスト
#[must_use]
pub fn filter_translations_by_namespace<'a>(
    db: &dyn I18nDatabase,
    translations: &'a [Translation],
    explicit_namespace: Option<&str>,
    declared_namespace: Option<&str>,
    declared_namespaces: Option<&[String]>,
    default_namespace: Option<&str>,
) -> Vec<&'a Translation> {
    // namespace 解決
    let resolved_namespace = resolve_namespace(
        explicit_namespace,
        declared_namespace,
        declared_namespaces,
        default_namespace,
    );

    // namespace が解決された場合、その namespace に一致する翻訳のみを返す
    // 解決されなかった場合、全翻訳を返す（後方互換性）
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

/// Namespace を優先度に従って解決する
///
/// # 優先度
/// 1. `explicit_namespace`
/// 2. `declared_namespaces` の最初
/// 3. `declared_namespace`
/// 4. `default_namespace`
/// 5. None
#[must_use]
pub const fn resolve_namespace<'a>(
    explicit_namespace: Option<&'a str>,
    declared_namespace: Option<&'a str>,
    declared_namespaces: Option<&'a [String]>,
    default_namespace: Option<&'a str>,
) -> Option<&'a str> {
    // 1. 明示的 namespace（キーから解析）
    if let Some(ns) = explicit_namespace {
        return Some(ns);
    }

    // 2. 宣言された複数 namespace の最初
    if let Some(namespaces) = declared_namespaces
        && let Some(first) = namespaces.first()
    {
        return Some(first.as_str());
    }

    // 3. 宣言された単一 namespace
    if let Some(ns) = declared_namespace {
        return Some(ns);
    }

    // 4. デフォルト namespace
    if let Some(ns) = default_namespace {
        return Some(ns);
    }

    // 5. 解決できない場合は None
    None
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

    // ===== resolve_namespace テスト =====

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

    // ===== filter_translations_by_namespace テスト =====

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

        // namespace 指定なし → 全翻訳を返す
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
