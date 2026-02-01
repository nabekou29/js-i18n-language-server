//! Code action generation for translation keys

use std::collections::HashSet;

use jsonc_parser::ParseOptions;
use jsonc_parser::cst::{
    CstInputValue,
    CstRootNode,
};
use tower_lsp::lsp_types::{
    Diagnostic,
    NumberOrString,
};

use crate::db::I18nDatabase;
use crate::input::translation::Translation;

/// Result of CST-based key insertion or update, preserving original formatting.
#[derive(Debug, Clone)]
pub struct KeyEditResult {
    pub new_text: String,
}

/// Result of CST-based key deletion, preserving original formatting.
#[derive(Debug, Clone)]
pub struct KeyDeletionResult {
    pub new_text: String,
    pub deleted_count: usize,
    pub deleted_keys: Vec<String>,
}

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

/// Insert a key with a value into a JSON translation file using CST to preserve formatting.
/// Supports nested keys (e.g., `common.hello`).
#[must_use]
pub fn insert_key_to_json(
    db: &dyn I18nDatabase,
    translation: &Translation,
    key: &str,
    value: &str,
    separator: &str,
) -> Option<KeyEditResult> {
    let json_text = translation.json_text(db);
    insert_key_to_json_text(json_text, key, value, separator)
}

#[must_use]
pub fn insert_key_to_json_text(
    json_text: &str,
    key: &str,
    value: &str,
    separator: &str,
) -> Option<KeyEditResult> {
    let root = CstRootNode::parse(json_text, &ParseOptions::default()).ok()?;
    let root_obj = root.object_value_or_set();

    let key_parts: Vec<&str> = key.split(separator).collect();

    let mut current_obj = root_obj;
    for (i, part) in key_parts.iter().enumerate() {
        if i == key_parts.len() - 1 {
            current_obj.append(part, CstInputValue::String(value.to_string()));
        } else {
            current_obj = current_obj.object_value_or_set(part);
        }
    }

    Some(KeyEditResult { new_text: root.to_string() })
}

/// Update an existing key's value in a JSON translation file using CST to preserve formatting.
/// Supports nested keys (e.g., `common.hello`).
#[must_use]
pub fn update_key_in_json_text(
    json_text: &str,
    key: &str,
    value: &str,
    separator: &str,
) -> Option<KeyEditResult> {
    let root = CstRootNode::parse(json_text, &ParseOptions::default()).ok()?;
    let root_obj = root.object_value()?;

    let key_parts: Vec<&str> = key.split(separator).collect();

    let mut current_obj = root_obj;
    for (i, part) in key_parts.iter().enumerate() {
        if i == key_parts.len() - 1 {
            let prop = current_obj.get(part)?;
            prop.set_value(CstInputValue::String(value.to_string()));
        } else {
            current_obj = current_obj.object_value(part)?;
        }
    }

    Some(KeyEditResult { new_text: root.to_string() })
}

/// Delete keys from JSON using CST to preserve formatting.
/// Empty parent objects are recursively removed after deletion.
#[must_use]
pub fn delete_keys_from_json_text(
    json_text: &str,
    keys_to_delete: &[String],
    separator: &str,
) -> Option<KeyDeletionResult> {
    let root = CstRootNode::parse(json_text, &ParseOptions::default()).ok()?;
    let root_obj = root.object_value()?;

    let mut deleted_keys = Vec::new();

    // Sort by depth (deepest first) to delete leaves before parents
    let mut sorted_keys: Vec<_> = keys_to_delete.to_vec();
    sorted_keys.sort_by(|a, b| {
        let depth_a = a.matches(separator).count();
        let depth_b = b.matches(separator).count();
        depth_b.cmp(&depth_a)
    });

    for key in &sorted_keys {
        if delete_single_key(&root_obj, key, separator) {
            deleted_keys.push(key.clone());
        }
    }

    cleanup_empty_objects(&root_obj);

    Some(KeyDeletionResult {
        new_text: root.to_string(),
        deleted_count: deleted_keys.len(),
        deleted_keys,
    })
}

