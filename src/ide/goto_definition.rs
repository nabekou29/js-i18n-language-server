//! Go to Definition implementation

use tower_lsp::lsp_types::{
    Location,
    Url,
};

use crate::db::I18nDatabase;
use crate::ide::plural::PLURAL_SUFFIXES;
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
/// For parent keys (e.g., `nested`), falls back to the first child key (`nested.key`) if no exact match.
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

        // Try to find a matching range in priority order:
        // 1. Exact match
        // 2. Plural variant fallback
        // 3. Child key prefix fallback
        let range = key_ranges
            .get(key_text.as_str())
            .or_else(|| find_plural_variant_range(key_text, key_ranges))
            .or_else(|| find_child_key_range(key_text, key_separator, key_ranges));

        let Some(range) = range else {
            continue;
        };

        if let Some(location) = create_location(translation.file_path(db), range) {
            locations.push(location);
        }
    }

    locations
}

/// Find the first plural variant range for the given key
fn find_plural_variant_range<'a>(
    key_text: &str,
    key_ranges: &'a std::collections::HashMap<String, SourceRange>,
) -> Option<&'a SourceRange> {
    PLURAL_SUFFIXES.iter().find_map(|suffix| {
        let variant_key = format!("{key_text}{suffix}");
        key_ranges.get(&variant_key)
    })
}

/// Find the first child key range using prefix matching
fn find_child_key_range<'a>(
    key_text: &str,
    separator: &str,
    key_ranges: &'a std::collections::HashMap<String, SourceRange>,
) -> Option<&'a SourceRange> {
    let prefix = format!("{key_text}{separator}");
    key_ranges.iter().find(|(k, _)| k.starts_with(&prefix)).map(|(_, range)| range)
}

/// Create a Location from file path and range
fn create_location(file_path: &str, range: &SourceRange) -> Option<Location> {
    let Ok(uri) = Url::from_file_path(file_path) else {
        tracing::warn!("Failed to create URI from file path: {}", file_path);
        return None;
    };
    Some(Location { uri, range: (*range).into() })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::expect_used, clippy::panic)]
mod tests {
    use std::collections::HashMap;

    use googletest::prelude::*;
    use rstest::*;
    use tower_lsp::lsp_types::Range;

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
            None,
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
            None,
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
            None,
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
            None,
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

