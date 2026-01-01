//! Hover implementation

use std::fmt::Write as _;

use crate::db::I18nDatabase;
use crate::input::translation::Translation;
use crate::interned::TransKey;

/// 子キーの表示で値を切り詰める最大長
const MAX_NESTED_VALUE_LENGTH: usize = 30;

/// 子キーを表示する最大数
const MAX_NESTED_KEYS_DISPLAY: usize = 5;

/// Generate hover content for a translation key
///
/// # 逆方向 prefix マッチ
/// 完全一致がない場合、子キー（例: `nested.key`）のリストを表示します。
/// これにより `t('nested')` で `nested.key` がある場合もホバー情報を表示できます。
pub fn generate_hover_content(
    db: &dyn I18nDatabase,
    key: TransKey<'_>,
    translations: &[Translation],
    key_separator: &str,
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

    // Sort by language code
    translations_found.sort_by(|a, b| a.0.cmp(&b.0));

    for (language, value) in translations_found {
        let _ = writeln!(content, "**{language}**: {value}");
    }

    Some(content)
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

        let content = generate_hover_content(&db, key, &translations, ".");

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

        let content = generate_hover_content(&db, key, &translations, ".").unwrap();

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

        let content = generate_hover_content(&db, key, &translations, ".");

        assert_that!(content, none());
    }

    #[rstest]
    fn generate_hover_content_with_empty_translations() {
        let db = I18nDatabaseImpl::default();

        let key = TransKey::new(&db, "common.hello".to_string());
        let translations: Vec<Translation> = vec![];

        let content = generate_hover_content(&db, key, &translations, ".");

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

        let content = generate_hover_content(&db, key, &translations, ".").unwrap();

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

        let content = generate_hover_content(&db, key, &translations, ".").unwrap();

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

        let content = generate_hover_content(&db, key, &translations, ".").unwrap();

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

        let content = generate_hover_content(&db, key, &translations, ".").unwrap();

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

        let content = generate_hover_content(&db, key, &translations, ".").unwrap();

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
}
