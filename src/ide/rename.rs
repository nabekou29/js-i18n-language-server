//! Rename support for translation keys.

use std::collections::HashMap;
use std::path::PathBuf;

use tower_lsp::lsp_types::{
    TextEdit,
    Url,
    WorkspaceEdit,
};

use crate::db::I18nDatabase;
use crate::ide::code_actions::{
    create_full_file_text_edit,
    rename_key_in_json_text,
};
use crate::ide::namespace::{
    filter_by_namespace,
    resolve_namespace,
};
use crate::input::source::SourceFile;
use crate::input::translation::Translation;
use crate::syntax::analyze_source;
use crate::syntax::analyzer::extractor::parse_key_with_namespace;

/// Computes workspace edits for renaming a translation key.
///
/// Updates both translation JSON files and source file references.
/// Supports namespace-prefixed keys (e.g., `"ns:key"`); namespace changes are rejected.
/// `target_namespace` is the resolved namespace from `KeyContext`, used to filter
/// source file usages when the namespace isn't explicit in the key text.
#[must_use]
#[allow(clippy::implicit_hasher, clippy::too_many_arguments)]
pub fn compute_rename_edits(
    db: &dyn I18nDatabase,
    old_key: &str,
    new_key: &str,
    target_namespace: Option<&str>,
    translations: &[Translation],
    source_files: &HashMap<PathBuf, SourceFile>,
    key_separator: &str,
    namespace_separator: Option<&str>,
    default_namespace: Option<&str>,
) -> WorkspaceEdit {
    let (old_ns, old_key_part) = parse_key_with_namespace(old_key, namespace_separator);
    let (new_ns, new_key_part) = parse_key_with_namespace(new_key, namespace_separator);

    // Reject namespace change
    if old_ns != new_ns {
        return WorkspaceEdit::default();
    }

    let mut changes: HashMap<Url, Vec<TextEdit>> = HashMap::new();

    // Use explicit namespace from key text, falling back to resolved target namespace
    let effective_ns = old_ns.as_deref().or(target_namespace);

    let target_translations = filter_by_namespace(db, translations, effective_ns);

    // Translation file edits
    for translation in &target_translations {
        let json_text = translation.json_text(db);
        if let Some(result) =
            rename_key_in_json_text(json_text, &old_key_part, &new_key_part, key_separator)
        {
            let file_path = translation.file_path(db);
            if let Ok(uri) = Url::from_file_path(file_path.as_str()) {
                let edit = create_full_file_text_edit(json_text, result.new_text);
                changes.entry(uri).or_default().push(edit);
            }
        }
    }

    // Source file edits: find references and replace key text
    for source_file in source_files.values() {
        let usages = analyze_source(db, *source_file, key_separator.to_string());
        let uri_str = source_file.uri(db);
        let Ok(uri) = uri_str.parse::<Url>() else {
            continue;
        };

        for usage in &usages {
            let usage_key_text = usage.key(db).text(db);
            let (usage_explicit_ns, usage_key_part) =
                parse_key_with_namespace(usage_key_text, namespace_separator);

            // Match key part
            if usage_key_part != old_key_part {
                continue;
            }

            // Match namespace when target has one
            if let Some(target_ns) = effective_ns {
                let declared_ns = usage.namespace(db);
                let declared_nss = usage.namespaces(db);
                let usage_ns = resolve_namespace(
                    usage_explicit_ns.as_deref(),
                    declared_ns.as_deref(),
                    declared_nss.as_deref(),
                    default_namespace,
                );
                if usage_ns.is_none_or(|ns| ns != target_ns) {
                    continue;
                }
            }

            let range = usage.range(db);
            let edit = TextEdit { range: range.to_unquoted_range(), new_text: new_key.to_string() };
            changes.entry(uri.clone()).or_default().push(edit);
        }
    }

    WorkspaceEdit { changes: Some(changes), ..Default::default() }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::expect_used)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;

    use googletest::prelude::*;
    use rstest::*;

    use super::*;
    use crate::db::I18nDatabaseImpl;
    use crate::input::source::{
        ProgrammingLanguage,
        SourceFile,
    };
    use crate::test_utils::create_translation_with_json;

    #[rstest]
    fn rename_updates_translation_files() {
        let db = I18nDatabaseImpl::default();

        let json_en = r#"{
  "hello": "Hello",
  "world": "World"
}"#;
        let json_ja = r#"{
  "hello": "こんにちは",
  "world": "世界"
}"#;

        let en = create_translation_with_json(
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
        let ja = create_translation_with_json(
            &db,
            "ja",
            None,
            "/locales/ja.json",
            HashMap::from([
                ("hello".to_string(), "こんにちは".to_string()),
                ("world".to_string(), "世界".to_string()),
            ]),
            json_ja,
        );
        let translations = vec![en, ja];

        let result = compute_rename_edits(
            &db,
            "hello",
            "greeting",
            None,
            &translations,
            &HashMap::new(),
            ".",
            None,
            None,
        );

        let changes = result.changes.unwrap();
        assert_that!(changes.len(), eq(2));

        let en_uri = Url::from_file_path("/locales/en.json").unwrap();
        let en_edits = &changes[&en_uri];
        assert_that!(en_edits.len(), eq(1));
        assert_that!(en_edits[0].new_text, contains_substring("\"greeting\": \"Hello\""));
        assert_that!(en_edits[0].new_text, not(contains_substring("\"hello\"")));

        let ja_uri = Url::from_file_path("/locales/ja.json").unwrap();
        let ja_edits = &changes[&ja_uri];
        assert_that!(ja_edits.len(), eq(1));
        assert_that!(ja_edits[0].new_text, contains_substring("\"greeting\": \"こんにちは\""));
    }

    #[rstest]
    fn rename_updates_source_references() {
        let db = I18nDatabaseImpl::default();

        let source_code = r#"const msg = t("common.hello");"#;
        let source_file = SourceFile::new(
            &db,
            "file:///src/app.ts".to_string(),
            source_code.to_string(),
            ProgrammingLanguage::TypeScript,
        );

        let mut source_files = HashMap::new();
        source_files.insert(PathBuf::from("/src/app.ts"), source_file);

        let result = compute_rename_edits(
            &db,
            "common.hello",
            "common.greeting",
            None,
            &[],
            &source_files,
            ".",
            None,
            None,
        );

        let changes = result.changes.unwrap();
        let source_uri: Url = "file:///src/app.ts".parse().unwrap();
        let edits = &changes[&source_uri];
        assert_that!(edits.len(), eq(1));
        assert_that!(edits[0].new_text, eq("common.greeting"));
    }

    #[rstest]
    fn rename_with_namespace_filters_translations() {
        let db = I18nDatabaseImpl::default();

        let common_json = r#"{
  "hello": "Hello"
}"#;
        let errors_json = r#"{
  "hello": "Error Hello"
}"#;

        let common = create_translation_with_json(
            &db,
            "en",
            Some("common"),
            "/locales/en/common.json",
            HashMap::from([("hello".to_string(), "Hello".to_string())]),
            common_json,
        );
        let errors = create_translation_with_json(
            &db,
            "en",
            Some("errors"),
            "/locales/en/errors.json",
            HashMap::from([("hello".to_string(), "Error Hello".to_string())]),
            errors_json,
        );
        let translations = vec![common, errors];

        let result = compute_rename_edits(
            &db,
            "common:hello",
            "common:greeting",
            Some("common"),
            &translations,
            &HashMap::new(),
            ".",
            Some(":"),
            None,
        );

        let changes = result.changes.unwrap();
        // Only common namespace should be updated
        assert_that!(changes.len(), eq(1));
        let common_uri = Url::from_file_path("/locales/en/common.json").unwrap();
        assert!(changes.contains_key(&common_uri));
    }

    #[rstest]
    fn rename_rejects_namespace_change() {
        let db = I18nDatabaseImpl::default();

        let result = compute_rename_edits(
            &db,
            "common:hello",
            "errors:hello",
            Some("common"),
            &[],
            &HashMap::new(),
            ".",
            Some(":"),
            None,
        );

        assert_that!(result.changes.unwrap_or_default(), is_empty());
    }
}