fn delete_single_key(root_obj: &jsonc_parser::cst::CstObject, key: &str, separator: &str) -> bool {
    let parts: Vec<&str> = key.split(separator).collect();

    let mut current_obj = root_obj.clone();
    for (i, part) in parts.iter().enumerate() {
        if i == parts.len() - 1 {
            if let Some(prop) = current_obj.get(part) {
                prop.remove();
                return true;
            }
            return false;
        }
        match current_obj.object_value(part) {
            Some(child) => current_obj = child,
            None => return false,
        }
    }
    false
}

/// Recursively remove empty parent objects after key deletion.
fn cleanup_empty_objects(obj: &jsonc_parser::cst::CstObject) {
    // Limit iterations to prevent infinite loops
    for _ in 0..100 {
        let mut removed_any = false;

        // Collect properties first since iterator may be invalidated during removal
        let props: Vec<_> = obj.properties();

        for prop in props {
            if let Some(child_obj) = prop.value().and_then(|v| v.as_object()) {
                cleanup_empty_objects(&child_obj);

                if child_obj.properties().is_empty() {
                    prop.remove();
                    removed_any = true;
                }
            }
        }

        if !removed_any {
            break;
        }
    }
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
    fn test_insert_key_flat() {
        let json = r#"{
  "hello": "world"
}"#;

        let result = insert_key_to_json_text(json, "goodbye", "さようなら", ".")
            .expect("insertion should succeed");

        expect_that!(result.new_text, contains_substring("\"goodbye\""));
        expect_that!(result.new_text, contains_substring("\"goodbye\": \"さようなら\""));
        expect_that!(result.new_text, contains_substring("\"hello\": \"world\""));
    }

    #[googletest::test]
    fn test_insert_key_nested_new_parent() {
        let json = r#"{
  "hello": "world"
}"#;

        let result = insert_key_to_json_text(json, "common.greeting", "こんにちは", ".")
            .expect("insertion should succeed");

        expect_that!(result.new_text, contains_substring("\"common\""));
        expect_that!(result.new_text, contains_substring("\"greeting\""));
        expect_that!(result.new_text, contains_substring("\"greeting\": \"こんにちは\""));
    }

    #[googletest::test]
    fn test_insert_key_nested_existing_parent() {
        let json = r#"{
  "common": {
    "hello": "こんにちは"
  }
}"#;

        let result = insert_key_to_json_text(json, "common.goodbye", "さようなら", ".")
            .expect("insertion should succeed");

        expect_that!(result.new_text, contains_substring("\"goodbye\": \"さようなら\""));
        expect_that!(result.new_text, contains_substring("\"hello\": \"こんにちは\""));
    }

    #[googletest::test]
    fn test_insert_key_preserves_formatting() {
        let json = r#"{
    "existing": "value"
}"#;

        let result = insert_key_to_json_text(json, "new", "new_value", ".")
            .expect("insertion should succeed");

        expect_that!(result.new_text, contains_substring("    \"existing\""));
    }

    #[googletest::test]
    fn test_update_key_value() {
        let json = r#"{
  "hello": "world"
}"#;

        let result =
            update_key_in_json_text(json, "hello", "updated", ".").expect("update should succeed");

        expect_that!(result.new_text, contains_substring("\"hello\": \"updated\""));
    }

    #[googletest::test]
    fn test_update_nested_key_value() {
        let json = r#"{
  "common": {
    "hello": "world"
  }
}"#;

        let result = update_key_in_json_text(json, "common.hello", "こんにちは", ".")
            .expect("update should succeed");

        expect_that!(result.new_text, contains_substring("\"hello\": \"こんにちは\""));
        expect_that!(result.new_text, contains_substring("\"common\""));
    }

    #[googletest::test]
    fn test_update_nonexistent_key_returns_none() {
        let json = r#"{
  "hello": "world"
}"#;

        let result = update_key_in_json_text(json, "nonexistent", "value", ".");

        expect_that!(result, none());
    }

    #[googletest::test]
    fn test_delete_single_key() {
        let json = r#"{
  "hello": "world",
  "unused": "value"
}"#;
        let result = delete_keys_from_json_text(json, &["unused".to_string()], ".")
            .expect("deletion should succeed");

        expect_that!(result.deleted_count, eq(1));
        expect_that!(result.new_text, not(contains_substring("\"unused\"")));
        expect_that!(result.new_text, contains_substring("\"hello\""));
    }

    #[googletest::test]
    fn test_delete_nested_key() {
        let json = r#"{
  "common": {
    "used": "value",
    "unused": "value"
  }
}"#;
        let result = delete_keys_from_json_text(json, &["common.unused".to_string()], ".")
            .expect("deletion should succeed");

        expect_that!(result.deleted_count, eq(1));
        expect_that!(result.new_text, not(contains_substring("\"unused\"")));
        expect_that!(result.new_text, contains_substring("\"used\""));
        expect_that!(result.new_text, contains_substring("\"common\""));
    }

    #[googletest::test]
    fn test_delete_cleanup_empty_parent() {
        let json = r#"{
  "used": "value",
  "empty_parent": {
    "unused": "value"
  }
}"#;
        let result = delete_keys_from_json_text(json, &["empty_parent.unused".to_string()], ".")
            .expect("deletion should succeed");

        expect_that!(result.new_text, not(contains_substring("\"empty_parent\"")));
        expect_that!(result.new_text, contains_substring("\"used\""));
    }

    #[googletest::test]
    fn test_delete_preserves_formatting() {
        let json = r#"{
    "used": "value",
    "unused": "value"
}"#;
        let result = delete_keys_from_json_text(json, &["unused".to_string()], ".")
            .expect("deletion should succeed");

        expect_that!(result.new_text, contains_substring("    \"used\""));
    }

    #[googletest::test]
    fn test_delete_multiple_keys() {
        let json = r#"{
  "a": "1",
  "b": "2",
  "c": "3"
}"#;
        let result = delete_keys_from_json_text(json, &["a".to_string(), "c".to_string()], ".")
            .expect("deletion should succeed");

        expect_that!(result.deleted_count, eq(2));
        expect_that!(result.new_text, not(contains_substring("\"a\"")));
        expect_that!(result.new_text, not(contains_substring("\"c\"")));
        expect_that!(result.new_text, contains_substring("\"b\""));
    }

    #[googletest::test]
    fn test_delete_nonexistent_key() {
        let json = r#"{
  "hello": "world"
}"#;
        let result = delete_keys_from_json_text(json, &["nonexistent".to_string()], ".")
            .expect("deletion should succeed");

        expect_that!(result.deleted_count, eq(0));
        expect_that!(result.new_text, contains_substring("\"hello\""));
    }

    #[googletest::test]
    fn test_delete_deeply_nested_cleanup() {
        let json = r#"{
  "keep": "value",
  "deep": {
    "nested": {
      "unused": "value"
    }
  }
}"#;
        let result = delete_keys_from_json_text(json, &["deep.nested.unused".to_string()], ".")
            .expect("deletion should succeed");

        expect_that!(result.new_text, not(contains_substring("\"deep\"")));
        expect_that!(result.new_text, not(contains_substring("\"nested\"")));
        expect_that!(result.new_text, contains_substring("\"keep\""));
    }

    #[googletest::test]
    fn test_delete_empty_keys_list() {
        let json = r#"{
  "hello": "world"
}"#;
        let result = delete_keys_from_json_text(json, &[], ".").expect("deletion should succeed");

        expect_that!(result.deleted_count, eq(0));
        expect_that!(result.new_text, contains_substring("\"hello\""));
    }
}
