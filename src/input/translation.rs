//! Translation file input definitions

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

/// Detect namespace from file path.
///
/// Extracts namespace from file name or directory name.
/// Language codes (en, ja, etc.) are not treated as namespaces.
///
/// # Examples
/// - `locales/en/common.json` -> Some("common") (file name is namespace)
/// - `locales/common/en.json` -> Some("common") (directory name is namespace)
/// - `locales/en.json` -> None (single file)
/// - `locales/en/translation.json` -> Some("translation")
fn detect_namespace_from_path(file_path: &Path) -> Option<String> {
    let file_stem = file_path.file_stem()?.to_string_lossy().to_string();
    let file_stem_normalized = normalize_language_code(&file_stem);

    let parent = file_path.parent()?;
    let parent_name = parent.file_name()?.to_string_lossy().to_string();
    let parent_name_normalized = normalize_language_code(&parent_name);

    // File name is namespace if not a language code
    if !LANGUAGE_CODES.contains(&file_stem_normalized) && !LANGUAGE_CODES.contains(&file_stem) {
        return Some(file_stem);
    }

    // Parent directory is namespace if not a language code or common parent
    let common_parents = ["locales", "messages", "translations", "i18n", "lang", "langs"];
    if !LANGUAGE_CODES.contains(&parent_name_normalized)
        && !LANGUAGE_CODES.contains(&parent_name)
        && !common_parents.contains(&parent_name.to_lowercase().as_str())
    {
        return Some(parent_name);
    }

    None
}

/// Salsa input representing translation data.
#[salsa::input]
pub struct Translation {
    pub language: String,

    /// Namespace inferred from file path (e.g., "common", "errors").
    #[returns(ref)]
    pub namespace: Option<String>,

    #[returns(ref)]
    pub file_path: String,

    /// Flattened translation key map (e.g., "common.hello" -> "Hello").
    #[returns(ref)]
    pub keys: HashMap<String, String>,

    #[returns(ref)]
    pub json_text: String,

    /// Key to source range mapping for go-to-definition.
    #[returns(ref)]
    pub key_ranges: HashMap<String, SourceRange>,

    /// Value to source range mapping for editing.
    #[returns(ref)]
    pub value_ranges: HashMap<String, SourceRange>,
}

/// Flatten nested JSON object into dot-separated key map.
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
    flatten_json_value(json, separator, prefix, &mut result);
    result
}

fn flatten_json_value(
    json: &Value,
    separator: &str,
    prefix: Option<&str>,
    result: &mut HashMap<String, String>,
) {
    match json {
        Value::Object(map) => {
            for (key, value) in map {
                let full_key =
                    prefix.map_or_else(|| key.clone(), |p| format!("{p}{separator}{key}"));
                flatten_json_value(value, separator, Some(&full_key), result);
            }
        }
        Value::Array(arr) => {
            for (index, value) in arr.iter().enumerate() {
                let full_key =
                    prefix.map_or_else(|| format!("[{index}]"), |p| format!("{p}[{index}]"));
                flatten_json_value(value, separator, Some(&full_key), result);
            }
        }
        Value::String(s) => {
            if let Some(key) = prefix {
                result.insert(key.to_string(), s.clone());
            }
        }
        _ => {
            if let Some(key) = prefix {
                result.insert(key.to_string(), json.to_string());
            }
        }
    }
}

/// Extract key and value source ranges from JSON text using tree-sitter.
#[must_use]
pub fn extract_key_value_ranges(
    json_text: &str,
    separator: &str,
) -> (HashMap<String, SourceRange>, HashMap<String, SourceRange>) {
    let mut key_ranges = HashMap::new();
    let mut value_ranges = HashMap::new();

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
            // tree-sitter 0.26+ requires u32 for child()
            #[allow(clippy::cast_possible_truncation)]
            for i in 0..(node.child_count() as u32) {
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
        "array" => {
            extract_array_elements(node, source, separator, prefix, key_ranges, value_ranges);
        }
        "pair" => {
            extract_pair(node, source, separator, prefix, key_ranges, value_ranges);
        }
        _ => {}
    }
}

