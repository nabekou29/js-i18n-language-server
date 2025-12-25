//! 翻訳ファイル入力定義

use std::collections::{
    HashMap,
    HashSet,
};
use std::path::Path;
use std::sync::LazyLock;

use serde_json::Value;

use crate::types::{
    SourcePosition,
    SourceRange,
};

/// RFC 5646 language codes
/// Based on <http://tools.ietf.org/html/rfc5646>
static LANGUAGE_CODES: LazyLock<HashSet<String>> = LazyLock::new(|| {
    [
        "af",
        "af-ZA",
        "ar",
        "ar-AE",
        "ar-BH",
        "ar-DZ",
        "ar-EG",
        "ar-IQ",
        "ar-JO",
        "ar-KW",
        "ar-LB",
        "ar-LY",
        "ar-MA",
        "ar-OM",
        "ar-QA",
        "ar-SA",
        "ar-SY",
        "ar-TN",
        "ar-YE",
        "az",
        "az-AZ",
        "az-Cyrl-AZ",
        "be",
        "be-BY",
        "bg",
        "bg-BG",
        "bs-BA",
        "ca",
        "ca-ES",
        "cs",
        "cs-CZ",
        "cy",
        "cy-GB",
        "da",
        "da-DK",
        "de",
        "de-AT",
        "de-CH",
        "de-DE",
        "de-LI",
        "de-LU",
        "dv",
        "dv-MV",
        "el",
        "el-GR",
        "en",
        "en-AU",
        "en-BZ",
        "en-CA",
        "en-CB",
        "en-GB",
        "en-IE",
        "en-JM",
        "en-NZ",
        "en-PH",
        "en-TT",
        "en-US",
        "en-ZA",
        "en-ZW",
        "eo",
        "es",
        "es-AR",
        "es-BO",
        "es-CL",
        "es-CO",
        "es-CR",
        "es-DO",
        "es-EC",
        "es-ES",
        "es-GT",
        "es-HN",
        "es-MX",
        "es-NI",
        "es-PA",
        "es-PE",
        "es-PR",
        "es-PY",
        "es-SV",
        "es-UY",
        "es-VE",
        "et",
        "et-EE",
        "eu",
        "eu-ES",
        "fa",
        "fa-IR",
        "fi",
        "fi-FI",
        "fo",
        "fo-FO",
        "fr",
        "fr-BE",
        "fr-CA",
        "fr-CH",
        "fr-FR",
        "fr-LU",
        "fr-MC",
        "gl",
        "gl-ES",
        "gu",
        "gu-IN",
        "he",
        "he-IL",
        "hi",
        "hi-IN",
        "hr",
        "hr-BA",
        "hr-HR",
        "hu",
        "hu-HU",
        "hy",
        "hy-AM",
        "id",
        "id-ID",
        "is",
        "is-IS",
        "it",
        "it-CH",
        "it-IT",
        "ja",
        "ja-JP",
        "ka",
        "ka-GE",
        "kk",
        "kk-KZ",
        "kn",
        "kn-IN",
        "ko",
        "ko-KR",
        "kok",
        "kok-IN",
        "ky",
        "ky-KG",
        "lt",
        "lt-LT",
        "lv",
        "lv-LV",
        "mi",
        "mi-NZ",
        "mk",
        "mk-MK",
        "mn",
        "mn-MN",
        "mr",
        "mr-IN",
        "ms",
        "ms-BN",
        "ms-MY",
        "mt",
        "mt-MT",
        "nb",
        "nb-NO",
        "nl",
        "nl-BE",
        "nl-NL",
        "nn-NO",
        "ns",
        "ns-ZA",
        "pa",
        "pa-IN",
        "pl",
        "pl-PL",
        "ps",
        "ps-AR",
        "pt",
        "pt-BR",
        "pt-PT",
        "qu",
        "qu-BO",
        "qu-EC",
        "qu-PE",
        "ro",
        "ro-RO",
        "ru",
        "ru-RU",
        "sa",
        "sa-IN",
        "se",
        "se-FI",
        "se-NO",
        "se-SE",
        "sk",
        "sk-SK",
        "sl",
        "sl-SI",
        "sq",
        "sq-AL",
        "sr-BA",
        "sr-Cyrl-BA",
        "sr-SP",
        "sr-Cyrl-SP",
        "sv",
        "sv-FI",
        "sv-SE",
        "sw",
        "sw-KE",
        "syr",
        "syr-SY",
        "ta",
        "ta-IN",
        "te",
        "te-IN",
        "th",
        "th-TH",
        "tl",
        "tl-PH",
        "tn",
        "tn-ZA",
        "tr",
        "tr-TR",
        "tt",
        "tt-RU",
        "ts",
        "uk",
        "uk-UA",
        "ur",
        "ur-PK",
        "uz",
        "uz-UZ",
        "uz-Cyrl-UZ",
        "vi",
        "vi-VN",
        "xh",
        "xh-ZA",
        "zh",
        "zh-CN",
        "zh-HK",
        "zh-MO",
        "zh-SG",
        "zh-TW",
        "zu",
        "zu-ZA",
    ]
    .iter()
    .flat_map(|code| {
        let code = (*code).to_string();
        let normalized = normalize_language_code(&code);
        vec![code, normalized]
    })
    .collect()
});

