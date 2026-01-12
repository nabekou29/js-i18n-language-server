//! Hover implementation

use std::fmt::Write as _;

use crate::db::I18nDatabase;
use crate::ide::plural::find_plural_variants;
use crate::input::translation::Translation;
use crate::interned::TransKey;

/// Maximum length for values displayed in child key listings
const MAX_NESTED_VALUE_LENGTH: usize = 30;

/// Maximum number of child keys to display
const MAX_NESTED_KEYS_DISPLAY: usize = 5;

/// Generate hover content for a translation key
///
/// # Sort Order
/// Languages are sorted in the following order:
/// 1. `current_language` (if set)
/// 2. `primary_languages` (in configuration order)
/// 3. Others (alphabetical order)
///
/// # Reverse Prefix Matching
/// When no exact match exists, displays a list of child keys (e.g., `nested.key`).
/// This enables hover information for cases like `t('nested')` when `nested.key` exists.
pub fn generate_hover_content(
    db: &dyn I18nDatabase,
    key: TransKey<'_>,
    translations: &[Translation],
    key_separator: &str,
    current_language: Option<&str>,
    primary_languages: Option<&[String]>,
) -> Option<String> {
    let key_text = key.text(db);

    // Collect translations for this key
    let mut translations_found = Vec::new();

    for translation in translations {
        let keys = translation.keys(db);
        let language = translation.language(db);

        // Exact match
        if let Some(value) = keys.get(key_text) {
            translations_found.push((language, value.clone()));
            continue;
        }

        // Check plural variants
        let plural_variants = find_plural_variants(key_text, keys);
        if !plural_variants.is_empty() {
            let formatted = format_plural_variants(&plural_variants, key_text);
            translations_found.push((language, formatted));
            continue;
        }

        // Reverse prefix match: collect child keys
        let prefix = format!("{key_text}{key_separator}");
        let nested_keys: Vec<_> = keys.iter().filter(|(k, _)| k.starts_with(&prefix)).collect();

        if !nested_keys.is_empty() {
            let nested_display = format_nested_keys(&nested_keys, &prefix);
            translations_found.push((language, nested_display));
        }
    }

    // No translations found
    if translations_found.is_empty() {
        return None;
    }

    // Format as markdown
    let mut content = format!("**Translation Key:** `{key_text}`\n\n");

    // Sort by priority: current_language -> primary_languages -> alphabetical
    sort_translations_by_priority(&mut translations_found, current_language, primary_languages);

    for (language, value) in translations_found {
        let _ = writeln!(content, "**{language}**: {value}");
    }

    Some(content)
}

/// Format plural variants into a display string
fn format_plural_variants(variants: &[(&str, &str)], base_key: &str) -> String {
    let mut result = String::from("(plural)\n");

    for (key, value) in variants {
        // Display only the suffix part after stripping the base key
        let suffix = key.strip_prefix(base_key).unwrap_or(key);
        let truncated_value = truncate_string(value, MAX_NESTED_VALUE_LENGTH);
        let _ = writeln!(result, "  `{suffix}`: {truncated_value}");
    }

    result.trim_end().to_string()
}

/// Format nested child keys into a display string
fn format_nested_keys(nested_keys: &[(&String, &String)], prefix: &str) -> String {
    let mut sorted_keys: Vec<_> = nested_keys.iter().collect();
    sorted_keys.sort_by(|(a, _), (b, _)| a.cmp(b));

    let display_keys: Vec<String> = sorted_keys
        .iter()
        .take(MAX_NESTED_KEYS_DISPLAY)
        .map(|(k, v)| {
            // Relative key name after stripping the prefix
            let relative_key = k.strip_prefix(prefix).unwrap_or(k);
            let truncated_value = truncate_string(v, MAX_NESTED_VALUE_LENGTH);
            // Wrap key name in backticks (escapes Markdown special characters)
            format!("  `.{relative_key}`: {truncated_value}")
        })
        .collect();

    let mut result = format!("{{...}}\n{}", display_keys.join("\n"));

    if nested_keys.len() > MAX_NESTED_KEYS_DISPLAY {
        let remaining = nested_keys.len() - MAX_NESTED_KEYS_DISPLAY;
        let _ = write!(result, "\n  ... and {remaining} more");
    }

    result
}