fn extract_array_elements(
    node: tree_sitter::Node<'_>,
    source: &[u8],
    separator: &str,
    prefix: Option<&str>,
    key_ranges: &mut HashMap<String, SourceRange>,
    value_ranges: &mut HashMap<String, SourceRange>,
) {
    let mut index = 0;
    // tree-sitter 0.26+ requires u32 for child()
    #[allow(clippy::cast_possible_truncation)]
    for i in 0..(node.child_count() as u32) {
        let Some(child) = node.child(i) else {
            continue;
        };

        if matches!(child.kind(), "[" | "]" | ",") {
            continue;
        }

        let full_key = prefix.map_or_else(|| format!("[{index}]"), |p| format!("{p}[{index}]"));
        let elem_range = SourceRange::from_node(&child);
        key_ranges.insert(full_key.clone(), elem_range);

        match child.kind() {
            "object" | "array" => {
                extract_keys_from_node(
                    child,
                    source,
                    separator,
                    Some(&full_key),
                    key_ranges,
                    value_ranges,
                );
            }
            _ => {
                value_ranges.insert(full_key, elem_range);
            }
        }

        index += 1;
    }
}

fn extract_pair(
    node: tree_sitter::Node<'_>,
    source: &[u8],
    separator: &str,
    prefix: Option<&str>,
    key_ranges: &mut HashMap<String, SourceRange>,
    value_ranges: &mut HashMap<String, SourceRange>,
) {
    let Some(key_node) = node.child_by_field_name("key") else {
        return;
    };
    let Some(value_node) = node.child_by_field_name("value") else {
        return;
    };

    let Ok(key_text) = key_node.utf8_text(source) else {
        tracing::warn!("Failed to get key text from node");
        return;
    };
    let key = key_text.trim_matches('"');

    let full_key = prefix.map_or_else(|| key.to_string(), |p| format!("{p}{separator}{key}"));

    key_ranges.insert(full_key.clone(), SourceRange::from_node(&key_node));

    match value_node.kind() {
        "object" | "array" => {
            extract_keys_from_node(
                value_node,
                source,
                separator,
                Some(&full_key),
                key_ranges,
                value_ranges,
            );
        }
        _ => {
            value_ranges.insert(full_key, SourceRange::from_node(&value_node));
        }
    }
}

impl Translation {
    /// Get translation key at cursor position.
    ///
    /// Returns the key if cursor is on a key or value position.
    pub fn key_at_position(
        self,
        db: &dyn crate::db::I18nDatabase,
        position: SourcePosition,
    ) -> Option<crate::interned::TransKey<'_>> {
        let key_ranges = self.key_ranges(db);
        for (key, range) in key_ranges {
            if range.contains(position) {
                return Some(crate::interned::TransKey::new(db, key.clone()));
            }
        }

        let value_ranges = self.value_ranges(db);
        for (key, range) in value_ranges {
            if range.contains(position) {
                return Some(crate::interned::TransKey::new(db, key.clone()));
            }
        }

        None
    }
}