/// Normalize language code (lowercase and replace - with _)
fn normalize_language_code(code: &str) -> String {
    code.to_lowercase().replace('-', "_")
}

/// Detect language from file path heuristically
///
/// Splits the path by '/' and '.', then searches backwards for a part
/// that matches a known language code.
///
/// # Examples
/// - `locales/en.json` → `en`
/// - `messages/ja-JP.json` → `ja-JP`
/// - `translations/en_US/common.json` → `en_US`
///
/// # Arguments
/// * `file_path` - File path to detect language from
///
/// # Returns
/// Detected language code or "unknown"
fn detect_language_from_path(file_path: &Path) -> String {
    // Split path by '/' and '.'
    let path_str = file_path.to_string_lossy();
    let parts: Vec<&str> = path_str.split(&['/', '.']).collect();

    // Search backwards for a known language code
    for part in parts.iter().rev() {
        let normalized = normalize_language_code(part);
        if LANGUAGE_CODES.contains(&normalized) || LANGUAGE_CODES.contains(*part) {
            return (*part).to_string();
        }
    }

    "unknown".to_string()
}

/// 翻訳データを表す Salsa Input
#[salsa::input]
pub struct Translation {
    /// 言語コード（例: "en", "ja"）
    pub language: String,

    /// ファイルパス
    #[returns(ref)]
    pub file_path: String,

    /// フラット化された翻訳キーマップ
    /// 例: { "common.hello": "Hello", "common.goodbye": "Goodbye" }
    #[returns(ref)]
    pub keys: HashMap<String, String>,

    /// JSON ファイルの元テキスト
    #[returns(ref)]
    pub json_text: String,

    /// キーと位置情報のマッピング
    /// 例: { "common.hello": SourceRange { start: (2, 5), end: (2, 17) } }
    #[returns(ref)]
    pub key_ranges: HashMap<String, SourceRange>,

    /// 値と位置情報のマッピング
    /// 例: { "common.hello": SourceRange { start: (2, 14), end: (2, 21) } }
    #[returns(ref)]
    pub value_ranges: HashMap<String, SourceRange>,
}

/// JSON をフラット化する
///
/// ネストされたJSONオブジェクトを、ドット区切りのキーを持つフラットなマップに変換します。
///
/// # Arguments
/// * `json` - JSON Value
/// * `separator` - キー区切り文字（通常は "." または "_"）
/// * `prefix` - プレフィックス（再帰用、通常は None で呼び出す）
///
/// # Returns
/// フラット化されたキーマップ
///
/// # Examples
/// ```
/// use serde_json::json;
/// use js_i18n_language_server::input::translation::flatten_json;
///
/// let json = json!({
///     "common": {
///         "hello": "Hello",
///         "goodbye": "Goodbye"
///     }
/// });
///
/// let flattened = flatten_json(&json, ".", None);
/// assert_eq!(flattened.get("common.hello"), Some(&"Hello".to_string()));
/// assert_eq!(flattened.get("common.goodbye"), Some(&"Goodbye".to_string()));
/// ```
#[must_use]
pub fn flatten_json(
    json: &Value,
    separator: &str,
    prefix: Option<&str>,
) -> HashMap<String, String> {
    let mut result = HashMap::new();

    if let Value::Object(map) = json {
        for (key, value) in map {
            let full_key = prefix.map_or_else(|| key.clone(), |p| format!("{p}{separator}{key}"));

            match value {
                Value::String(s) => {
                    result.insert(full_key, s.clone());
                }
                Value::Object(_) => {
                    // 再帰的にフラット化
                    let nested = flatten_json(value, separator, Some(&full_key));
                    result.extend(nested);
                }
                _ => {
                    // その他の型は文字列に変換
                    result.insert(full_key, value.to_string());
                }
            }
        }
    }

    result
}

