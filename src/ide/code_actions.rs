//! Code Action 生成モジュール

use std::cmp::Ordering;
use std::collections::HashSet;

use jsonc_parser::ParseOptions;
use jsonc_parser::cst::{
    CstInputValue,
    CstRootNode,
};
use tower_lsp::lsp_types::{
    CodeActionOrCommand,
    Command,
    Diagnostic,
    NumberOrString,
    Position,
    Range,
};

use crate::db::I18nDatabase;
use crate::input::translation::Translation;

/// キー挿入結果（CST ベース）
///
/// フォーマットを保持したまま JSON を変更した結果を返します。
#[derive(Debug, Clone)]
pub struct KeyInsertionResult {
    /// 変更後の JSON テキスト全体
    pub new_text: String,
    /// 値入力位置（カーソルを配置する範囲）
    pub cursor_range: Range,
}

/// 診断から `missing_languages` を抽出
#[must_use]
pub fn extract_missing_languages(diagnostics: &[Diagnostic]) -> HashSet<String> {
    diagnostics
        .iter()
        .filter(|d| {
            matches!(
                &d.code,
                Some(NumberOrString::String(s)) if s == "missing-translation"
            )
        })
        .filter_map(|d| d.data.as_ref())
        .filter_map(|data| data.get("missing_languages"))
        .filter_map(|v| v.as_array())
        .flat_map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)))
        .collect()
}

/// Code Action (Command) を生成（全言語対象）
///
/// # Arguments
/// * `key` - 翻訳キー
/// * `all_languages` - すべての利用可能な言語
/// * `missing_languages` - 不足している言語（診断から取得）
/// * `primary_language` - 優先表示する言語
///
/// # Returns
/// ソートされた Code Action のリスト
/// - 優先言語 (primary) が先頭
/// - 次に不足している言語 (missing)
/// - 最後にその他の言語
#[must_use]
#[allow(clippy::implicit_hasher)]
pub fn generate_code_actions(
    key: &str,
    all_languages: &[String],
    missing_languages: &HashSet<String>,
    primary_language: Option<&str>,
) -> Vec<CodeActionOrCommand> {
    let mut languages: Vec<(String, bool, bool)> = all_languages
        .iter()
        .map(|lang| {
            let is_primary = primary_language == Some(lang.as_str());
            let is_missing = missing_languages.contains(lang);
            (lang.clone(), is_primary, is_missing)
        })
        .collect();

    // ソート: primary > missing > others
    languages.sort_by(|a, b| match (a.1, b.1) {
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        _ => match (a.2, b.2) {
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            _ => Ordering::Equal,
        },
    });

    languages
        .into_iter()
        .map(|(lang, _, is_missing)| {
            // キーの存在有無でメッセージを変える
            let title = if is_missing {
                format!("Add translation for {lang}")
            } else {
                format!("Edit translation for {lang}")
            };
            CodeActionOrCommand::Command(Command {
                title,
                command: "i18n.editTranslation".to_string(),
                arguments: Some(vec![
                    serde_json::Value::String(lang),
                    serde_json::Value::String(key.to_string()),
                ]),
            })
        })
        .collect()
}

/// JSON ファイルへのキー挿入（CST ベース）
///
/// ネストしたキー（例: `common.hello`）にも対応。
/// jsonc-parser の CST 機能を使い、フォーマットを保持したまま
/// キーを追加します。
///
/// # Arguments
/// * `db` - Salsa データベース
/// * `translation` - 対象の翻訳ファイル
/// * `key` - 追加するキー（ドット区切りなど）
/// * `separator` - キーのセパレータ（例: "."）
///
/// # Returns
/// 成功時は `Some(KeyInsertionResult)`、パース失敗時は `None`
#[must_use]
pub fn insert_key_to_json(
    db: &dyn I18nDatabase,
    translation: &Translation,
    key: &str,
    separator: &str,
) -> Option<KeyInsertionResult> {
    let json_text = translation.json_text(db);
    insert_key_to_json_text(json_text, key, separator)
}