/// Load translation file and create a Translation input.
///
/// # Errors
/// Returns error if file read or JSON parse fails.
pub fn load_translation_file(
    db: &dyn crate::db::I18nDatabase,
    file_path: &Path,
    separator: &str,
) -> Result<Translation, String> {
    let content = std::fs::read_to_string(file_path)
        .map_err(|e| format!("Failed to read translation file: {e}"))?;

    let json: Value =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse JSON: {e}"))?;

    let keys = flatten_json(&json, separator, None);
    let (key_ranges, value_ranges) = extract_key_value_ranges(&content, separator);
    let language = detect_language_from_path(file_path);
    let namespace = detect_namespace_from_path(file_path);

    Ok(Translation::new(
        db,
        language,
        namespace,
        file_path.to_string_lossy().to_string(),
        keys,
        content,
        key_ranges,
        value_ranges,
    ))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
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

    #[rstest]
    // File name is namespace
    #[case("/path/to/locales/en/common.json", Some("common"))]
    #[case("/path/to/locales/ja/errors.json", Some("errors"))]
    #[case("/path/to/locales/en/translation.json", Some("translation"))]
    // Directory name is namespace
    #[case("/path/to/locales/common/en.json", Some("common"))]
    #[case("/path/to/locales/errors/ja.json", Some("errors"))]
    // Single file -> None
    #[case("/path/to/locales/en.json", None)]
    #[case("/path/to/messages/ja.json", None)]
    // Common parent directories are excluded
    #[case("/path/to/i18n/en.json", None)]
    #[case("/path/to/translations/en.json", None)]
    fn test_detect_namespace_from_path(#[case] path: &str, #[case] expected: Option<&str>) {
        let result = detect_namespace_from_path(Path::new(path));
        assert_eq!(result.as_deref(), expected);
    }

    #[googletest::test]
    fn test_extract_key_value_ranges_simple() {
        let json_text = r#"{
  "hello": "Hello",
  "goodbye": "Goodbye"
}"#;

        let (key_ranges, value_ranges) = extract_key_value_ranges(json_text, ".");

        expect_that!(key_ranges.len(), eq(2));
        expect_that!(key_ranges.contains_key("hello"), eq(true));
        expect_that!(key_ranges.contains_key("goodbye"), eq(true));

        let hello_range = key_ranges.get("hello");
        expect_that!(hello_range, some(anything()));
        if let Some(range) = hello_range {
            expect_that!(range.start.line, eq(1));
            expect_that!(range.start.character, eq(2));
        }

        expect_that!(value_ranges.len(), eq(2));
        expect_that!(value_ranges.contains_key("hello"), eq(true));
        expect_that!(value_ranges.contains_key("goodbye"), eq(true));

        let hello_value_range = value_ranges.get("hello");
        expect_that!(hello_value_range, some(anything()));
        if let Some(range) = hello_value_range {
            expect_that!(range.start.line, eq(1));
            expect_that!(range.start.character, eq(11));
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

        expect_that!(key_ranges.len(), eq(3));
        expect_that!(key_ranges.contains_key("common"), eq(true));
        expect_that!(key_ranges.contains_key("common.hello"), eq(true));
        expect_that!(key_ranges.contains_key("common.goodbye"), eq(true));

        // Object values don't have value ranges, only leaf values do
        expect_that!(value_ranges.len(), eq(2));
        expect_that!(value_ranges.contains_key("common"), eq(false));
        expect_that!(value_ranges.contains_key("common.hello"), eq(true));
        expect_that!(value_ranges.contains_key("common.goodbye"), eq(true));
    }

    #[googletest::test]
    fn test_extract_key_value_ranges_with_dots_in_keys() {
        // Keys containing dots are not split but concatenated with the separator
        let json_text = r#"{
  "hoge.fuga": {
    "piyo": "Hello"
  },
  "hoge": {
    "foo.bar": "World"
  }
}"#;

        let (key_ranges, _value_ranges) = extract_key_value_ranges(json_text, ".");

        expect_that!(key_ranges.contains_key("hoge.fuga"), eq(true));
        expect_that!(key_ranges.contains_key("hoge.fuga.piyo"), eq(true));

        expect_that!(key_ranges.contains_key("hoge"), eq(true));
        expect_that!(key_ranges.contains_key("hoge.foo.bar"), eq(true));

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
            None,
            "/test.json".to_string(),
            keys,
            json_text.to_string(),
            key_ranges,
            value_ranges,
        );

        let position = SourcePosition { line: 1, character: 3 };
        let key = translation.key_at_position(&db, position);
        assert!(key.is_some());
        assert_eq!(key.unwrap().text(&db), &"hello".to_string());

        let position = SourcePosition { line: 3, character: 5 };
        let key = translation.key_at_position(&db, position);
        assert!(key.is_some());
        assert_eq!(key.unwrap().text(&db), &"nested.key".to_string());

        let position = SourcePosition { line: 1, character: 12 };
        let key = translation.key_at_position(&db, position);
        assert!(key.is_some());
        assert_eq!(key.unwrap().text(&db), &"hello".to_string());

        let position = SourcePosition { line: 3, character: 12 };
        let key = translation.key_at_position(&db, position);
        assert!(key.is_some());
        assert_eq!(key.unwrap().text(&db), &"nested.key".to_string());
    }

    #[googletest::test]
    fn test_flatten_json_with_array() {
        let json = json!({
            "items": ["apple", "banana", "cherry"]
        });

        let result = flatten_json(&json, ".", None);

        expect_that!(result.get("items[0]"), some(eq(&"apple".to_string())));
        expect_that!(result.get("items[1]"), some(eq(&"banana".to_string())));
        expect_that!(result.get("items[2]"), some(eq(&"cherry".to_string())));
        expect_that!(result.len(), eq(3));
    }

    #[googletest::test]
    fn test_flatten_json_with_nested_array() {
        let json = json!({
            "menu": {
                "items": ["item1", "item2"]
            }
        });

        let result = flatten_json(&json, ".", None);

        expect_that!(result.get("menu.items[0]"), some(eq(&"item1".to_string())));
        expect_that!(result.get("menu.items[1]"), some(eq(&"item2".to_string())));
    }

    #[googletest::test]
    fn test_flatten_json_with_array_of_objects() {
        let json = json!({
            "users": [
                { "name": "Alice" },
                { "name": "Bob" }
            ]
        });

        let result = flatten_json(&json, ".", None);

        expect_that!(result.get("users[0].name"), some(eq(&"Alice".to_string())));
        expect_that!(result.get("users[1].name"), some(eq(&"Bob".to_string())));
    }

    #[googletest::test]
    fn test_flatten_json_with_nested_arrays() {
        let json = json!({
            "matrix": [
                ["a", "b"],
                ["c", "d"]
            ]
        });

        let result = flatten_json(&json, ".", None);

        expect_that!(result.get("matrix[0][0]"), some(eq(&"a".to_string())));
        expect_that!(result.get("matrix[0][1]"), some(eq(&"b".to_string())));
        expect_that!(result.get("matrix[1][0]"), some(eq(&"c".to_string())));
        expect_that!(result.get("matrix[1][1]"), some(eq(&"d".to_string())));
    }

    #[googletest::test]
    fn test_extract_key_value_ranges_with_array() {
        let json_text = r#"{
  "items": ["apple", "banana"]
}"#;

        let (key_ranges, value_ranges) = extract_key_value_ranges(json_text, ".");

        expect_that!(key_ranges.contains_key("items"), eq(true));
        expect_that!(key_ranges.contains_key("items[0]"), eq(true));
        expect_that!(key_ranges.contains_key("items[1]"), eq(true));

        expect_that!(value_ranges.contains_key("items[0]"), eq(true));
        expect_that!(value_ranges.contains_key("items[1]"), eq(true));
    }

    #[googletest::test]
    fn test_extract_key_value_ranges_with_array_of_objects() {
        let json_text = r#"{
  "users": [
    { "name": "Alice" },
    { "name": "Bob" }
  ]
}"#;

        let (key_ranges, value_ranges) = extract_key_value_ranges(json_text, ".");

        expect_that!(key_ranges.contains_key("users"), eq(true));
        expect_that!(key_ranges.contains_key("users[0]"), eq(true));
        expect_that!(key_ranges.contains_key("users[0].name"), eq(true));
        expect_that!(key_ranges.contains_key("users[1]"), eq(true));
        expect_that!(key_ranges.contains_key("users[1].name"), eq(true));

        expect_that!(value_ranges.contains_key("users[0].name"), eq(true));
        expect_that!(value_ranges.contains_key("users[1].name"), eq(true));
    }
}
