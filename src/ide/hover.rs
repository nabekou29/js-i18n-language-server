//! Hover implementation

use std::fmt::Write as _;

use crate::db::I18nDatabase;
use crate::ide::plural::find_plural_variants;
use crate::input::translation::Translation;
use crate::interned::TransKey;

/// 子キーの表示で値を切り詰める最大長
const MAX_NESTED_VALUE_LENGTH: usize = 30;

/// 子キーを表示する最大数
const MAX_NESTED_KEYS_DISPLAY: usize = 5;

/// Generate hover content for a translation key
///
/// # ソート順
/// 言語は以下の順序でソートされます：
/// 1. `current_language`（設定されている場合）
/// 2. `primary_languages`（設定順）
/// 3. その他（アルファベット順）
///
/// # 逆方向 prefix マッチ
/// 完全一致がない場合、子キー（例: `nested.key`）のリストを表示します。
/// これにより `t('nested')` で `nested.key` がある場合もホバー情報を表示できます。
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

        // 完全一致
        if let Some(value) = keys.get(key_text) {
            translations_found.push((language, value.clone()));
            continue;
        }

        // plural バリアントをチェック
        let plural_variants = find_plural_variants(key_text, keys);
        if !plural_variants.is_empty() {
            let formatted = format_plural_variants(&plural_variants, key_text);
            translations_found.push((language, formatted));
            continue;
        }

        // 逆方向 prefix マッチ：子キーを収集
        let prefix = format!("{key_text}{key_separator}");
        let nested_keys: Vec<_> = keys.iter().filter(|(k, _)| k.starts_with(&prefix)).collect();

        if !nested_keys.is_empty() {
            // 子キーをフォーマットして表示
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

    // Sort by priority: current_language → primary_languages → alphabetical
    sort_translations_by_priority(&mut translations_found, current_language, primary_languages);

    for (language, value) in translations_found {
        let _ = writeln!(content, "**{language}**: {value}");
    }

    Some(content)
}

/// plural バリアントをフォーマットして表示用文字列を生成
fn format_plural_variants(variants: &[(&str, &str)], base_key: &str) -> String {
    let mut result = String::from("(plural)\n");

    for (key, value) in variants {
        // ベースキーを除いた suffix 部分のみ表示
        let suffix = key.strip_prefix(base_key).unwrap_or(key);
        let truncated_value = truncate_string(value, MAX_NESTED_VALUE_LENGTH);
        let _ = writeln!(result, "  `{suffix}`: {truncated_value}");
    }

    result.trim_end().to_string()
}

/// 子キーをフォーマットして表示用文字列を生成
fn format_nested_keys(nested_keys: &[(&String, &String)], prefix: &str) -> String {
    let mut sorted_keys: Vec<_> = nested_keys.iter().collect();
    sorted_keys.sort_by(|(a, _), (b, _)| a.cmp(b));

    let display_keys: Vec<String> = sorted_keys
        .iter()
        .take(MAX_NESTED_KEYS_DISPLAY)
        .map(|(k, v)| {
            // prefix を除いた相対キー名
            let relative_key = k.strip_prefix(prefix).unwrap_or(k);
            let truncated_value = truncate_string(v, MAX_NESTED_VALUE_LENGTH);
            // キー名をバッククォートで囲む（Markdown 特殊文字のエスケープ）
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

/// 文字列を指定した長さに切り詰める
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len - 3).collect();
        format!("{truncated}...")
    }
}

/// 翻訳結果を優先度順にソート
///
/// ソート順:
/// 1. `current_language`（設定されている場合）
/// 2. `primary_languages`（設定順）
/// 3. その他（アルファベット順）
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