/// JSON ファイルからキーと値の位置情報のマッピングを抽出
///
/// tree-sitter-json を使って JSON をパースし、各キーと値の位置情報を取得します。
///
/// # Arguments
/// * `json_text` - JSON ファイルの元テキスト
/// * `separator` - キー区切り文字（通常は "." または "_"）
///
/// # Returns
/// (キーと位置情報のマッピング, 値と位置情報のマッピング) のタプル
///
/// # Examples
/// ```json
/// {
///   "common": {
///     "hello": "Hello"
///   }
/// }
/// ```
/// 上記の JSON の場合、キーの位置情報（`"hello"` の位置）と値の位置情報（`"Hello"` の位置）が
/// それぞれマッピングされます。
#[must_use]
#[allow(dead_code)]
pub fn extract_key_value_ranges(
    json_text: &str,
    separator: &str,
) -> (HashMap<String, SourceRange>, HashMap<String, SourceRange>) {
    let mut key_ranges = HashMap::new();
    let mut value_ranges = HashMap::new();

    // tree-sitter でパース
    let mut parser = tree_sitter::Parser::new();
    let Ok(()) = parser.set_language(&tree_sitter_json::LANGUAGE.into()) else {
        tracing::warn!("Failed to set tree-sitter-json language");
        return (key_ranges, value_ranges);
    };

    let Some(tree) = parser.parse(json_text, None) else {
        tracing::warn!("Failed to parse JSON with tree-sitter");
        return (key_ranges, value_ranges);
    };

    let root_node = tree.root_node();

    // 再帰的にキーと値の位置情報を抽出
    extract_keys_from_node(
        root_node,
        json_text.as_bytes(),
        separator,
        None,
        &mut key_ranges,
        &mut value_ranges,
    );

    (key_ranges, value_ranges)
}

/// ノードから再帰的にキーと値の位置情報を抽出するヘルパー関数
///
/// # Arguments
/// * `node` - 現在のノード
/// * `source` - JSON ソーステキストのバイト列
/// * `separator` - キー区切り文字
/// * `prefix` - 現在のキープレフィックス（親のキーパス）
/// * `key_ranges` - キーの位置情報を格納する `HashMap`
/// * `value_ranges` - 値の位置情報を格納する `HashMap`
fn extract_keys_from_node(
    node: tree_sitter::Node<'_>,
    source: &[u8],
    separator: &str,
    prefix: Option<&str>,
    key_ranges: &mut HashMap<String, SourceRange>,
    value_ranges: &mut HashMap<String, SourceRange>,
) {
    match node.kind() {
        "document" | "object" => {
            // object の場合、子ノードを探索
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    extract_keys_from_node(
                        child,
                        source,
                        separator,
                        prefix,
                        key_ranges,
                        value_ranges,
                    );
                }
            }
        }
        "pair" => {
            // pair ノードの場合、キーと値を抽出
            // pair の構造: { key: value }
            // 子ノード[0]: キー（string）
            // 子ノード[1]: コロン ":"
            // 子ノード[2]: 値（string, object, array など）

            let Some(key_node) = node.child_by_field_name("key") else {
                return;
            };

            let Some(value_node) = node.child_by_field_name("value") else {
                return;
            };

            // キー文字列を取得（ダブルクォートを削除）
            let Ok(key_text) = key_node.utf8_text(source) else {
                tracing::warn!("Failed to get key text from node");
                return;
            };
            let key = key_text.trim_matches('"');

            // 完全なキーパスを構築
            let full_key =
                prefix.map_or_else(|| key.to_string(), |p| format!("{p}{separator}{key}"));

            // キーノードの位置情報を SourceRange に変換
            let key_start_pos = key_node.start_position();
            let key_end_pos = key_node.end_position();
            #[allow(clippy::cast_possible_truncation)]
            let key_range = SourceRange {
                start: SourcePosition {
                    line: key_start_pos.row as u32,
                    character: key_start_pos.column as u32,
                },
                end: SourcePosition {
                    line: key_end_pos.row as u32,
                    character: key_end_pos.column as u32,
                },
            };

            // キーの位置情報を追加
            key_ranges.insert(full_key.clone(), key_range);

            // 値が文字列の場合、値の位置情報も記録
            if value_node.kind() == "string" {
                let value_start_pos = value_node.start_position();
                let value_end_pos = value_node.end_position();
                #[allow(clippy::cast_possible_truncation)]
                let value_range = SourceRange {
                    start: SourcePosition {
                        line: value_start_pos.row as u32,
                        character: value_start_pos.column as u32,
                    },
                    end: SourcePosition {
                        line: value_end_pos.row as u32,
                        character: value_end_pos.column as u32,
                    },
                };
                value_ranges.insert(full_key.clone(), value_range);
            }

            // 値が object の場合は再帰的に探索
            if value_node.kind() == "object" {
                extract_keys_from_node(
                    value_node,
                    source,
                    separator,
                    Some(&full_key),
                    key_ranges,
                    value_ranges,
                );
            }
        }
        _ => {
            // その他のノードタイプは無視
        }
    }
}

