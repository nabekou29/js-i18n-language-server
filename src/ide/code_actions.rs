//! Code action generation for translation keys

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

/// Result of CST-based key insertion, preserving original formatting.
#[derive(Debug, Clone)]
pub struct KeyInsertionResult {
    pub new_text: String,
    pub cursor_range: Range,
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

/// Generate code actions for all languages, sorted by priority (primary > missing > others).
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

    // Sort: primary > missing > others (tuple comparison in descending order)
    languages.sort_by_key(|item| std::cmp::Reverse((item.1, item.2)));

    languages
        .into_iter()
        .map(|(lang, _, is_missing)| {
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

/// Insert a key into a JSON translation file using CST to preserve formatting.
/// Supports nested keys (e.g., `common.hello`).
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

#[must_use]
pub fn insert_key_to_json_text(
    json_text: &str,
    key: &str,
    separator: &str,
) -> Option<KeyInsertionResult> {
    let root = CstRootNode::parse(json_text, &ParseOptions::default()).ok()?;
    let root_obj = root.object_value_or_set();

    let key_parts: Vec<&str> = key.split(separator).collect();

    let mut current_obj = root_obj;
    for (i, part) in key_parts.iter().enumerate() {
        if i == key_parts.len() - 1 {
            current_obj.append(part, CstInputValue::String(String::new()));
        } else {
            current_obj = current_obj.object_value_or_set(part);
        }
    }

    let new_text = root.to_string();
    let cursor_range = find_cursor_position(&new_text, key, separator)?;

    Some(KeyInsertionResult { new_text, cursor_range })
}

/// Find cursor position inside the newly added empty string value.
#[allow(clippy::cast_possible_truncation)]
fn find_cursor_position(json_text: &str, key: &str, separator: &str) -> Option<Range> {
    let leaf_key = key.split(separator).last()?;
    let pattern = format!("\"{leaf_key}\": \"\"");

    // Search from end since newly added keys appear at the end
    let pos = json_text.rfind(&pattern)?;

    let before = &json_text[..pos];
    let line = before.matches('\n').count() as u32;
    let last_newline = before.rfind('\n').map_or(0, |i| i + 1);

    // Cursor position inside `""`: offset = 1(") + leaf_key.len() + 1(") + 1(:) + 1( ) + 1(") = leaf_key.len() + 5
    let col_start = (pos - last_newline + leaf_key.len() + 5) as u32;

    Some(Range {
        start: Position { line, character: col_start },
        end: Position { line, character: col_start },
    })
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

        if let CodeActionOrCommand::Command(cmd) = &actions[0] {
            expect_that!(cmd.title, contains_substring("zh"));
        }
    }

    #[googletest::test]
    fn test_generate_code_actions_priority_order() {
        let all_languages = vec!["en".to_string(), "ja".to_string(), "zh".to_string()];
        let missing_languages: HashSet<String> = ["zh"].iter().map(|s| s.to_string()).collect();

        let actions =
            generate_code_actions("common.hello", &all_languages, &missing_languages, Some("ja"));

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
        let json = r#"{
  "hello": "world"
}"#;

        let result =
            insert_key_to_json_text(json, "goodbye", ".").expect("insertion should succeed");

        expect_that!(result.new_text, contains_substring("\"goodbye\""));
        expect_that!(result.new_text, contains_substring("\"goodbye\": \"\""));
        expect_that!(result.new_text, contains_substring("\"hello\": \"world\""));
    }

    #[googletest::test]
    fn test_insert_key_nested_new_parent() {
        let json = r#"{
  "hello": "world"
}"#;

        let result = insert_key_to_json_text(json, "common.greeting", ".")
            .expect("insertion should succeed");

        expect_that!(result.new_text, contains_substring("\"common\""));
        expect_that!(result.new_text, contains_substring("\"greeting\""));
        expect_that!(result.new_text, contains_substring("\"greeting\": \"\""));
    }

    #[googletest::test]
    fn test_insert_key_nested_existing_parent() {
        let json = r#"{
  "common": {
    "hello": "こんにちは"
  }
}"#;

        let result =
            insert_key_to_json_text(json, "common.goodbye", ".").expect("insertion should succeed");

        expect_that!(result.new_text, contains_substring("\"goodbye\": \"\""));
        expect_that!(result.new_text, contains_substring("\"hello\": \"こんにちは\""));
    }

    #[googletest::test]
    fn test_insert_key_preserves_formatting() {
        let json = r#"{
    "existing": "value"
}"#;

        let result = insert_key_to_json_text(json, "new", ".").expect("insertion should succeed");

        expect_that!(result.new_text, contains_substring("    \"existing\""));
    }

    #[googletest::test]
    fn test_insert_key_cursor_position() {
        let json = r#"{"hello": "world"}"#;

        let result = insert_key_to_json_text(json, "new", ".").expect("insertion should succeed");

        expect_that!(result.cursor_range.start.line, ge(0));
        expect_that!(result.cursor_range.start.character, ge(0));
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