/// 言語の優先度を計算
fn get_language_priority<'a>(
    lang: &'a str,
    current_language: Option<&str>,
    primary_languages: Option<&[String]>,
) -> LanguagePriority<'a> {
    // current_language は最高優先度
    if current_language.is_some_and(|c| c == lang) {
        return LanguagePriority::Current;
    }

    // primary_languages は設定順
    if let Some(primaries) = primary_languages
        && let Some(pos) = primaries.iter().position(|p| p == lang)
    {
        return LanguagePriority::Primary(pos);
    }

    // その他はアルファベット順
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

        // ソート優先度なしの場合はアルファベット順
        let content = generate_hover_content(&db, key, &translations, ".", None, None).unwrap();

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

        let content = generate_hover_content(&db, key, &translations, ".", None, None).unwrap();

        // en のみ含まれている
        assert_that!(content, contains_substring("**en**: Hello"));
        assert_that!(content, not(contains_substring("**ja**")));
    }

    #[rstest]
    fn generate_hover_content_with_nested_children() {
        let db = I18nDatabaseImpl::default();

        // "nested" キーは存在せず、"nested.key" と "nested.foo" が存在
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

        // キーが含まれている
        assert_that!(content, contains_substring("**Translation Key:** `nested`"));

        // 子キーがリスト表示されている（バッククォートで囲まれている）
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

        // アルファベット順にソートされている
        let alpha_pos = content.find("`.alpha`").unwrap();
        let beta_pos = content.find("`.beta`").unwrap();
        let zebra_pos = content.find("`.zebra`").unwrap();
        assert_that!(alpha_pos, lt(beta_pos));
        assert_that!(beta_pos, lt(zebra_pos));
    }

    #[rstest]
    fn generate_hover_content_nested_keys_truncated_value() {
        let db = I18nDatabaseImpl::default();

        // 30文字を超える長い値
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

        // 値が切り詰められて "..." が付いている
        assert_that!(content, contains_substring("..."));
        // 完全な値は含まれていない
        assert_that!(content, not(contains_substring(long_value)));
    }

    #[rstest]
    fn generate_hover_content_nested_keys_max_display() {
        let db = I18nDatabaseImpl::default();

        // 6個の子キーを作成（最大表示数は5）
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

        // "... and 1 more" が表示される
        assert_that!(content, contains_substring("... and 1 more"));
    }

    #[rstest]
    fn test_truncate_string() {
        // 短い文字列はそのまま
        let result1 = truncate_string("hello", 10);
        assert_that!(result1.as_str(), eq("hello"));

        // 制限を超える文字列は切り詰められる
        let result2 = truncate_string("hello world", 8);
        assert_that!(result2.as_str(), eq("hello..."));

        // ちょうど制限と同じ
        let result3 = truncate_string("hello", 5);
        assert_that!(result3.as_str(), eq("hello"));
    }

    #[rstest]
    fn generate_hover_content_with_current_language_priority() {
        let db = I18nDatabaseImpl::default();

        // 3つの言語（アルファベット順: en, ja, zh）
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

        // current_language = "ja" を指定
        let content =
            generate_hover_content(&db, key, &translations, ".", Some("ja"), None).unwrap();

        // ja が最初に表示される
        let ja_pos = content.find("**ja**").unwrap();
        let en_pos = content.find("**en**").unwrap();
        let zh_pos = content.find("**zh**").unwrap();
        assert_that!(ja_pos, lt(en_pos));
        assert_that!(ja_pos, lt(zh_pos));
        // 残りはアルファベット順
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

        // primary_languages = ["zh", "ja"] を指定
        let primary = vec!["zh".to_string(), "ja".to_string()];
        let content =
            generate_hover_content(&db, key, &translations, ".", None, Some(&primary)).unwrap();

        // zh, ja, en の順で表示される
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
        // current が最優先
        let primary = vec!["zh".to_string(), "ja".to_string()];
        let content =
            generate_hover_content(&db, key, &translations, ".", Some("en"), Some(&primary))
                .unwrap();

        // en, zh, ja の順で表示される
        let en_pos = content.find("**en**").unwrap();
        let zh_pos = content.find("**zh**").unwrap();
        let ja_pos = content.find("**ja**").unwrap();
        assert_that!(en_pos, lt(zh_pos));
        assert_that!(zh_pos, lt(ja_pos));
    }

    #[rstest]
    fn generate_hover_content_with_plural_variants() {
        let db = I18nDatabaseImpl::default();

        // "items" キーは存在せず、"items_one" と "items_other" が存在
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

        // キーが含まれている
        assert_that!(content, contains_substring("**Translation Key:** `items`"));

        // plural バリアントが表示されている
        assert_that!(content, contains_substring("(plural)"));
        assert_that!(content, contains_substring("`_one`: {{count}} item"));
        assert_that!(content, contains_substring("`_other`: {{count}} items"));
    }

    #[rstest]
    fn generate_hover_content_with_ordinal_plural_variants() {
        let db = I18nDatabaseImpl::default();

        // ordinal suffix のテスト
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

        // plural バリアントが表示されている
        assert_that!(content, contains_substring("(plural)"));
        assert_that!(content, contains_substring("`_ordinal_one`: {{count}}st"));
        assert_that!(content, contains_substring("`_ordinal_two`: {{count}}nd"));
        assert_that!(content, contains_substring("`_ordinal_few`: {{count}}rd"));
        assert_that!(content, contains_substring("`_ordinal_other`: {{count}}th"));
    }

    #[rstest]
    fn generate_hover_content_exact_match_over_plural() {
        let db = I18nDatabaseImpl::default();

        // "items" キーが完全一致で存在する場合は plural より優先
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

        // 完全一致の値が表示される
        assert_that!(content, contains_substring("**en**: Items (exact)"));
        // plural バリアントは表示されない
        assert_that!(content, not(contains_substring("(plural)")));
    }
}
