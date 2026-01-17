//! i18next plural suffix handling.

use std::collections::{
    HashMap,
    HashSet,
};

/// Longer suffixes must come first to avoid `_one` matching `place_ordinal_one`.
pub const PLURAL_SUFFIXES: &[&str] = &[
    "_ordinal_zero",
    "_ordinal_one",
    "_ordinal_two",
    "_ordinal_few",
    "_ordinal_many",
    "_ordinal_other",
    "_zero",
    "_one",
    "_two",
    "_few",
    "_many",
    "_other",
];

/// Returns the base key by stripping any plural suffix, or `None` if no suffix found.
#[must_use]
pub fn get_plural_base_key(key: &str) -> Option<&str> {
    for suffix in PLURAL_SUFFIXES {
        if let Some(base) = key.strip_suffix(suffix)
            && !base.is_empty()
        {
            return Some(base);
        }
    }
    None
}

/// Returns true if any plural variant of the base key exists.
#[must_use]
#[allow(clippy::implicit_hasher)]
pub fn has_plural_variants(base_key: &str, available_keys: &HashSet<String>) -> bool {
    PLURAL_SUFFIXES.iter().any(|suffix| {
        let variant_key = format!("{base_key}{suffix}");
        available_keys.contains(&variant_key)
    })
}

/// Returns all existing plural variants of the base key as (key, value) pairs.
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

/// Returns true if the key or its base key (for plural variants) is used.
#[must_use]
#[allow(clippy::implicit_hasher)]
pub fn is_key_used_with_plural(key: &str, used_keys: &HashSet<String>) -> bool {
    if used_keys.contains(key) {
        return true;
    }

    get_plural_base_key(key).is_some_and(|base_key| used_keys.contains(base_key))
}

/// Returns true if the key exists or has plural variants.
#[must_use]
#[allow(clippy::implicit_hasher)]
pub fn key_exists_with_plural(key: &str, available_keys: &HashSet<String>) -> bool {
    if available_keys.contains(key) {
        return true;
    }

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
            ["items_one", "items_other", "single"].iter().copied().map(String::from).collect();

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
        let used_keys: HashSet<String> =
            ["items", "other"].iter().copied().map(String::from).collect();

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
            ["items_one", "items_other", "single"].iter().copied().map(String::from).collect();

        // Direct match
        assert!(key_exists_with_plural("items_one", &keys));
        assert!(key_exists_with_plural("single", &keys));

        // Base key with plural variants
        assert!(key_exists_with_plural("items", &keys));

        // Not exists
        assert!(!key_exists_with_plural("missing", &keys));
    }
}