/// Truncate string to specified length
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len - 3).collect();
        format!("{truncated}...")
    }
}

/// Sort translations by priority
///
/// Sort order:
/// 1. `current_language` (if set)
/// 2. `primary_languages` (in configuration order)
/// 3. Others (alphabetical order)
fn sort_translations_by_priority(
    translations: &mut [(String, String)],
    current_language: Option<&str>,
    primary_languages: Option<&[String]>,
) {
    translations.sort_by(|a, b| {
        let priority_a = get_language_priority(&a.0, current_language, primary_languages);
        let priority_b = get_language_priority(&b.0, current_language, primary_languages);

        match (priority_a, priority_b) {
            (LanguagePriority::Current, LanguagePriority::Current) => std::cmp::Ordering::Equal,
            (LanguagePriority::Current, _) => std::cmp::Ordering::Less,
            (_, LanguagePriority::Current) => std::cmp::Ordering::Greater,
            (LanguagePriority::Primary(a_idx), LanguagePriority::Primary(b_idx)) => {
                a_idx.cmp(&b_idx)
            }
            (LanguagePriority::Primary(_), _) => std::cmp::Ordering::Less,
            (_, LanguagePriority::Primary(_)) => std::cmp::Ordering::Greater,
            (LanguagePriority::Other(a_lang), LanguagePriority::Other(b_lang)) => {
                a_lang.cmp(b_lang)
            }
        }
    });
}

/// Language priority for sorting
#[derive(Debug, Clone, PartialEq, Eq)]
enum LanguagePriority<'a> {
    /// Current language (highest priority)
    Current,
    /// Primary language with its position index
    Primary(usize),
    /// Other language (sorted alphabetically)
    Other(&'a str),
}

