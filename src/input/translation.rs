//! 翻訳ファイル入力定義

use std::collections::{
    HashMap,
    HashSet,
};
use std::path::Path;
use std::sync::LazyLock;

use serde_json::Value;

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

    // ファイルパスから言語コードを検出
    let language = detect_language_from_path(file_path);

    Ok(Translation::new(db, language, file_path.to_string_lossy().to_string(), keys))
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
}