/// JSON テキストへのキー挿入（CST ベース）
///
/// テスト用に `json_text` を直接受け取るバージョン。
#[must_use]
pub fn insert_key_to_json_text(
    json_text: &str,
    key: &str,
    separator: &str,
) -> Option<KeyInsertionResult> {
    // CST でパース
    let root = CstRootNode::parse(json_text, &ParseOptions::default()).ok()?;
    let root_obj = root.object_value_or_set();

    // キーをセパレータで分割
    let key_parts: Vec<&str> = key.split(separator).collect();

    // ネストしたオブジェクトを辿りながら作成
    let mut current_obj = root_obj;
    for (i, part) in key_parts.iter().enumerate() {
        if i == key_parts.len() - 1 {
            // 最後のキー: 空文字列を追加
            current_obj.append(part, CstInputValue::String(String::new()));
        } else {
            // 中間キー: オブジェクトを取得または作成
            current_obj = current_obj.object_value_or_set(part);
        }
    }

    // 結果を文字列に変換
    let new_text = root.to_string();

    // カーソル位置を計算（追加したキーの値の位置を検索）
    let cursor_range = find_cursor_position(&new_text, key, separator)?;

    Some(KeyInsertionResult { new_text, cursor_range })
}

/// 挿入後のカーソル位置を計算
///
/// 追加したキーの空文字列 `""` の中にカーソルを配置する位置を返します。
#[allow(clippy::cast_possible_truncation)] // 翻訳JSONが42億行を超えることはない
fn find_cursor_position(json_text: &str, key: &str, separator: &str) -> Option<Range> {
    // キーの最後の部分を取得
    let leaf_key = key.split(separator).last()?;

    // パターンを検索: "leaf_key": ""
    let pattern = format!("\"{leaf_key}\": \"\"");

    // 最後の出現位置を探す（新しく追加されたものが最後にある可能性が高い）
    let pos = json_text.rfind(&pattern)?;

    // 行と列を計算
    let before = &json_text[..pos];
    let line = before.matches('\n').count() as u32;
    let last_newline = before.rfind('\n').map_or(0, |i| i + 1);

    // カーソル位置: `""` の中（閉じクォートの手前）
    // pattern = `"leaf_key": ""`
    // offset = 1(") + leaf_key.len() + 1(") + 1(:) + 1( ) + 1(") = leaf_key.len() + 5
    let col_start = (pos - last_newline + leaf_key.len() + 5) as u32;

    Some(Range {
        start: Position { line, character: col_start },
        end: Position { line, character: col_start },
    })
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::iter_on_single_items,
    clippy::redundant_closure_for_method_calls
)]
mod tests {
    use googletest::prelude::*;

    use super::*;

    #[googletest::test]
    fn test_extract_missing_languages() {
        let diagnostics = vec![Diagnostic {
            code: Some(NumberOrString::String("missing-translation".to_string())),
            data: Some(serde_json::json!({
                "key": "common.hello",
                "missing_languages": ["ja", "zh"]
            })),
            ..Default::default()
        }];

        let result = extract_missing_languages(&diagnostics);

        expect_that!(result, len(eq(2)));
        expect_that!(result, contains(eq(&"ja".to_string())));
        expect_that!(result, contains(eq(&"zh".to_string())));
    }

    #[googletest::test]
    fn test_extract_missing_languages_empty() {
        let diagnostics = vec![Diagnostic {
            code: Some(NumberOrString::String("other-diagnostic".to_string())),
            data: None,
            ..Default::default()
        }];

        let result = extract_missing_languages(&diagnostics);

        expect_that!(result, is_empty());
    }

    #[googletest::test]
    fn test_generate_code_actions_basic() {
        let all_languages = vec!["en".to_string(), "ja".to_string()];
        let missing_languages: HashSet<String> = HashSet::new();

        let actions =
            generate_code_actions("common.hello", &all_languages, &missing_languages, None);

        expect_that!(actions, len(eq(2)));
    }

    #[googletest::test]
    fn test_generate_code_actions_with_primary() {
        let all_languages = vec!["en".to_string(), "ja".to_string(), "zh".to_string()];
        let missing_languages: HashSet<String> = HashSet::new();

        let actions =
            generate_code_actions("common.hello", &all_languages, &missing_languages, Some("ja"));

        expect_that!(actions, len(eq(3)));

        // primary language (ja) が先頭
        if let CodeActionOrCommand::Command(cmd) = &actions[0] {
            expect_that!(cmd.title, contains_substring("ja"));
        }
    }