impl Translation {
    /// カーソル位置から翻訳キーを取得
    ///
    /// キーまたは値の位置にカーソルがある場合、対応するキーを返します。
    ///
    /// # Arguments
    /// * `db` - Salsa データベース
    /// * `position` - カーソル位置
    ///
    /// # Returns
    /// カーソル位置にあるキー、見つからない場合は None
    pub fn key_at_position(
        self,
        db: &dyn crate::db::I18nDatabase,
        position: SourcePosition,
    ) -> Option<crate::interned::TransKey<'_>> {
        // まずキーの範囲をチェック
        let key_ranges = self.key_ranges(db);
        for (key, range) in key_ranges {
            if position_in_range(position, *range) {
                return Some(crate::interned::TransKey::new(db, key.clone()));
            }
        }

        // 次に値の範囲をチェック
        let value_ranges = self.value_ranges(db);
        for (key, range) in value_ranges {
            if position_in_range(position, *range) {
                return Some(crate::interned::TransKey::new(db, key.clone()));
            }
        }

        None
    }
}

/// 位置が範囲内にあるかをチェック
const fn position_in_range(position: SourcePosition, range: SourceRange) -> bool {
    // 開始位置より前の場合
    if position.line < range.start.line {
        return false;
    }
    if position.line == range.start.line && position.character < range.start.character {
        return false;
    }

    // 終了位置より後の場合
    if position.line > range.end.line {
        return false;
    }
    if position.line == range.end.line && position.character > range.end.character {
        return false;
    }

    true
}

