//! Plural suffix handling.

use std::collections::{
    HashMap,
    HashSet,
};

use crate::framework::PluralStrategy;

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
///
/// Returns `None` immediately for `PluralStrategy::Icu` (no suffix-based plurals).
#[must_use]
pub fn get_plural_base_key(key: &str, strategy: PluralStrategy) -> Option<&str> {
    if strategy == PluralStrategy::Icu {
        return None;
    }
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
pub fn has_plural_variants(
    base_key: &str,
    available_keys: &HashSet<String>,
    strategy: PluralStrategy,
) -> bool {
    if strategy == PluralStrategy::Icu {
        return false;
    }
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
    strategy: PluralStrategy,
) -> Vec<(&'a str, &'a str)> {
    if strategy == PluralStrategy::Icu {
        return Vec::new();
    }
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
pub fn is_key_used_with_plural(
    key: &str,
    used_keys: &HashSet<String>,
    strategy: PluralStrategy,
) -> bool {
    if used_keys.contains(key) {
        return true;
    }

    get_plural_base_key(key, strategy).is_some_and(|base_key| used_keys.contains(base_key))
}

/// Returns true if the key exists or has plural variants.
#[must_use]
#[allow(clippy::implicit_hasher)]
pub fn key_exists_with_plural(
    key: &str,
    available_keys: &HashSet<String>,
    strategy: PluralStrategy,
) -> bool {
    if available_keys.contains(key) {
        return true;
    }

    has_plural_variants(key, available_keys, strategy)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_plural_base_key() {
        // Cardinal suffixes
        assert_eq!(get_plural_base_key("items_zero", PluralStrategy::SuffixBased), Some("items"));
        assert_eq!(get_plural_base_key("items_one", PluralStrategy::SuffixBased), Some("items"));
        assert_eq!(get_plural_base_key("items_two", PluralStrategy::SuffixBased), Some("items"));
        assert_eq!(get_plural_base_key("items_few", PluralStrategy::SuffixBased), Some("items"));
        assert_eq!(get_plural_base_key("items_many", PluralStrategy::SuffixBased), Some("items"));
        assert_eq!(get_plural_base_key("items_other", PluralStrategy::SuffixBased), Some("items"));

        // Ordinal suffixes
        assert_eq!(
            get_plural_base_key("place_ordinal_one", PluralStrategy::SuffixBased),
            Some("place")
        );
        assert_eq!(
            get_plural_base_key("place_ordinal_two", PluralStrategy::SuffixBased),
            Some("place")
        );
        assert_eq!(
            get_plural_base_key("place_ordinal_few", PluralStrategy::SuffixBased),
            Some("place")
        );
        assert_eq!(
            get_plural_base_key("place_ordinal_other", PluralStrategy::SuffixBased),
            Some("place")
        );

        // No suffix or unknown suffix
        assert_eq!(get_plural_base_key("items", PluralStrategy::SuffixBased), None);
        assert_eq!(get_plural_base_key("items_unknown", PluralStrategy::SuffixBased), None);
        assert_eq!(get_plural_base_key("_one", PluralStrategy::SuffixBased), None);
    }

    #[test]
    fn test_has_plural_variants() {
        let keys: HashSet<String> =
            ["items_one", "items_other", "single"].iter().copied().map(String::from).collect();

        assert!(has_plural_variants("items", &keys, PluralStrategy::SuffixBased));
        assert!(!has_plural_variants("single", &keys, PluralStrategy::SuffixBased));
        assert!(!has_plural_variants("missing", &keys, PluralStrategy::SuffixBased));
    }

    #[test]
    fn test_find_plural_variants() {
        let mut keys = HashMap::new();
        keys.insert("items_one".to_string(), "{{count}} item".to_string());
        keys.insert("items_other".to_string(), "{{count}} items".to_string());
        keys.insert("single".to_string(), "Single value".to_string());

        let variants = find_plural_variants("items", &keys, PluralStrategy::SuffixBased);
        assert_eq!(variants.len(), 2);

        let variant_keys: Vec<&str> = variants.iter().map(|(k, _)| *k).collect();
        assert!(variant_keys.contains(&"items_one"));
        assert!(variant_keys.contains(&"items_other"));

        // No variants
        let no_variants = find_plural_variants("single", &keys, PluralStrategy::SuffixBased);
        assert!(no_variants.is_empty());
    }

    #[test]
    fn test_is_key_used_with_plural() {
        let used_keys: HashSet<String> =
            ["items", "other"].iter().copied().map(String::from).collect();

        // Direct match
        assert!(is_key_used_with_plural("items", &used_keys, PluralStrategy::SuffixBased));
        assert!(is_key_used_with_plural("other", &used_keys, PluralStrategy::SuffixBased));

        // Plural variant of used base key
        assert!(is_key_used_with_plural("items_one", &used_keys, PluralStrategy::SuffixBased));
        assert!(is_key_used_with_plural("items_other", &used_keys, PluralStrategy::SuffixBased));
        assert!(is_key_used_with_plural(
            "items_ordinal_few",
            &used_keys,
            PluralStrategy::SuffixBased
        ));

        // Not used
        assert!(!is_key_used_with_plural("missing", &used_keys, PluralStrategy::SuffixBased));
        assert!(!is_key_used_with_plural("missing_one", &used_keys, PluralStrategy::SuffixBased));
    }

    #[test]
    fn test_key_exists_with_plural() {
        let keys: HashSet<String> =
            ["items_one", "items_other", "single"].iter().copied().map(String::from).collect();

        // Direct match
        assert!(key_exists_with_plural("items_one", &keys, PluralStrategy::SuffixBased));
        assert!(key_exists_with_plural("single", &keys, PluralStrategy::SuffixBased));

        // Base key with plural variants
        assert!(key_exists_with_plural("items", &keys, PluralStrategy::SuffixBased));

        // Not exists
        assert!(!key_exists_with_plural("missing", &keys, PluralStrategy::SuffixBased));
    }

    // --- ICU strategy tests ---

    #[test]
    fn icu_get_plural_base_key_returns_none() {
        assert_eq!(get_plural_base_key("items_one", PluralStrategy::Icu), None);
        assert_eq!(get_plural_base_key("items_other", PluralStrategy::Icu), None);
    }

    #[test]
    fn icu_has_plural_variants_returns_false() {
        let keys: HashSet<String> =
            ["items_one", "items_other"].iter().copied().map(String::from).collect();
        assert!(!has_plural_variants("items", &keys, PluralStrategy::Icu));
    }

    #[test]
    fn icu_find_plural_variants_returns_empty() {
        let mut keys = HashMap::new();
        keys.insert("items_one".to_string(), "one item".to_string());
        assert!(find_plural_variants("items", &keys, PluralStrategy::Icu).is_empty());
    }

    #[test]
    fn icu_is_key_used_with_plural_only_exact_match() {
        let used_keys: HashSet<String> = std::iter::once("items").map(String::from).collect();

        // Direct match still works
        assert!(is_key_used_with_plural("items", &used_keys, PluralStrategy::Icu));
        // Suffix stripping disabled
        assert!(!is_key_used_with_plural("items_one", &used_keys, PluralStrategy::Icu));
    }

    #[test]
    fn icu_key_exists_with_plural_only_exact_match() {
        let keys: HashSet<String> =
            ["items_one", "items_other"].iter().copied().map(String::from).collect();

        // Direct match still works
        assert!(key_exists_with_plural("items_one", &keys, PluralStrategy::Icu));
        // No plural variant expansion
        assert!(!key_exists_with_plural("items", &keys, PluralStrategy::Icu));
    }
}
