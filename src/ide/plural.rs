//! i18next の plural suffix 処理モジュール
//!
//! i18next では複数形（plural）や序数（ordinal）を扱う際に、
//! キーに suffix を付加して言語ごとの形式を管理します。
//!
//! # Suffix 一覧
//! - Cardinal: `_zero`, `_one`, `_two`, `_few`, `_many`, `_other`
//! - Ordinal: `_ordinal_zero`, `_ordinal_one`, `_ordinal_two`, `_ordinal_few`, `_ordinal_many`, `_ordinal_other`
//!
//! # 使用例
//! ```json
//! {
//!   "items_one": "{{count}} item",
//!   "items_other": "{{count}} items"
//! }
//! ```
//! ```typescript
//! t("items", { count: 1 }) // → "1 item"
//! t("items", { count: 5 }) // → "5 items"
//! ```

use std::collections::{
    HashMap,
    HashSet,
};

/// i18next の plural suffix（Cardinal + Ordinal）
///
/// **重要**: 長い suffix を先に配置すること。
/// `_one` が `_ordinal_one` より先にあると、`place_ordinal_one` が
/// `_one` でマッチして `place_ordinal` になってしまう。
pub const PLURAL_SUFFIXES: &[&str] = &[
    // Ordinal（長い suffix を先に）
    "_ordinal_zero",
    "_ordinal_one",
    "_ordinal_two",
    "_ordinal_few",
    "_ordinal_many",
    "_ordinal_other",
    // Cardinal
    "_zero",
    "_one",
    "_two",
    "_few",
    "_many",
    "_other",
];

/// キーから plural suffix を除いたベースキーを取得
///
/// # Examples
/// - `"items_one"` → `Some("items")`
/// - `"items_ordinal_few"` → `Some("items")`
/// - `"items"` → `None`（suffix なし）
/// - `"items_unknown"` → `None`（未知の suffix）
#[must_use]
pub fn get_plural_base_key(key: &str) -> Option<&str> {
    for suffix in PLURAL_SUFFIXES {
        if let Some(base) = key.strip_suffix(suffix) {
            // 空のベースキーは無効
            if !base.is_empty() {
                return Some(base);
            }
        }
    }
    None
}

/// キーの plural バリアントが存在するかチェック
///
/// # Arguments
/// * `base_key` - ベースキー（例: `"items"`）
/// * `available_keys` - 利用可能なキーのセット
///
/// # Returns
/// 少なくとも1つの plural バリアントが存在すれば `true`
#[must_use]
#[allow(clippy::implicit_hasher)]
pub fn has_plural_variants(base_key: &str, available_keys: &HashSet<String>) -> bool {
    PLURAL_SUFFIXES.iter().any(|suffix| {
        let variant_key = format!("{base_key}{suffix}");
        available_keys.contains(&variant_key)
    })
}

/// キーの全 plural バリアントを取得
///
/// # Arguments
/// * `base_key` - ベースキー（例: `"items"`）
/// * `keys` - キーと値のマップ
///
/// # Returns
/// 存在する plural バリアントのキーと値のペアのベクター
#[must_use]
#[allow(clippy::implicit_hasher)]
pub fn find_plural_variants<'a>(
    base_key: &str,
    keys: &'a HashMap<String, String>,
) -> Vec<(&'a str, &'a str)> {
    PLURAL_SUFFIXES
        .iter()
        .filter_map(|suffix| {
            let variant_key = format!("{base_key}{suffix}");
            keys.get_key_value(&variant_key).map(|(k, v)| (k.as_str(), v.as_str()))
        })
        .collect()
}

/// キーが使用されているかチェック（plural suffix を考慮）
///
/// ベースキーが使用されている場合、そのキーの plural バリアントも使用済みとみなします。
///
/// # Arguments
/// * `key` - チェック対象のキー（例: `"items_one"`）
/// * `used_keys` - ソースコードで使用されているキーのセット
///
/// # Returns
/// キーまたはそのベースキーが使用されていれば `true`
#[must_use]
#[allow(clippy::implicit_hasher)]
pub fn is_key_used_with_plural(key: &str, used_keys: &HashSet<String>) -> bool {
    // 完全一致
    if used_keys.contains(key) {
        return true;
    }

    // plural バリアントの場合、ベースキーが使用されているかチェック
    get_plural_base_key(key).is_some_and(|base_key| used_keys.contains(base_key))
}