/// 翻訳ファイルを読み込む
///
/// JSONファイルをパースして、Translation Input を作成します。
///
/// # Arguments
/// * `db` - Salsa データベース
/// * `file_path` - 翻訳ファイルのパス
/// * `separator` - キー区切り文字
///
/// # Returns
/// * `Ok(Translation)` - 成功時
/// * `Err(String)` - エラー時（ファイル読み込みまたはJSONパースエラー）
///
/// # Errors
/// - ファイルの読み込みに失敗した場合
/// - JSONのパースに失敗した場合
pub fn load_translation_file(
    db: &dyn crate::db::I18nDatabase,
    file_path: &Path,
    separator: &str,
) -> Result<Translation, String> {
    // ファイルを読み込み
    let content = std::fs::read_to_string(file_path)
        .map_err(|e| format!("Failed to read translation file: {e}"))?;

    // JSON をパース
    let json: Value =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse JSON: {e}"))?;

    // フラット化
    let keys = flatten_json(&json, separator, None);

    // キーと値の位置情報のマッピングを抽出
    let (key_ranges, value_ranges) = extract_key_value_ranges(&content, separator);

    // ファイルパスから言語コードを検出
    let language = detect_language_from_path(file_path);

    Ok(Translation::new(
        db,
        language,
        file_path.to_string_lossy().to_string(),
        keys,
        content,
        key_ranges,
        value_ranges,
    ))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use googletest::prelude::*;
    use rstest::rstest;
    use serde_json::json;

    use super::*;

    #[googletest::test]
    fn test_flatten_json_simple() {
        let json = json!({
            "hello": "Hello",
            "goodbye": "Goodbye"
        });

        let result = flatten_json(&json, ".", None);

        expect_that!(result.get("hello"), some(eq(&"Hello".to_string())));
        expect_that!(result.get("goodbye"), some(eq(&"Goodbye".to_string())));
        expect_that!(result.len(), eq(2));
    }

    #[googletest::test]
    fn test_flatten_json_nested() {
        let json = json!({
            "common": {
                "hello": "Hello",
                "goodbye": "Goodbye"
            },
            "errors": {
                "notFound": "Not found"
            }
        });

        let result = flatten_json(&json, ".", None);

        expect_that!(result.get("common.hello"), some(eq(&"Hello".to_string())));
        expect_that!(result.get("common.goodbye"), some(eq(&"Goodbye".to_string())));
        expect_that!(result.get("errors.notFound"), some(eq(&"Not found".to_string())));
        expect_that!(result.len(), eq(3));
    }

    #[googletest::test]
    fn test_flatten_json_deep_nested() {
        let json = json!({
            "a": {
                "b": {
                    "c": "Deep value"
                }
            }
        });

        let result = flatten_json(&json, ".", None);

        expect_that!(result.get("a.b.c"), some(eq(&"Deep value".to_string())));
        expect_that!(result.len(), eq(1));
    }

    #[googletest::test]
    fn test_flatten_json_custom_separator() {
        let json = json!({
            "common": {
                "hello": "Hello"
            }
        });

        let result = flatten_json(&json, "_", None);

        expect_that!(result.get("common_hello"), some(eq(&"Hello".to_string())));
    }

    #[googletest::test]
    fn test_flatten_json_non_string_values() {
        let json = json!({
            "number": 42,
            "boolean": true,
            "null": null
        });

        let result = flatten_json(&json, ".", None);

        expect_that!(result.get("number"), some(eq(&"42".to_string())));
        expect_that!(result.get("boolean"), some(eq(&"true".to_string())));
        expect_that!(result.get("null"), some(eq(&"null".to_string())));
    }

    #[rstest]
    // Basic language detection
    #[case("/path/to/locales/en/trans.json", "en")]
    #[case("/path/to/locales/ja/trans.json", "ja")]
    #[case("/path/to/locales/hoge/trans.json", "unknown")]
    // Language name can be included anywhere in the path
    #[case("/path/to/locales/sub/en.json", "en")]
    #[case("/path/to/en/locales/trans.json", "en")]
    #[case("/path/to/locales/en-trans.json", "unknown")] // Hyphenated, not separated
    // Language names with various cases and separators
    #[case("/path/to/locales/en-us/trans.json", "en-us")]
    #[case("/path/to/locales/en_us/trans.json", "en_us")]
    #[case("/path/to/locales/en-US/trans.json", "en-US")]
    // When multiple locale names are included, the last match is returned
    #[case("/path/to/locales/en/ja.json", "ja")]
    fn test_detect_language_from_path(#[case] path: &str, #[case] expected: &str) {
        let result = detect_language_from_path(Path::new(path));
        assert_eq!(result, expected);
    }

    #[googletest::test]
    fn test_extract_key_value_ranges_simple() {
        let json_text = r#"{
  "hello": "Hello",
  "goodbye": "Goodbye"
}"#;

        let (key_ranges, value_ranges) = extract_key_value_ranges(json_text, ".");

        // キーの位置情報を確認
        expect_that!(key_ranges.len(), eq(2));
        expect_that!(key_ranges.contains_key("hello"), eq(true));
        expect_that!(key_ranges.contains_key("goodbye"), eq(true));

        // "hello" キーの位置情報を確認（2行目、2文字目から）
        let hello_range = key_ranges.get("hello");
        expect_that!(hello_range, some(anything()));
        if let Some(range) = hello_range {
            expect_that!(range.start.line, eq(1)); // 0-indexed
            expect_that!(range.start.character, eq(2));
        }

        // 値の位置情報を確認
        expect_that!(value_ranges.len(), eq(2));
        expect_that!(value_ranges.contains_key("hello"), eq(true));
        expect_that!(value_ranges.contains_key("goodbye"), eq(true));

        // "hello" の値 "Hello" の位置情報を確認
        let hello_value_range = value_ranges.get("hello");
        expect_that!(hello_value_range, some(anything()));
        if let Some(range) = hello_value_range {
            expect_that!(range.start.line, eq(1)); // 0-indexed
            expect_that!(range.start.character, eq(11)); // "hello": の後
        }
    }

    #[googletest::test]
    fn test_extract_key_value_ranges_nested() {
        let json_text = r#"{
  "common": {
    "hello": "Hello",
    "goodbye": "Goodbye"
  }
}"#;

        let (key_ranges, value_ranges) = extract_key_value_ranges(json_text, ".");

        // キーの位置情報を確認
        expect_that!(key_ranges.len(), eq(3)); // "common", "common.hello", "common.goodbye"
        expect_that!(key_ranges.contains_key("common"), eq(true));
        expect_that!(key_ranges.contains_key("common.hello"), eq(true));
        expect_that!(key_ranges.contains_key("common.goodbye"), eq(true));

        // 値の位置情報を確認（オブジェクト値の "common" は含まれない）
        expect_that!(value_ranges.len(), eq(2)); // "common.hello", "common.goodbye" のみ
        expect_that!(value_ranges.contains_key("common"), eq(false)); // オブジェクト値は含まれない
        expect_that!(value_ranges.contains_key("common.hello"), eq(true));
        expect_that!(value_ranges.contains_key("common.goodbye"), eq(true));
    }

    #[googletest::test]
    fn test_extract_key_value_ranges_with_dots_in_keys() {
        // ユーザーが指摘したケース: キー自体に `.` が含まれている場合
        let json_text = r#"{
  "hoge.fuga": {
    "piyo": "Hello"
  },
  "hoge": {
    "foo.bar": "World"
  }
}"#;

        let (key_ranges, _value_ranges) = extract_key_value_ranges(json_text, ".");

        // "hoge.fuga" と "piyo" を分割せず、"hoge.fuga" というキーとして認識
        expect_that!(key_ranges.contains_key("hoge.fuga"), eq(true));
        expect_that!(key_ranges.contains_key("hoge.fuga.piyo"), eq(true));

        // "hoge" の下の "foo.bar" も同様
        expect_that!(key_ranges.contains_key("hoge"), eq(true));
        expect_that!(key_ranges.contains_key("hoge.foo.bar"), eq(true));

        // 間違った分割結果がないことを確認
        expect_that!(key_ranges.contains_key("hoge.foo"), eq(false));
        expect_that!(key_ranges.contains_key("foo.bar"), eq(false));
    }

    #[googletest::test]
    fn test_translation_key_at_position() {
        use crate::db::I18nDatabaseImpl;

        let db = I18nDatabaseImpl::default();

        let json_text = r#"{
  "hello": "Hello",
  "nested": {
    "key": "Value"
  }
}"#;

        let default_json = json!({});
        let parsed: Option<Value> = serde_json::from_str(json_text).ok();
        let json_ref = parsed.as_ref().unwrap_or(&default_json);
        let keys = flatten_json(json_ref, ".", None);
        let (key_ranges, value_ranges) = extract_key_value_ranges(json_text, ".");

        let translation = Translation::new(
            &db,
            "en".to_string(),
            "/test.json".to_string(),
            keys,
            json_text.to_string(),
            key_ranges,
            value_ranges,
        );

        // "hello" キーの位置（1行目、2文字目）にカーソルがある場合
        let position = SourcePosition { line: 1, character: 3 };
        let key = translation.key_at_position(&db, position);

        assert!(key.is_some(), "Expected key to be Some, but got None");
        if let Some(k) = key {
            assert_eq!(k.text(&db), &"hello".to_string());
        }

        // "nested.key" の "key" の位置（3行目）にカーソルがある場合
        let position = SourcePosition { line: 3, character: 5 };
        let key = translation.key_at_position(&db, position);

        assert!(key.is_some(), "Expected key to be Some, but got None");
        if let Some(k) = key {
            assert_eq!(k.text(&db), &"nested.key".to_string());
        }

        // "hello" の値 "Hello" の位置にカーソルがある場合もキーを取得できる
        let position = SourcePosition { line: 1, character: 12 }; // "Hello" の位置
        let key = translation.key_at_position(&db, position);

        assert!(key.is_some(), "Expected key from value position to be Some, but got None");
        if let Some(k) = key {
            assert_eq!(k.text(&db), &"hello".to_string());
        }

        // "nested.key" の値 "Value" の位置にカーソルがある場合もキーを取得できる
        let position = SourcePosition { line: 3, character: 12 }; // "Value" の位置
        let key = translation.key_at_position(&db, position);

        assert!(key.is_some(), "Expected key from value position to be Some, but got None");
        if let Some(k) = key {
            assert_eq!(k.text(&db), &"nested.key".to_string());
        }
    }
}
