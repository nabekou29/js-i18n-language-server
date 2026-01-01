//! Go to Definition implementation

use tower_lsp::lsp_types::{
    Location,
    Range,
};

use crate::db::I18nDatabase;
use crate::input::translation::Translation;
use crate::interned::TransKey;
use crate::types::SourceRange;

/// Find translation key definitions
///
/// # Arguments
/// * `db` - Salsa database
/// * `key` - Translation key
/// * `translations` - All translation data
/// * `key_separator` - Key separator (e.g., ".")
///
/// # 逆方向 prefix マッチ
/// 完全一致がない場合、最初の子キー（例: `nested.key`）の位置にフォールバックします。
/// これにより `t('nested')` で `nested.key` がある場合もジャンプできます。
///
/// # Returns
/// All locations where the translation key is defined (returns all if exists in multiple language files)
pub fn find_definitions(
    db: &dyn I18nDatabase,
    key: TransKey<'_>,
    translations: &[Translation],
    key_separator: &str,
) -> Vec<Location> {
    let key_text = key.text(db);
    let mut locations = Vec::new();

    for translation in translations {
        let key_ranges = translation.key_ranges(db);

        // Check if this key exists in this translation file
        if let Some(range) = key_ranges.get(key_text.as_str()) {
            // Create URI from file path
            let file_path = translation.file_path(db);
            let Ok(uri) = tower_lsp::lsp_types::Url::from_file_path(file_path) else {
                tracing::warn!("Failed to create URI from file path: {}", file_path);
                continue;
            };

            locations.push(Location { uri, range: lsp_range_from_source_range(*range) });
            continue;
        }

        // 逆方向 prefix マッチ：完全一致がない場合、最初の子キーにフォールバック
        let prefix = format!("{key_text}{key_separator}");
        if let Some((_, range)) = key_ranges.iter().find(|(k, _)| k.starts_with(&prefix)) {
            let file_path = translation.file_path(db);
            let Ok(uri) = tower_lsp::lsp_types::Url::from_file_path(file_path) else {
                tracing::warn!("Failed to create URI from file path: {}", file_path);
                continue;
            };

            locations.push(Location { uri, range: lsp_range_from_source_range(*range) });
        }
    }

    locations
}