/// キーが存在するかチェック（plural suffix を考慮）
///
/// キー自体が存在しない場合でも、plural バリアントが存在すれば有効とみなします。
///
/// # Arguments
/// * `key` - チェック対象のキー（例: `"items"`）
/// * `available_keys` - 利用可能なキーのセット
///
/// # Returns
/// キー自体または plural バリアントが存在すれば `true`
#[must_use]
#[allow(clippy::implicit_hasher)]
pub fn key_exists_with_plural(key: &str, available_keys: &HashSet<String>) -> bool {
    // 完全一致
    if available_keys.contains(key) {
        return true;
    }

    // plural バリアントが存在するかチェック
    has_plural_variants(key, available_keys)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_plural_base_key() {
        // Cardinal suffixes
        assert_eq!(get_plural_base_key("items_zero"), Some("items"));
        assert_eq!(get_plural_base_key("items_one"), Some("items"));
        assert_eq!(get_plural_base_key("items_two"), Some("items"));
        assert_eq!(get_plural_base_key("items_few"), Some("items"));
        assert_eq!(get_plural_base_key("items_many"), Some("items"));
        assert_eq!(get_plural_base_key("items_other"), Some("items"));

        // Ordinal suffixes
        assert_eq!(get_plural_base_key("place_ordinal_one"), Some("place"));
        assert_eq!(get_plural_base_key("place_ordinal_two"), Some("place"));
        assert_eq!(get_plural_base_key("place_ordinal_few"), Some("place"));
        assert_eq!(get_plural_base_key("place_ordinal_other"), Some("place"));

        // No suffix or unknown suffix
        assert_eq!(get_plural_base_key("items"), None);
        assert_eq!(get_plural_base_key("items_unknown"), None);
        assert_eq!(get_plural_base_key("_one"), None); // empty base key
    }

    #[test]
    fn test_has_plural_variants() {
        let keys: HashSet<String> =
            ["items_one", "items_other", "single"].iter().map(|s| s.to_string()).collect();

        assert!(has_plural_variants("items", &keys));
        assert!(!has_plural_variants("single", &keys));
        assert!(!has_plural_variants("missing", &keys));
    }

    #[test]
    fn test_find_plural_variants() {
        let mut keys = HashMap::new();
        keys.insert("items_one".to_string(), "{{count}} item".to_string());
        keys.insert("items_other".to_string(), "{{count}} items".to_string());
        keys.insert("single".to_string(), "Single value".to_string());

        let variants = find_plural_variants("items", &keys);
        assert_eq!(variants.len(), 2);

        let variant_keys: Vec<&str> = variants.iter().map(|(k, _)| *k).collect();
        assert!(variant_keys.contains(&"items_one"));
        assert!(variant_keys.contains(&"items_other"));

        // No variants
        let no_variants = find_plural_variants("single", &keys);
        assert!(no_variants.is_empty());
    }

    #[test]
    fn test_is_key_used_with_plural() {
        let used_keys: HashSet<String> = ["items", "other"].iter().map(|s| s.to_string()).collect();

        // Direct match
        assert!(is_key_used_with_plural("items", &used_keys));
        assert!(is_key_used_with_plural("other", &used_keys));

        // Plural variant of used base key
        assert!(is_key_used_with_plural("items_one", &used_keys));
        assert!(is_key_used_with_plural("items_other", &used_keys));
        assert!(is_key_used_with_plural("items_ordinal_few", &used_keys));

        // Not used
        assert!(!is_key_used_with_plural("missing", &used_keys));
        assert!(!is_key_used_with_plural("missing_one", &used_keys));
    }

    #[test]
    fn test_key_exists_with_plural() {
        let keys: HashSet<String> =
            ["items_one", "items_other", "single"].iter().map(|s| s.to_string()).collect();

        // Direct match
        assert!(key_exists_with_plural("items_one", &keys));
        assert!(key_exists_with_plural("single", &keys));

        // Base key with plural variants
        assert!(key_exists_with_plural("items", &keys));

        // Not exists
        assert!(!key_exists_with_plural("missing", &keys));
    }
}
