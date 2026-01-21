//! Translation key matching utilities.

/// Checks if `child_key` is a child of `parent_key`.
///
/// Supports both separator-based (e.g., `items.foo`) and array notation (e.g., `items[0]`).
#[must_use]
pub fn is_child_key(child_key: &str, parent_key: &str, separator: &str) -> bool {
    let Some(remainder) = child_key.strip_prefix(parent_key) else {
        return false;
    };

    !remainder.is_empty() && (remainder.starts_with(separator) || remainder.starts_with('['))
}

#[cfg(test)]
mod tests {
    use googletest::prelude::*;
    use rstest::*;

    use super::*;

    #[rstest]
    #[case("items.foo", "items", ".")]
    #[case("items[0]", "items", ".")]
    #[case("items[0].name", "items", ".")]
    #[case("items[0][1]", "items", ".")]
    #[case("deep.nested.key", "deep", ".")]
    #[case("deep.nested.key", "deep.nested", ".")]
    fn is_child_key_positive_cases(
        #[case] child: &str,
        #[case] parent: &str,
        #[case] separator: &str,
    ) {
        assert_that!(is_child_key(child, parent, separator), eq(true));
    }

    #[rstest]
    #[case("itemsX", "items", ".")]
    #[case("items", "items", ".")]
    #[case("other.key", "items", ".")]
    #[case("item", "items", ".")]
    #[case("itemsfoo", "items", ".")]
    fn is_child_key_negative_cases(
        #[case] child: &str,
        #[case] parent: &str,
        #[case] separator: &str,
    ) {
        assert_that!(is_child_key(child, parent, separator), eq(false));
    }

    #[rstest]
    fn is_child_key_with_custom_separator() {
        assert_that!(is_child_key("ns:key", "ns", ":"), eq(true));
        assert_that!(is_child_key("ns:key:sub", "ns:key", ":"), eq(true));
        assert_that!(is_child_key("ns[0]", "ns", ":"), eq(true));
        assert_that!(is_child_key("a/b/c", "a", "/"), eq(true));
        assert_that!(is_child_key("a/b/c", "a/b", "/"), eq(true));
    }
}
