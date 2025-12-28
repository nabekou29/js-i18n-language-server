//! Hover implementation

use std::fmt::Write as _;

use crate::db::I18nDatabase;
use crate::input::translation::Translation;
use crate::interned::TransKey;

/// Generate hover content for a translation key
pub fn generate_hover_content(
    db: &dyn I18nDatabase,
    key: TransKey<'_>,
    translations: &[Translation],
) -> Option<String> {
    let key_text = key.text(db);

    // Collect translations for this key
    let mut translations_found = Vec::new();

    for translation in translations {
        let keys = translation.keys(db);
        if let Some(value) = keys.get(key_text) {
            let language = translation.language(db);
            translations_found.push((language.clone(), value.clone()));
        }
    }

    // No translations found
    if translations_found.is_empty() {
        return None;
    }

    // Format as markdown
    let mut content = format!("**Translation Key:** `{key_text}`\n\n");

    // Sort by language code
    translations_found.sort_by(|a, b| a.0.cmp(&b.0));

    for (language, value) in translations_found {
        let _ = writeln!(content, "**{language}**: {value}");
    }

    Some(content)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::collections::HashMap;

    use googletest::prelude::*;
    use rstest::*;

    use super::*;
    use crate::db::I18nDatabaseImpl;

    /// テスト用の Translation を作成するヘルパー関数
    fn create_translation(
        db: &I18nDatabaseImpl,
        language: &str,
        file_path: &str,
        keys: HashMap<String, String>,
    ) -> Translation {
        Translation::new(
            db,
            language.to_string(),
            file_path.to_string(),
            keys,
            "{}".to_string(), // raw_content (テストでは使用しない)
            HashMap::new(),   // key_ranges (テストでは使用しない)
            HashMap::new(),   // value_ranges (テストでは使用しない)
        )
    }

    #[rstest]
    fn generate_hover_content_with_single_translation() {
        let db = I18nDatabaseImpl::default();

        let translation = create_translation(
            &db,
            "en",
            "/test/locales/en.json",
            HashMap::from([("common.hello".to_string(), "Hello".to_string())]),
        );

        let key = TransKey::new(&db, "common.hello".to_string());
        let translations = vec![translation];

        let content = generate_hover_content(&db, key, &translations);

        assert_that!(content, some(contains_substring("**Translation Key:** `common.hello`")));
        assert_that!(content.as_ref().unwrap(), contains_substring("**en**: Hello"));
    }

    #[rstest]
    fn generate_hover_content_with_multiple_languages() {
        let db = I18nDatabaseImpl::default();

        // 意図的にソート順と異なる順序で追加（ja → en）
        let ja_translation = create_translation(
            &db,
            "ja",
            "/test/locales/ja.json",
            HashMap::from([("common.hello".to_string(), "こんにちは".to_string())]),
        );

        let en_translation = create_translation(
            &db,
            "en",
            "/test/locales/en.json",
            HashMap::from([("common.hello".to_string(), "Hello".to_string())]),
        );

        let key = TransKey::new(&db, "common.hello".to_string());
        let translations = vec![ja_translation, en_translation];

        let content = generate_hover_content(&db, key, &translations).unwrap();

        // キーが含まれている
        assert_that!(content, contains_substring("**Translation Key:** `common.hello`"));

        // 両方の言語が含まれている
        assert_that!(content, contains_substring("**en**: Hello"));
        assert_that!(content, contains_substring("**ja**: こんにちは"));

        // 言語コード順にソートされている（en が ja より先）
        let en_pos = content.find("**en**").unwrap();
        let ja_pos = content.find("**ja**").unwrap();
        assert_that!(en_pos, lt(ja_pos));
    }

    #[rstest]
    fn generate_hover_content_with_no_translations() {
        let db = I18nDatabaseImpl::default();

        let translation = create_translation(
            &db,
            "en",
            "/test/locales/en.json",
            HashMap::from([("common.hello".to_string(), "Hello".to_string())]),
        );

        // 存在しないキーを検索
        let key = TransKey::new(&db, "nonexistent.key".to_string());
        let translations = vec![translation];

        let content = generate_hover_content(&db, key, &translations);

        assert_that!(content, none());
    }

    #[rstest]
    fn generate_hover_content_with_empty_translations() {
        let db = I18nDatabaseImpl::default();

        let key = TransKey::new(&db, "common.hello".to_string());
        let translations: Vec<Translation> = vec![];

        let content = generate_hover_content(&db, key, &translations);

        assert_that!(content, none());
    }

    #[rstest]
    fn generate_hover_content_with_partial_translations() {
        let db = I18nDatabaseImpl::default();

        // en にはキーがあるが、ja にはない
        let en_translation = create_translation(
            &db,
            "en",
            "/test/locales/en.json",
            HashMap::from([("common.hello".to_string(), "Hello".to_string())]),
        );

        let ja_translation = create_translation(
            &db,
            "ja",
            "/test/locales/ja.json",
            HashMap::from([("common.goodbye".to_string(), "さようなら".to_string())]),
        );

        let key = TransKey::new(&db, "common.hello".to_string());
        let translations = vec![en_translation, ja_translation];

        let content = generate_hover_content(&db, key, &translations).unwrap();

        // en のみ含まれている
        assert_that!(content, contains_substring("**en**: Hello"));
        assert_that!(content, not(contains_substring("**ja**")));
    }
}
