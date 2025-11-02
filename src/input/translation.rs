//! 翻訳ファイル入力定義

use std::collections::HashMap;
use std::path::Path;

use serde_json::Value;

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
            let full_key =
                if let Some(p) = prefix { format!("{p}{separator}{key}") } else { key.clone() };

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

    // 言語コードをファイル名から抽出（例: en.json -> "en"）
    let language = file_path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown").to_string();

    Ok(Translation::new(db, language, file_path.to_string_lossy().to_string(), keys))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn test_flatten_json_simple() {
        let json = json!({
            "hello": "Hello",
            "goodbye": "Goodbye"
        });

        let result = flatten_json(&json, ".", None);

        assert_eq!(result.get("hello"), Some(&"Hello".to_string()));
        assert_eq!(result.get("goodbye"), Some(&"Goodbye".to_string()));
        assert_eq!(result.len(), 2);
    }

    #[test]
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

        assert_eq!(result.get("common.hello"), Some(&"Hello".to_string()));
        assert_eq!(result.get("common.goodbye"), Some(&"Goodbye".to_string()));
        assert_eq!(result.get("errors.notFound"), Some(&"Not found".to_string()));
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_flatten_json_deep_nested() {
        let json = json!({
            "a": {
                "b": {
                    "c": "Deep value"
                }
            }
        });

        let result = flatten_json(&json, ".", None);

        assert_eq!(result.get("a.b.c"), Some(&"Deep value".to_string()));
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_flatten_json_custom_separator() {
        let json = json!({
            "common": {
                "hello": "Hello"
            }
        });

        let result = flatten_json(&json, "_", None);

        assert_eq!(result.get("common_hello"), Some(&"Hello".to_string()));
    }

    #[test]
    fn test_flatten_json_non_string_values() {
        let json = json!({
            "number": 42,
            "boolean": true,
            "null": null
        });

        let result = flatten_json(&json, ".", None);

        assert_eq!(result.get("number"), Some(&"42".to_string()));
        assert_eq!(result.get("boolean"), Some(&"true".to_string()));
        assert_eq!(result.get("null"), Some(&"null".to_string()));
    }
}