/// Convert `SourceRange` to LSP `Range`
const fn lsp_range_from_source_range(range: SourceRange) -> Range {
    Range {
        start: tower_lsp::lsp_types::Position {
            line: range.start.line,
            character: range.start.character,
        },
        end: tower_lsp::lsp_types::Position {
            line: range.end.line,
            character: range.end.character,
        },
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
    use crate::types::{
        SourcePosition,
        SourceRange,
    };

    #[rstest]
    fn find_definitions_single_translation() {
        let db = I18nDatabaseImpl::default();

        // Create test translation data
        let mut key_ranges = HashMap::new();
        key_ranges.insert(
            "common.hello".to_string(),
            SourceRange {
                start: SourcePosition { line: 1, character: 2 },
                end: SourcePosition { line: 1, character: 15 },
            },
        );

        let translation = Translation::new(
            &db,
            "en".to_string(),
            "/test/locales/en.json".to_string(),
            HashMap::from([("common.hello".to_string(), "Hello".to_string())]),
            r#"{"common": {"hello": "Hello"}}"#.to_string(),
            key_ranges,
            HashMap::new(),
        );

        let key = TransKey::new(&db, "common.hello".to_string());
        let translations = vec![translation];

        let locations = find_definitions(&db, key, &translations, ".");

        assert_that!(locations.len(), eq(1));
        assert_that!(locations[0].uri.path(), ends_with("en.json"));
        assert_that!(locations[0].range.start.line, eq(1));
        assert_that!(locations[0].range.start.character, eq(2));
    }

    #[rstest]
    fn find_definitions_multiple_translations() {
        let db = I18nDatabaseImpl::default();

        // English translation file
        let mut en_key_ranges = HashMap::new();
        en_key_ranges.insert(
            "common.hello".to_string(),
            SourceRange {
                start: SourcePosition { line: 1, character: 2 },
                end: SourcePosition { line: 1, character: 15 },
            },
        );

        let en_translation = Translation::new(
            &db,
            "en".to_string(),
            "/test/locales/en.json".to_string(),
            HashMap::from([("common.hello".to_string(), "Hello".to_string())]),
            r#"{"common": {"hello": "Hello"}}"#.to_string(),
            en_key_ranges,
            HashMap::new(),
        );

        // Japanese translation file
        let mut ja_key_ranges = HashMap::new();
        ja_key_ranges.insert(
            "common.hello".to_string(),
            SourceRange {
                start: SourcePosition { line: 2, character: 4 },
                end: SourcePosition { line: 2, character: 17 },
            },
        );

        let ja_translation = Translation::new(
            &db,
            "ja".to_string(),
            "/test/locales/ja.json".to_string(),
            HashMap::from([("common.hello".to_string(), "Hello in Japanese".to_string())]),
            r#"{"common": {"hello": "Hello in Japanese"}}"#.to_string(),
            ja_key_ranges,
            HashMap::new(),
        );

        let key = TransKey::new(&db, "common.hello".to_string());
        let translations = vec![en_translation, ja_translation];

        let locations = find_definitions(&db, key, &translations, ".");

        // Definitions found in both translation files
        assert_that!(locations.len(), eq(2));

        // Verify URIs are different
        let paths: Vec<&str> = locations.iter().map(|loc| loc.uri.path()).collect();
        assert_that!(paths, contains(ends_with("en.json")));
        assert_that!(paths, contains(ends_with("ja.json")));
    }

    #[rstest]
    fn find_definitions_not_found() {
        let db = I18nDatabaseImpl::default();

        // Search for non-existent key
        let translation = Translation::new(
            &db,
            "en".to_string(),
            "/test/locales/en.json".to_string(),
            HashMap::from([("common.hello".to_string(), "Hello".to_string())]),
            r#"{"common": {"hello": "Hello"}}"#.to_string(),
            HashMap::new(),
            HashMap::new(),
        );

        let key = TransKey::new(&db, "nonexistent.key".to_string());
        let translations = vec![translation];

        let locations = find_definitions(&db, key, &translations, ".");

        // No definitions found
        assert_that!(locations, is_empty());
    }

    #[rstest]
    fn find_definitions_fallback_to_child_key() {
        let db = I18nDatabaseImpl::default();

        // 親キー "nested" は存在せず、子キー "nested.key" と "nested.foo" のみ存在
        let mut key_ranges = HashMap::new();
        key_ranges.insert(
            "nested.key".to_string(),
            SourceRange {
                start: SourcePosition { line: 1, character: 2 },
                end: SourcePosition { line: 1, character: 14 },
            },
        );
        key_ranges.insert(
            "nested.foo".to_string(),
            SourceRange {
                start: SourcePosition { line: 2, character: 2 },
                end: SourcePosition { line: 2, character: 14 },
            },
        );

        let translation = Translation::new(
            &db,
            "en".to_string(),
            "/test/locales/en.json".to_string(),
            HashMap::from([
                ("nested.key".to_string(), "Key Value".to_string()),
                ("nested.foo".to_string(), "Foo Value".to_string()),
            ]),
            r#"{"nested": {"key": "Key Value", "foo": "Foo Value"}}"#.to_string(),
            key_ranges,
            HashMap::new(),
        );

        // "nested" で検索すると、最初の子キー "nested.foo"（アルファベット順）の位置にジャンプ
        let key = TransKey::new(&db, "nested".to_string());
        let translations = vec![translation];

        let locations = find_definitions(&db, key, &translations, ".");

        // 子キーの位置が返される
        assert_that!(locations.len(), eq(1));
        assert_that!(locations[0].uri.path(), ends_with("en.json"));
        // HashMap のイテレーション順序は不定なので、どちらかの位置が返される
        let line = locations[0].range.start.line;
        assert_that!(line, any![eq(1), eq(2)]);
    }

    #[rstest]
    fn find_definitions_exact_match_takes_priority() {
        let db = I18nDatabaseImpl::default();

        // 親キー "nested" と子キー "nested.key" の両方が存在
        let mut key_ranges = HashMap::new();
        key_ranges.insert(
            "nested".to_string(),
            SourceRange {
                start: SourcePosition { line: 0, character: 2 },
                end: SourcePosition { line: 0, character: 10 },
            },
        );
        key_ranges.insert(
            "nested.key".to_string(),
            SourceRange {
                start: SourcePosition { line: 1, character: 2 },
                end: SourcePosition { line: 1, character: 14 },
            },
        );

        let translation = Translation::new(
            &db,
            "en".to_string(),
            "/test/locales/en.json".to_string(),
            HashMap::from([
                ("nested".to_string(), "Parent Value".to_string()),
                ("nested.key".to_string(), "Key Value".to_string()),
            ]),
            r#"{"nested": "Parent Value"}"#.to_string(),
            key_ranges,
            HashMap::new(),
        );

        // "nested" で検索すると、完全一致の "nested" の位置にジャンプ（子キーにはフォールバックしない）
        let key = TransKey::new(&db, "nested".to_string());
        let translations = vec![translation];

        let locations = find_definitions(&db, key, &translations, ".");

        // 完全一致の位置が返される
        assert_that!(locations.len(), eq(1));
        assert_that!(locations[0].range.start.line, eq(0));
        assert_that!(locations[0].range.start.character, eq(2));
    }

    #[rstest]
    fn lsp_range_conversion() {
        let source_range = SourceRange {
            start: SourcePosition { line: 5, character: 10 },
            end: SourcePosition { line: 5, character: 25 },
        };

        let lsp_range = lsp_range_from_source_range(source_range);

        assert_that!(lsp_range.start.line, eq(5));
        assert_that!(lsp_range.start.character, eq(10));
        assert_that!(lsp_range.end.line, eq(5));
        assert_that!(lsp_range.end.character, eq(25));
    }
}