    #[googletest::test]
    fn test_generate_code_actions_with_missing() {
        let all_languages = vec!["en".to_string(), "ja".to_string(), "zh".to_string()];
        let missing_languages: HashSet<String> = ["zh"].iter().map(|s| s.to_string()).collect();

        let actions =
            generate_code_actions("common.hello", &all_languages, &missing_languages, None);

        expect_that!(actions, len(eq(3)));

        // missing language (zh) が先頭
        if let CodeActionOrCommand::Command(cmd) = &actions[0] {
            expect_that!(cmd.title, contains_substring("zh"));
        }
    }

    #[googletest::test]
    fn test_generate_code_actions_priority_order() {
        let all_languages = vec!["en".to_string(), "ja".to_string(), "zh".to_string()];
        let missing_languages: HashSet<String> = ["zh"].iter().map(|s| s.to_string()).collect();

        // primary: ja, missing: zh, other: en
        let actions =
            generate_code_actions("common.hello", &all_languages, &missing_languages, Some("ja"));

        // 順序: ja (primary) > zh (missing) > en (other)
        if let CodeActionOrCommand::Command(cmd) = &actions[0] {
            expect_that!(cmd.title, contains_substring("ja"));
        }
        if let CodeActionOrCommand::Command(cmd) = &actions[1] {
            expect_that!(cmd.title, contains_substring("zh"));
        }
        if let CodeActionOrCommand::Command(cmd) = &actions[2] {
            expect_that!(cmd.title, contains_substring("en"));
        }
    }

    #[googletest::test]
    fn test_insert_key_flat() {
        // フラットなキーの挿入テスト
        let json = r#"{
  "hello": "world"
}"#;

        let result =
            insert_key_to_json_text(json, "goodbye", ".").expect("insertion should succeed");

        // 新しいキーが追加されていることを確認
        expect_that!(result.new_text, contains_substring("\"goodbye\""));
        expect_that!(result.new_text, contains_substring("\"goodbye\": \"\""));
        // 既存のキーが保持されていることを確認
        expect_that!(result.new_text, contains_substring("\"hello\": \"world\""));
    }

    #[googletest::test]
    fn test_insert_key_nested_new_parent() {
        // 親キーが存在しないネストしたキーの挿入テスト
        let json = r#"{
  "hello": "world"
}"#;

        let result = insert_key_to_json_text(json, "common.greeting", ".")
            .expect("insertion should succeed");

        // ネスト構造が作成されていることを確認
        expect_that!(result.new_text, contains_substring("\"common\""));
        expect_that!(result.new_text, contains_substring("\"greeting\""));
        expect_that!(result.new_text, contains_substring("\"greeting\": \"\""));
    }

    #[googletest::test]
    fn test_insert_key_nested_existing_parent() {
        // 親キーが存在するネストしたキーの挿入テスト
        let json = r#"{
  "common": {
    "hello": "こんにちは"
  }
}"#;

        let result =
            insert_key_to_json_text(json, "common.goodbye", ".").expect("insertion should succeed");

        // 既存の親オブジェクト内に追加されていることを確認
        expect_that!(result.new_text, contains_substring("\"goodbye\": \"\""));
        // 既存のキーが保持されていることを確認
        expect_that!(result.new_text, contains_substring("\"hello\": \"こんにちは\""));
    }

    #[googletest::test]
    fn test_insert_key_preserves_formatting() {
        // フォーマット（インデント）が保持されることを確認
        let json = r#"{
    "existing": "value"
}"#;

        let result = insert_key_to_json_text(json, "new", ".").expect("insertion should succeed");

        // 4スペースインデントが保持されていることを確認
        expect_that!(result.new_text, contains_substring("    \"existing\""));
    }

    #[googletest::test]
    fn test_insert_key_cursor_position() {
        // カーソル位置が正しく計算されることを確認
        let json = r#"{"hello": "world"}"#;

        let result = insert_key_to_json_text(json, "new", ".").expect("insertion should succeed");

        // カーソル位置が設定されていることを確認（空文字列の中）
        expect_that!(result.cursor_range.start.line, ge(0));
        expect_that!(result.cursor_range.start.character, ge(0));
    }
}
