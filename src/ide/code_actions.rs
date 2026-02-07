//! Code action generation for translation keys

use std::collections::HashMap;
use std::collections::HashSet;

use jsonc_parser::ParseOptions;
use jsonc_parser::cst::{
    CstInputValue,
    CstRootNode,
};
use tower_lsp::lsp_types::{
    CodeAction,
    CodeActionKind,
    CodeActionOrCommand,
    Command,
    Diagnostic,
    NumberOrString,
    TextEdit,
    Url,
    WorkspaceEdit,
};

use crate::db::I18nDatabase;
use crate::input::translation::Translation;
use crate::syntax::analyzer::extractor::parse_key_with_namespace;

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

/// Generate code actions for all languages, sorted by priority (primary > missing > others).
/// Returns commands with `{ lang, key }` args; the client handles value input.
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
                command: "i18n.executeClientEditTranslation".to_string(),
                arguments: Some(vec![serde_json::json!({
                    "lang": lang,
                    "key": key,
                })]),
            })
        })
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

/// Rename a key in JSON using CST to preserve formatting and property order.
/// Uses pivot object strategy: finds common prefix of old/new paths, operates below the pivot.
#[must_use]
pub fn rename_key_in_json_text(
    json_text: &str,
    old_key: &str,
    new_key: &str,
    separator: &str,
) -> Option<KeyEditResult> {
    if old_key == new_key {
        return None;
    }

    let old_parts: Vec<&str> = old_key.split(separator).collect();
    let new_parts: Vec<&str> = new_key.split(separator).collect();

    // Reject if one key is a prefix of the other
    let common_len = old_parts.iter().zip(new_parts.iter()).take_while(|(a, b)| a == b).count();
    if common_len == old_parts.len() || common_len == new_parts.len() {
        return None;
    }

    let root = CstRootNode::parse(json_text, &ParseOptions::default()).ok()?;
    let root_obj = root.object_value()?;

    // Read value at old_key path
    let value = {
        let mut current = root_obj.clone();
        let mut val = None;
        for (i, part) in old_parts.iter().enumerate() {
            if i == old_parts.len() - 1 {
                let prop = current.get(part)?;
                val = prop
                    .value()
                    .and_then(|v| v.as_string_lit())
                    .and_then(|s| s.decoded_value().ok());
            } else {
                current = current.object_value(part)?;
            }
        }
        val?
    };

    // Check new_key doesn't already exist
    {
        let mut current = root_obj.clone();
        let mut exists = false;
        for (i, part) in new_parts.iter().enumerate() {
            if i == new_parts.len() - 1 {
                if current.get(part).is_some() {
                    exists = true;
                }
            } else {
                match current.object_value(part) {
                    Some(child) => current = child,
                    None => break,
                }
            }
        }
        if exists {
            return None;
        }
    }

    // Navigate to pivot object (at common prefix)
    let mut pivot = root_obj.clone();
    for part in &old_parts[..common_len] {
        pivot = pivot.object_value(part)?;
    }

    // Delete old suffix from pivot
    let old_suffix_key: String = old_parts[common_len..].join(separator);
    delete_single_key(&pivot, &old_suffix_key, separator);

    // Cleanup empty objects under pivot (pivot itself is preserved)
    cleanup_empty_objects(&pivot);

    // Insert new suffix with preserved value
    let new_suffix = &new_parts[common_len..];
    let mut current = pivot;
    for (i, part) in new_suffix.iter().enumerate() {
        if i == new_suffix.len() - 1 {
            current.append(part, CstInputValue::String(value.clone()));
        } else {
            current = current.object_value_or_set(part);
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

/// Generate a code action to delete a translation key from all translation files.
/// Returns `None` if the key is not found in any translation.
#[must_use]
pub fn generate_delete_key_code_action(
    db: &dyn I18nDatabase,
    key: &str,
    translations: &[Translation],
    key_separator: &str,
    namespace_separator: Option<&str>,
) -> Option<CodeActionOrCommand> {
    let (ns, key_part) = parse_key_with_namespace(key, namespace_separator);

    let target_translations: Vec<&Translation> = if let Some(ref ns) = ns {
        translations.iter().filter(|t| t.namespace(db).as_ref().is_some_and(|n| n == ns)).collect()
    } else {
        translations.iter().collect()
    };

    let mut changes: HashMap<Url, Vec<TextEdit>> = HashMap::new();

    for translation in &target_translations {
        let json_text = translation.json_text(db);
        let result = delete_keys_from_json_text(json_text, &[key_part.clone()], key_separator);
        if let Some(result) = result {
            if result.deleted_count == 0 {
                continue;
            }
            let file_path = translation.file_path(db);
            if let Ok(uri) = Url::from_file_path(file_path.as_str()) {
                let line_count = json_text.lines().count() as u32;
                let last_line_len = json_text.lines().last().map_or(0, |l| l.len()) as u32;
                let edit = TextEdit {
                    range: tower_lsp::lsp_types::Range {
                        start: tower_lsp::lsp_types::Position { line: 0, character: 0 },
                        end: tower_lsp::lsp_types::Position {
                            line: line_count.saturating_sub(1),
                            character: last_line_len,
                        },
                    },
                    new_text: result.new_text,
                };
                changes.entry(uri).or_default().push(edit);
            }
        }
    }

    if changes.is_empty() {
        return None;
    }

    Some(CodeActionOrCommand::CodeAction(CodeAction {
        title: format!("Delete '{key_part}' from all translations"),
        kind: Some(CodeActionKind::REFACTOR),
        edit: Some(WorkspaceEdit { changes: Some(changes), ..Default::default() }),
        ..Default::default()
    }))
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::iter_on_single_items,
    clippy::redundant_closure_for_method_calls,
    clippy::panic,
    clippy::wildcard_enum_match_arm,
    clippy::match_wildcard_for_single_variants
)]
mod tests {
    use googletest::prelude::*;
    use rstest::*;

    use super::*;

    #[rstest]
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

        assert_that!(result, len(eq(2)));
        assert_that!(result, contains(eq(&"ja".to_string())));
        assert_that!(result, contains(eq(&"zh".to_string())));
    }

    #[rstest]
    fn test_extract_missing_languages_empty() {
        let diagnostics = vec![Diagnostic {
            code: Some(NumberOrString::String("other-diagnostic".to_string())),
            data: None,
            ..Default::default()
        }];

        let result = extract_missing_languages(&diagnostics);

        assert_that!(result, is_empty());
    }

    #[rstest]
    fn generate_code_actions_basic() {
        let languages = vec!["en".to_string(), "ja".to_string()];
        let missing = HashSet::new();

        let actions = generate_code_actions("common.hello", &languages, &missing, None);

        assert_that!(actions, len(eq(2)));
        // Both are "Edit" since none are missing
        let titles: Vec<_> = actions
            .iter()
            .map(|a| match a {
                CodeActionOrCommand::Command(c) => c.title.clone(),
                _ => panic!("expected Command"),
            })
            .collect();
        assert_that!(titles, each(contains_substring("Edit translation for")));
    }

    #[rstest]
    fn generate_code_actions_with_missing() {
        let languages = vec!["en".to_string(), "ja".to_string()];
        let missing: HashSet<String> = ["ja".to_string()].into();

        let actions = generate_code_actions("common.hello", &languages, &missing, None);

        let titles: Vec<_> = actions
            .iter()
            .map(|a| match a {
                CodeActionOrCommand::Command(c) => c.title.clone(),
                _ => panic!("expected Command"),
            })
            .collect();
        // "ja" is missing so sorted first, then "en"
        assert_that!(titles[0], eq("Add translation for ja"));
        assert_that!(titles[1], eq("Edit translation for en"));
    }

    #[rstest]
    fn generate_code_actions_with_primary() {
        let languages = vec!["en".to_string(), "ja".to_string(), "zh".to_string()];
        let missing = HashSet::new();

        let actions = generate_code_actions("common.hello", &languages, &missing, Some("ja"));

        let first_title = match &actions[0] {
            CodeActionOrCommand::Command(c) => &c.title,
            _ => panic!("expected Command"),
        };
        assert_that!(first_title, eq("Edit translation for ja"));
    }

    #[rstest]
    fn generate_code_actions_args_format() {
        let languages = vec!["en".to_string()];
        let missing = HashSet::new();

        let actions = generate_code_actions("greeting.hello", &languages, &missing, None);

        let args = match &actions[0] {
            CodeActionOrCommand::Command(c) => c.arguments.as_ref().unwrap(),
            _ => panic!("expected Command"),
        };
        let arg = &args[0];
        assert_that!(arg["lang"].as_str().unwrap(), eq("en"));
        assert_that!(arg["key"].as_str().unwrap(), eq("greeting.hello"));
    }

    #[rstest]
    fn test_insert_key_flat() {
        let json = r#"{
  "hello": "world"
}"#;

        let result = insert_key_to_json_text(json, "goodbye", "さようなら", ".")
            .expect("insertion should succeed");

        assert_that!(result.new_text, contains_substring("\"goodbye\""));
        assert_that!(result.new_text, contains_substring("\"goodbye\": \"さようなら\""));
        assert_that!(result.new_text, contains_substring("\"hello\": \"world\""));
    }

    #[rstest]
    fn test_insert_key_nested_new_parent() {
        let json = r#"{
  "hello": "world"
}"#;

        let result = insert_key_to_json_text(json, "common.greeting", "こんにちは", ".")
            .expect("insertion should succeed");

        assert_that!(result.new_text, contains_substring("\"common\""));
        assert_that!(result.new_text, contains_substring("\"greeting\""));
        assert_that!(result.new_text, contains_substring("\"greeting\": \"こんにちは\""));
    }

    #[rstest]
    fn test_insert_key_nested_existing_parent() {
        let json = r#"{
  "common": {
    "hello": "こんにちは"
  }
}"#;

        let result = insert_key_to_json_text(json, "common.goodbye", "さようなら", ".")
            .expect("insertion should succeed");

        assert_that!(result.new_text, contains_substring("\"goodbye\": \"さようなら\""));
        assert_that!(result.new_text, contains_substring("\"hello\": \"こんにちは\""));
    }

    #[rstest]
    fn test_insert_key_preserves_formatting() {
        let json = r#"{
    "existing": "value"
}"#;

        let result = insert_key_to_json_text(json, "new", "new_value", ".")
            .expect("insertion should succeed");

        assert_that!(result.new_text, contains_substring("    \"existing\""));
    }

    #[rstest]
    fn test_update_key_value() {
        let json = r#"{
  "hello": "world"
}"#;

        let result =
            update_key_in_json_text(json, "hello", "updated", ".").expect("update should succeed");

        assert_that!(result.new_text, contains_substring("\"hello\": \"updated\""));
    }

    #[rstest]
    fn test_update_nested_key_value() {
        let json = r#"{
  "common": {
    "hello": "world"
  }
}"#;

        let result = update_key_in_json_text(json, "common.hello", "こんにちは", ".")
            .expect("update should succeed");

        assert_that!(result.new_text, contains_substring("\"hello\": \"こんにちは\""));
        assert_that!(result.new_text, contains_substring("\"common\""));
    }

    #[rstest]
    fn test_update_nonexistent_key_returns_none() {
        let json = r#"{
  "hello": "world"
}"#;

        let result = update_key_in_json_text(json, "nonexistent", "value", ".");

        assert_that!(result, none());
    }

    #[rstest]
    fn test_delete_single_key() {
        let json = r#"{
  "hello": "world",
  "unused": "value"
}"#;
        let result = delete_keys_from_json_text(json, &["unused".to_string()], ".")
            .expect("deletion should succeed");

        assert_that!(result.deleted_count, eq(1));
        assert_that!(result.new_text, not(contains_substring("\"unused\"")));
        assert_that!(result.new_text, contains_substring("\"hello\""));
    }

    #[rstest]
    fn test_delete_nested_key() {
        let json = r#"{
  "common": {
    "used": "value",
    "unused": "value"
  }
}"#;
        let result = delete_keys_from_json_text(json, &["common.unused".to_string()], ".")
            .expect("deletion should succeed");

        assert_that!(result.deleted_count, eq(1));
        assert_that!(result.new_text, not(contains_substring("\"unused\"")));
        assert_that!(result.new_text, contains_substring("\"used\""));
        assert_that!(result.new_text, contains_substring("\"common\""));
    }

    #[rstest]
    fn test_delete_cleanup_empty_parent() {
        let json = r#"{
  "used": "value",
  "empty_parent": {
    "unused": "value"
  }
}"#;
        let result = delete_keys_from_json_text(json, &["empty_parent.unused".to_string()], ".")
            .expect("deletion should succeed");

        assert_that!(result.new_text, not(contains_substring("\"empty_parent\"")));
        assert_that!(result.new_text, contains_substring("\"used\""));
    }

    #[rstest]
    fn test_delete_preserves_formatting() {
        let json = r#"{
    "used": "value",
    "unused": "value"
}"#;
        let result = delete_keys_from_json_text(json, &["unused".to_string()], ".")
            .expect("deletion should succeed");

        assert_that!(result.new_text, contains_substring("    \"used\""));
    }

    #[rstest]
    fn test_delete_multiple_keys() {
        let json = r#"{
  "a": "1",
  "b": "2",
  "c": "3"
}"#;
        let result = delete_keys_from_json_text(json, &["a".to_string(), "c".to_string()], ".")
            .expect("deletion should succeed");

        assert_that!(result.deleted_count, eq(2));
        assert_that!(result.new_text, not(contains_substring("\"a\"")));
        assert_that!(result.new_text, not(contains_substring("\"c\"")));
        assert_that!(result.new_text, contains_substring("\"b\""));
    }

    #[rstest]
    fn test_delete_nonexistent_key() {
        let json = r#"{
  "hello": "world"
}"#;
        let result = delete_keys_from_json_text(json, &["nonexistent".to_string()], ".")
            .expect("deletion should succeed");

        assert_that!(result.deleted_count, eq(0));
        assert_that!(result.new_text, contains_substring("\"hello\""));
    }

    #[rstest]
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

        assert_that!(result.new_text, not(contains_substring("\"deep\"")));
        assert_that!(result.new_text, not(contains_substring("\"nested\"")));
        assert_that!(result.new_text, contains_substring("\"keep\""));
    }

    #[rstest]
    fn test_delete_empty_keys_list() {
        let json = r#"{
  "hello": "world"
}"#;
        let result = delete_keys_from_json_text(json, &[], ".").expect("deletion should succeed");

        assert_that!(result.deleted_count, eq(0));
        assert_that!(result.new_text, contains_substring("\"hello\""));
    }

    // === rename_key_in_json_text tests ===

    #[rstest]
    fn rename_key_same_parent() {
        // Case 1: a.b → a.c (pivot = a)
        let json = r#"{
  "a": {
    "b": "hello",
    "x": "other"
  }
}"#;

        let result =
            rename_key_in_json_text(json, "a.b", "a.c", ".").expect("rename should succeed");

        assert_that!(result.new_text, not(contains_substring("\"b\"")));
        assert_that!(result.new_text, contains_substring("\"c\": \"hello\""));
        assert_that!(result.new_text, contains_substring("\"x\": \"other\""));
        assert_that!(result.new_text, contains_substring("\"a\""));
    }

    #[rstest]
    fn rename_key_different_parent_empty() {
        // Case 2: a.b → c.d (pivot = root, a becomes empty)
        let json = r#"{
  "a": {
    "b": "hello"
  }
}"#;

        let result =
            rename_key_in_json_text(json, "a.b", "c.d", ".").expect("rename should succeed");

        assert_that!(result.new_text, not(contains_substring("\"a\"")));
        assert_that!(result.new_text, contains_substring("\"c\""));
        assert_that!(result.new_text, contains_substring("\"d\": \"hello\""));
    }

    #[rstest]
    fn rename_key_different_parent_with_siblings() {
        // Case 3: a.b → c.d (pivot = root, a has sibling x)
        let json = r#"{
  "a": {
    "b": "hello",
    "x": "other"
  }
}"#;

        let result =
            rename_key_in_json_text(json, "a.b", "c.d", ".").expect("rename should succeed");

        assert_that!(result.new_text, contains_substring("\"a\""));
        assert_that!(result.new_text, contains_substring("\"x\": \"other\""));
        assert_that!(result.new_text, not(contains_substring("\"b\"")));
        assert_that!(result.new_text, contains_substring("\"c\""));
        assert_that!(result.new_text, contains_substring("\"d\": \"hello\""));
    }

    #[rstest]
    fn rename_key_deep_nested_no_siblings_preserves_order() {
        // Case 4: a.b.c → a.b.d (pivot = a.b, no siblings)
        // Key point: a and a.b positions must be preserved
        let json = r#"{
  "x": "first",
  "a": {
    "b": {
      "c": "hello"
    }
  },
  "y": "last"
}"#;

        let result =
            rename_key_in_json_text(json, "a.b.c", "a.b.d", ".").expect("rename should succeed");

        assert_that!(result.new_text, contains_substring("\"d\": \"hello\""));
        assert_that!(result.new_text, not(contains_substring("\"c\"")));
        // Verify order is preserved: x before a, a before y
        let x_pos = result.new_text.find("\"x\"").unwrap();
        let a_pos = result.new_text.find("\"a\"").unwrap();
        let y_pos = result.new_text.find("\"y\"").unwrap();
        assert!(x_pos < a_pos, "x should come before a");
        assert!(a_pos < y_pos, "a should come before y");
    }

    #[rstest]
    fn rename_key_deep_nested_with_siblings() {
        // Case 5: a.b.c → a.b.d (pivot = a.b, c has sibling x)
        let json = r#"{
  "a": {
    "b": {
      "c": "hello",
      "x": "other"
    }
  }
}"#;

        let result =
            rename_key_in_json_text(json, "a.b.c", "a.b.d", ".").expect("rename should succeed");

        assert_that!(result.new_text, not(contains_substring("\"c\"")));
        assert_that!(result.new_text, contains_substring("\"d\": \"hello\""));
        assert_that!(result.new_text, contains_substring("\"x\": \"other\""));
    }

    #[rstest]
    fn rename_key_mid_path_diverge() {
        // Case 6: a.b.c → a.x.y (pivot = a)
        let json = r#"{
  "a": {
    "b": {
      "c": "hello"
    }
  }
}"#;

        let result =
            rename_key_in_json_text(json, "a.b.c", "a.x.y", ".").expect("rename should succeed");

        assert_that!(result.new_text, contains_substring("\"a\""));
        assert_that!(result.new_text, not(contains_substring("\"b\"")));
        assert_that!(result.new_text, contains_substring("\"x\""));
        assert_that!(result.new_text, contains_substring("\"y\": \"hello\""));
    }

    #[rstest]
    fn rename_key_flat() {
        // Simple flat key rename: a → b
        let json = r#"{
  "a": "hello",
  "x": "other"
}"#;

        let result = rename_key_in_json_text(json, "a", "b", ".").expect("rename should succeed");

        assert_that!(result.new_text, not(contains_substring("\"a\"")));
        assert_that!(result.new_text, contains_substring("\"b\": \"hello\""));
        assert_that!(result.new_text, contains_substring("\"x\": \"other\""));
    }

    #[rstest]
    fn rename_key_same_key_returns_none() {
        let json = r#"{ "a": "hello" }"#;

        let result = rename_key_in_json_text(json, "a", "a", ".");

        assert_that!(result, none());
    }

    #[rstest]
    fn rename_key_old_not_found_returns_none() {
        let json = r#"{ "a": "hello" }"#;

        let result = rename_key_in_json_text(json, "nonexistent", "b", ".");

        assert_that!(result, none());
    }

    #[rstest]
    fn rename_key_new_already_exists_returns_none() {
        let json = r#"{ "a": "hello", "b": "world" }"#;

        let result = rename_key_in_json_text(json, "a", "b", ".");

        assert_that!(result, none());
    }

    #[rstest]
    fn rename_key_prefix_relation_returns_none() {
        // old key is prefix of new key
        let json = r#"{ "a": { "b": "hello" } }"#;

        let result = rename_key_in_json_text(json, "a.b", "a.b.c", ".");

        assert_that!(result, none());
    }

    // === generate_delete_key_code_action tests ===

    use crate::db::I18nDatabaseImpl;

    fn create_test_translation(
        db: &I18nDatabaseImpl,
        language: &str,
        namespace: Option<&str>,
        file_path: &str,
        keys: HashMap<String, String>,
        json_text: &str,
    ) -> Translation {
        Translation::new(
            db,
            language.to_string(),
            namespace.map(String::from),
            file_path.to_string(),
            keys,
            json_text.to_string(),
            HashMap::new(),
            HashMap::new(),
        )
    }

    #[rstest]
    fn delete_key_action_basic() {
        let db = I18nDatabaseImpl::default();
        let json_en = r#"{
  "hello": "Hello",
  "world": "World"
}"#;
        let en = create_test_translation(
            &db,
            "en",
            None,
            "/locales/en.json",
            HashMap::from([
                ("hello".to_string(), "Hello".to_string()),
                ("world".to_string(), "World".to_string()),
            ]),
            json_en,
        );

        let result = generate_delete_key_code_action(&db, "hello", &[en], ".", None);

        assert_that!(result, some(anything()));
        let action = match result.unwrap() {
            CodeActionOrCommand::CodeAction(a) => a,
            _ => panic!("expected CodeAction"),
        };
        assert_that!(action.title, eq("Delete 'hello' from all translations"));
        assert_that!(action.kind.as_ref(), some(eq(&CodeActionKind::REFACTOR)));

        let edit = action.edit.expect("should have workspace edit");
        let changes = edit.changes.expect("should have changes");
        let en_uri = Url::from_file_path("/locales/en.json").unwrap();
        let en_edits = &changes[&en_uri];
        assert_that!(en_edits.len(), eq(1));
        assert_that!(en_edits[0].new_text, not(contains_substring("\"hello\"")));
        assert_that!(en_edits[0].new_text, contains_substring("\"world\""));
    }

    #[rstest]
    fn delete_key_action_multiple_languages() {
        let db = I18nDatabaseImpl::default();
        let json_en = r#"{ "hello": "Hello" }"#;
        let json_ja = r#"{ "hello": "こんにちは" }"#;

        let en = create_test_translation(
            &db,
            "en",
            None,
            "/locales/en.json",
            HashMap::from([("hello".to_string(), "Hello".to_string())]),
            json_en,
        );
        let ja = create_test_translation(
            &db,
            "ja",
            None,
            "/locales/ja.json",
            HashMap::from([("hello".to_string(), "こんにちは".to_string())]),
            json_ja,
        );

        let result = generate_delete_key_code_action(&db, "hello", &[en, ja], ".", None);

        let action = match result.unwrap() {
            CodeActionOrCommand::CodeAction(a) => a,
            _ => panic!("expected CodeAction"),
        };
        let changes = action.edit.unwrap().changes.unwrap();
        assert_that!(changes.len(), eq(2));
    }

    #[rstest]
    fn delete_key_action_not_found_returns_none() {
        let db = I18nDatabaseImpl::default();
        let json = r#"{ "hello": "Hello" }"#;
        let en = create_test_translation(
            &db,
            "en",
            None,
            "/locales/en.json",
            HashMap::from([("hello".to_string(), "Hello".to_string())]),
            json,
        );

        let result = generate_delete_key_code_action(&db, "nonexistent", &[en], ".", None);

        assert_that!(result, none());
    }

    #[rstest]
    fn delete_key_action_with_namespace() {
        let db = I18nDatabaseImpl::default();
        let common_json = r#"{ "hello": "Hello" }"#;
        let errors_json = r#"{ "hello": "Error Hello" }"#;

        let common = create_test_translation(
            &db,
            "en",
            Some("common"),
            "/locales/en/common.json",
            HashMap::from([("hello".to_string(), "Hello".to_string())]),
            common_json,
        );
        let errors = create_test_translation(
            &db,
            "en",
            Some("errors"),
            "/locales/en/errors.json",
            HashMap::from([("hello".to_string(), "Error Hello".to_string())]),
            errors_json,
        );

        let result =
            generate_delete_key_code_action(&db, "common:hello", &[common, errors], ".", Some(":"));

        let action = match result.unwrap() {
            CodeActionOrCommand::CodeAction(a) => a,
            _ => panic!("expected CodeAction"),
        };
        // Title should show key_part without namespace
        assert_that!(action.title, eq("Delete 'hello' from all translations"));
        let changes = action.edit.unwrap().changes.unwrap();
        // Only common namespace should be affected
        assert_that!(changes.len(), eq(1));
        let common_uri = Url::from_file_path("/locales/en/common.json").unwrap();
        assert!(changes.contains_key(&common_uri));
    }

    #[rstest]
    fn delete_key_action_nested_key() {
        let db = I18nDatabaseImpl::default();
        let json = r#"{
  "common": {
    "hello": "Hello"
  }
}"#;
        let en = create_test_translation(
            &db,
            "en",
            None,
            "/locales/en.json",
            HashMap::from([("common.hello".to_string(), "Hello".to_string())]),
            json,
        );

        let result = generate_delete_key_code_action(&db, "common.hello", &[en], ".", None);

        let action = match result.unwrap() {
            CodeActionOrCommand::CodeAction(a) => a,
            _ => panic!("expected CodeAction"),
        };
        let changes = action.edit.unwrap().changes.unwrap();
        let en_uri = Url::from_file_path("/locales/en.json").unwrap();
        let new_text = &changes[&en_uri][0].new_text;
        // Nested key deleted, empty parent cleaned up
        assert_that!(new_text, not(contains_substring("\"common\"")));
        assert_that!(new_text, not(contains_substring("\"hello\"")));
    }
}