/// Calculate language priority for sorting
fn get_language_priority<'a>(
    lang: &'a str,
    current_language: Option<&str>,
    primary_languages: Option<&[String]>,
) -> LanguagePriority<'a> {
    // current_language has highest priority
    if current_language.is_some_and(|c| c == lang) {
        return LanguagePriority::Current;
    }

    // primary_languages are sorted by configuration order
    if let Some(primaries) = primary_languages
        && let Some(pos) = primaries.iter().position(|p| p == lang)
    {
        return LanguagePriority::Primary(pos);
    }

    // Others are sorted alphabetically
    LanguagePriority::Other(lang)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::collections::HashMap;

    use googletest::prelude::*;
    use rstest::*;

    use super::*;
    use crate::db::I18nDatabaseImpl;
    use crate::test_utils::create_translation;

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

        let content = generate_hover_content(&db, key, &translations, ".", None, None);

        assert_that!(content, some(contains_substring("**Translation Key:** `common.hello`")));
        assert_that!(content.as_ref().unwrap(), contains_substring("**en**: Hello"));
    }

    #[rstest]
    fn generate_hover_content_with_multiple_languages() {
        let db = I18nDatabaseImpl::default();

        // Intentionally added in order different from sort order (ja -> en)
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

        // Without sort priority, alphabetical order is used
        let content = generate_hover_content(&db, key, &translations, ".", None, None).unwrap();

        // Key is included
        assert_that!(content, contains_substring("**Translation Key:** `common.hello`"));

        // Both languages are included
        assert_that!(content, contains_substring("**en**: Hello"));
        assert_that!(content, contains_substring("**ja**: こんにちは"));

        // Sorted by language code (en comes before ja)
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

        // Search for non-existent key
        let key = TransKey::new(&db, "nonexistent.key".to_string());
        let translations = vec![translation];

        let content = generate_hover_content(&db, key, &translations, ".", None, None);

        assert_that!(content, none());
    }

    #[rstest]
    fn generate_hover_content_with_empty_translations() {
        let db = I18nDatabaseImpl::default();

        let key = TransKey::new(&db, "common.hello".to_string());
        let translations: Vec<Translation> = vec![];

        let content = generate_hover_content(&db, key, &translations, ".", None, None);

        assert_that!(content, none());
    }

    #[rstest]
    fn generate_hover_content_with_partial_translations() {
        let db = I18nDatabaseImpl::default();

        // en has the key, but ja does not
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

        let content = generate_hover_content(&db, key, &translations, ".", None, None).unwrap();

        // Only en is included
        assert_that!(content, contains_substring("**en**: Hello"));
        assert_that!(content, not(contains_substring("**ja**")));
    }

    #[rstest]
    fn generate_hover_content_with_nested_children() {
        let db = I18nDatabaseImpl::default();

        // "nested" key does not exist, but "nested.key" and "nested.foo" exist
        let translation = create_translation(
            &db,
            "en",
            "/test/locales/en.json",
            HashMap::from([
                ("nested.key".to_string(), "Key Value".to_string()),
                ("nested.foo".to_string(), "Foo Value".to_string()),
            ]),
        );

        let key = TransKey::new(&db, "nested".to_string());
        let translations = vec![translation];

        let content = generate_hover_content(&db, key, &translations, ".", None, None).unwrap();

        // Key is included
        assert_that!(content, contains_substring("**Translation Key:** `nested`"));

        // Child keys are displayed as list (wrapped in backticks)
        assert_that!(content, contains_substring("{...}"));
        assert_that!(content, contains_substring("`.foo`: Foo Value"));
        assert_that!(content, contains_substring("`.key`: Key Value"));
    }

    #[rstest]
    fn generate_hover_content_nested_keys_are_sorted() {
        let db = I18nDatabaseImpl::default();

        let translation = create_translation(
            &db,
            "en",
            "/test/locales/en.json",
            HashMap::from([
                ("nested.zebra".to_string(), "Z".to_string()),
                ("nested.alpha".to_string(), "A".to_string()),
                ("nested.beta".to_string(), "B".to_string()),
            ]),
        );

        let key = TransKey::new(&db, "nested".to_string());
        let translations = vec![translation];

        let content = generate_hover_content(&db, key, &translations, ".", None, None).unwrap();

        // Sorted alphabetically
        let alpha_pos = content.find("`.alpha`").unwrap();
        let beta_pos = content.find("`.beta`").unwrap();
        let zebra_pos = content.find("`.zebra`").unwrap();
        assert_that!(alpha_pos, lt(beta_pos));
        assert_that!(beta_pos, lt(zebra_pos));
    }

    #[rstest]
    fn generate_hover_content_nested_keys_truncated_value() {
        let db = I18nDatabaseImpl::default();

        // Long value exceeding 30 characters
        let long_value = "This is a very long translation value that exceeds the limit";

        let translation = create_translation(
            &db,
            "en",
            "/test/locales/en.json",
            HashMap::from([("nested.long".to_string(), long_value.to_string())]),
        );

        let key = TransKey::new(&db, "nested".to_string());
        let translations = vec![translation];

        let content = generate_hover_content(&db, key, &translations, ".", None, None).unwrap();

        // Value is truncated with "..."
        assert_that!(content, contains_substring("..."));
        // Full value is not included
        assert_that!(content, not(contains_substring(long_value)));
    }

    #[rstest]
    fn generate_hover_content_nested_keys_max_display() {
        let db = I18nDatabaseImpl::default();

        // Create 6 child keys (max display is 5)
        let translation = create_translation(
            &db,
            "en",
            "/test/locales/en.json",
            HashMap::from([
                ("nested.a".to_string(), "A".to_string()),
                ("nested.b".to_string(), "B".to_string()),
                ("nested.c".to_string(), "C".to_string()),
                ("nested.d".to_string(), "D".to_string()),
                ("nested.e".to_string(), "E".to_string()),
                ("nested.f".to_string(), "F".to_string()),
            ]),
        );

        let key = TransKey::new(&db, "nested".to_string());
        let translations = vec![translation];

        let content = generate_hover_content(&db, key, &translations, ".", None, None).unwrap();

        // "... and 1 more" is displayed
        assert_that!(content, contains_substring("... and 1 more"));
    }

    #[rstest]
    fn test_truncate_string() {
        // Short string remains unchanged
        let result1 = truncate_string("hello", 10);
        assert_that!(result1.as_str(), eq("hello"));

        // String exceeding limit is truncated
        let result2 = truncate_string("hello world", 8);
        assert_that!(result2.as_str(), eq("hello..."));

        // String exactly at limit
        let result3 = truncate_string("hello", 5);
        assert_that!(result3.as_str(), eq("hello"));
    }

    #[rstest]
    fn generate_hover_content_with_current_language_priority() {
        let db = I18nDatabaseImpl::default();

        // Three languages (alphabetical order: en, ja, zh)
        let en_translation = create_translation(
            &db,
            "en",
            "/test/locales/en.json",
            HashMap::from([("key".to_string(), "English".to_string())]),
        );
        let ja_translation = create_translation(
            &db,
            "ja",
            "/test/locales/ja.json",
            HashMap::from([("key".to_string(), "日本語".to_string())]),
        );
        let zh_translation = create_translation(
            &db,
            "zh",
            "/test/locales/zh.json",
            HashMap::from([("key".to_string(), "中文".to_string())]),
        );

        let key = TransKey::new(&db, "key".to_string());
        let translations = vec![en_translation, ja_translation, zh_translation];

        // Specify current_language = "ja"
        let content =
            generate_hover_content(&db, key, &translations, ".", Some("ja"), None).unwrap();

        // ja is displayed first
        let ja_pos = content.find("**ja**").unwrap();
        let en_pos = content.find("**en**").unwrap();
        let zh_pos = content.find("**zh**").unwrap();
        assert_that!(ja_pos, lt(en_pos));
        assert_that!(ja_pos, lt(zh_pos));
        // Remaining are in alphabetical order
        assert_that!(en_pos, lt(zh_pos));
    }

    #[rstest]
    fn generate_hover_content_with_primary_languages() {
        let db = I18nDatabaseImpl::default();

        let en_translation = create_translation(
            &db,
            "en",
            "/test/locales/en.json",
            HashMap::from([("key".to_string(), "English".to_string())]),
        );
        let ja_translation = create_translation(
            &db,
            "ja",
            "/test/locales/ja.json",
            HashMap::from([("key".to_string(), "日本語".to_string())]),
        );
        let zh_translation = create_translation(
            &db,
            "zh",
            "/test/locales/zh.json",
            HashMap::from([("key".to_string(), "中文".to_string())]),
        );

        let key = TransKey::new(&db, "key".to_string());
        let translations = vec![en_translation, ja_translation, zh_translation];

        // Specify primary_languages = ["zh", "ja"]
        let primary = vec!["zh".to_string(), "ja".to_string()];
        let content =
            generate_hover_content(&db, key, &translations, ".", None, Some(&primary)).unwrap();

        // Displayed in order: zh, ja, en
        let zh_pos = content.find("**zh**").unwrap();
        let ja_pos = content.find("**ja**").unwrap();
        let en_pos = content.find("**en**").unwrap();
        assert_that!(zh_pos, lt(ja_pos));
        assert_that!(ja_pos, lt(en_pos));
    }

    #[rstest]
    fn generate_hover_content_current_overrides_primary() {
        let db = I18nDatabaseImpl::default();

        let en_translation = create_translation(
            &db,
            "en",
            "/test/locales/en.json",
            HashMap::from([("key".to_string(), "English".to_string())]),
        );
        let ja_translation = create_translation(
            &db,
            "ja",
            "/test/locales/ja.json",
            HashMap::from([("key".to_string(), "日本語".to_string())]),
        );
        let zh_translation = create_translation(
            &db,
            "zh",
            "/test/locales/zh.json",
            HashMap::from([("key".to_string(), "中文".to_string())]),
        );

        let key = TransKey::new(&db, "key".to_string());
        let translations = vec![en_translation, ja_translation, zh_translation];

        // current_language = "en", primary_languages = ["zh", "ja"]
        // current has highest priority
        let primary = vec!["zh".to_string(), "ja".to_string()];
        let content =
            generate_hover_content(&db, key, &translations, ".", Some("en"), Some(&primary))
                .unwrap();

        // Displayed in order: en, zh, ja
        let en_pos = content.find("**en**").unwrap();
        let zh_pos = content.find("**zh**").unwrap();
        let ja_pos = content.find("**ja**").unwrap();
        assert_that!(en_pos, lt(zh_pos));
        assert_that!(zh_pos, lt(ja_pos));
    }

    #[rstest]
    fn generate_hover_content_with_plural_variants() {
        let db = I18nDatabaseImpl::default();

        // "items" key does not exist, but "items_one" and "items_other" exist
        let translation = create_translation(
            &db,
            "en",
            "/test/locales/en.json",
            HashMap::from([
                ("items_one".to_string(), "{{count}} item".to_string()),
                ("items_other".to_string(), "{{count}} items".to_string()),
            ]),
        );

        let key = TransKey::new(&db, "items".to_string());
        let translations = vec![translation];

        let content = generate_hover_content(&db, key, &translations, ".", None, None).unwrap();

        // Key is included
        assert_that!(content, contains_substring("**Translation Key:** `items`"));

        // Plural variants are displayed
        assert_that!(content, contains_substring("(plural)"));
        assert_that!(content, contains_substring("`_one`: {{count}} item"));
        assert_that!(content, contains_substring("`_other`: {{count}} items"));
    }

    #[rstest]
    fn generate_hover_content_with_ordinal_plural_variants() {
        let db = I18nDatabaseImpl::default();

        // Test ordinal suffixes
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

        let key = TransKey::new(&db, "place".to_string());
        let translations = vec![translation];

        let content = generate_hover_content(&db, key, &translations, ".", None, None).unwrap();

        // Plural variants are displayed
        assert_that!(content, contains_substring("(plural)"));
        assert_that!(content, contains_substring("`_ordinal_one`: {{count}}st"));
        assert_that!(content, contains_substring("`_ordinal_two`: {{count}}nd"));
        assert_that!(content, contains_substring("`_ordinal_few`: {{count}}rd"));
        assert_that!(content, contains_substring("`_ordinal_other`: {{count}}th"));
    }

    #[rstest]
    fn generate_hover_content_exact_match_over_plural() {
        let db = I18nDatabaseImpl::default();

        // When "items" key exists as exact match, it takes priority over plural
        let translation = create_translation(
            &db,
            "en",
            "/test/locales/en.json",
            HashMap::from([
                ("items".to_string(), "Items (exact)".to_string()),
                ("items_one".to_string(), "{{count}} item".to_string()),
                ("items_other".to_string(), "{{count}} items".to_string()),
            ]),
        );

        let key = TransKey::new(&db, "items".to_string());
        let translations = vec![translation];

        let content = generate_hover_content(&db, key, &translations, ".", None, None).unwrap();

        // Exact match value is displayed
        assert_that!(content, contains_substring("**en**: Items (exact)"));
        // Plural variants are not displayed
        assert_that!(content, not(contains_substring("(plural)")));
    }
}