        // Parent key "nested" doesn't exist, only child keys "nested.key" and "nested.foo"
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
            None,
            "/test/locales/en.json".to_string(),
            HashMap::from([
                ("nested.key".to_string(), "Key Value".to_string()),
                ("nested.foo".to_string(), "Foo Value".to_string()),
            ]),
            r#"{"nested": {"key": "Key Value", "foo": "Foo Value"}}"#.to_string(),
            key_ranges,
            HashMap::new(),
        );

        // Searching "nested" jumps to the first child key position
        let key = TransKey::new(&db, "nested".to_string());
        let translations = vec![translation];

        let locations = find_definitions(&db, key, &translations, ".");

        assert_that!(locations.len(), eq(1));
        assert_that!(locations[0].uri.path(), ends_with("en.json"));
        // HashMap iteration order is unspecified
        let line = locations[0].range.start.line;
        assert_that!(line, any![eq(1), eq(2)]);
    }

    #[rstest]
    fn find_definitions_exact_match_takes_priority() {
        let db = I18nDatabaseImpl::default();

        // Both parent key "nested" and child key "nested.key" exist
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
            None,
            "/test/locales/en.json".to_string(),
            HashMap::from([
                ("nested".to_string(), "Parent Value".to_string()),
                ("nested.key".to_string(), "Key Value".to_string()),
            ]),
            r#"{"nested": "Parent Value"}"#.to_string(),
            key_ranges,
            HashMap::new(),
        );

        // Searching "nested" jumps to exact match (no fallback to child)
        let key = TransKey::new(&db, "nested".to_string());
        let translations = vec![translation];

        let locations = find_definitions(&db, key, &translations, ".");

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

        let lsp_range: Range = source_range.into();

        assert_that!(lsp_range.start.line, eq(5));
        assert_that!(lsp_range.start.character, eq(10));
        assert_that!(lsp_range.end.line, eq(5));
        assert_that!(lsp_range.end.character, eq(25));
    }

    #[rstest]
    fn find_definitions_fallback_to_plural_variant() {
        let db = I18nDatabaseImpl::default();

        // "items" key doesn't exist, only plural variants "items_one" and "items_other"
        let mut key_ranges = HashMap::new();
        key_ranges.insert(
            "items_one".to_string(),
            SourceRange {
                start: SourcePosition { line: 1, character: 2 },
                end: SourcePosition { line: 1, character: 13 },
            },
        );
        key_ranges.insert(
            "items_other".to_string(),
            SourceRange {
                start: SourcePosition { line: 2, character: 2 },
                end: SourcePosition { line: 2, character: 15 },
            },
        );

        let translation = Translation::new(
            &db,
            "en".to_string(),
            None,
            "/test/locales/en.json".to_string(),
            HashMap::from([
                ("items_one".to_string(), "{{count}} item".to_string()),
                ("items_other".to_string(), "{{count}} items".to_string()),
            ]),
            r#"{"items_one": "{{count}} item", "items_other": "{{count}} items"}"#.to_string(),
            key_ranges,
            HashMap::new(),
        );

        // Searching "items" jumps to the first plural variant
        let key = TransKey::new(&db, "items".to_string());
        let translations = vec![translation];

        let locations = find_definitions(&db, key, &translations, ".");

        // First matching plural variant (_one matches before _other in PLURAL_SUFFIXES order)
        assert_that!(locations.len(), eq(1));
        assert_that!(locations[0].uri.path(), ends_with("en.json"));
        assert_that!(locations[0].range.start.line, eq(1));
    }

    #[rstest]
    fn find_definitions_fallback_to_ordinal_plural_variant() {
        let db = I18nDatabaseImpl::default();

        // "place" key doesn't exist, only ordinal variants
        let mut key_ranges = HashMap::new();
        key_ranges.insert(
            "place_ordinal_one".to_string(),
            SourceRange {
                start: SourcePosition { line: 1, character: 2 },
                end: SourcePosition { line: 1, character: 19 },
            },
        );
        key_ranges.insert(
            "place_ordinal_other".to_string(),
            SourceRange {
                start: SourcePosition { line: 2, character: 2 },
                end: SourcePosition { line: 2, character: 21 },
            },
        );

        let translation = Translation::new(
            &db,
            "en".to_string(),
            None,
            "/test/locales/en.json".to_string(),
            HashMap::from([
                ("place_ordinal_one".to_string(), "{{count}}st".to_string()),
                ("place_ordinal_other".to_string(), "{{count}}th".to_string()),
            ]),
            r#"{"place_ordinal_one": "{{count}}st", "place_ordinal_other": "{{count}}th"}"#
                .to_string(),
            key_ranges,
            HashMap::new(),
        );

        // Searching "place" jumps to ordinal plural variant
        let key = TransKey::new(&db, "place".to_string());
        let translations = vec![translation];

        let locations = find_definitions(&db, key, &translations, ".");

        assert_that!(locations.len(), eq(1));
        assert_that!(locations[0].uri.path(), ends_with("en.json"));
        // _ordinal_one matches first in PLURAL_SUFFIXES order
        assert_that!(locations[0].range.start.line, eq(1));
    }

    #[rstest]
    fn find_definitions_exact_match_over_plural() {
        let db = I18nDatabaseImpl::default();

        // Both "items" key and plural variants exist
        let mut key_ranges = HashMap::new();
        key_ranges.insert(
            "items".to_string(),
            SourceRange {
                start: SourcePosition { line: 0, character: 2 },
                end: SourcePosition { line: 0, character: 9 },
            },
        );
        key_ranges.insert(
            "items_one".to_string(),
            SourceRange {
                start: SourcePosition { line: 1, character: 2 },
                end: SourcePosition { line: 1, character: 13 },
            },
        );
        key_ranges.insert(
            "items_other".to_string(),
            SourceRange {
                start: SourcePosition { line: 2, character: 2 },
                end: SourcePosition { line: 2, character: 15 },
            },
        );

        let translation = Translation::new(
            &db,
            "en".to_string(),
            None,
            "/test/locales/en.json".to_string(),
            HashMap::from([
                ("items".to_string(), "Items (exact)".to_string()),
                ("items_one".to_string(), "{{count}} item".to_string()),
                ("items_other".to_string(), "{{count}} items".to_string()),
            ]),
            r#"{"items": "Items (exact)", "items_one": "{{count}} item"}"#.to_string(),
            key_ranges,
            HashMap::new(),
        );

        // Searching "items" jumps to exact match
        let key = TransKey::new(&db, "items".to_string());
        let translations = vec![translation];

        let locations = find_definitions(&db, key, &translations, ".");

        assert_that!(locations.len(), eq(1));
        assert_that!(locations[0].range.start.line, eq(0));
        assert_that!(locations[0].range.start.character, eq(2));
    }
}
