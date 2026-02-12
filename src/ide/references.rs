//! References implementation

use std::collections::HashMap;
use std::path::PathBuf;

use tower_lsp::lsp_types::Location;

use crate::db::I18nDatabase;
use crate::ide::namespace::resolve_usage_namespace;
use crate::ide::plural::get_plural_base_key;
use crate::input::source::SourceFile;
use crate::syntax::analyze_source;

/// Finds all references to a translation key across all source files.
///
/// For plural keys (e.g., `items_one`), also matches calls to the base key (`items`)
/// since i18next resolves `t("items", { count: n })` to plural variants internally.
///
/// When `target_namespace` is `Some`, only usages whose resolved namespace matches
/// are returned. This prevents cross-namespace false positives when multiple
/// namespaces share the same key name.
pub fn find_references<S: std::hash::BuildHasher>(
    db: &dyn I18nDatabase,
    key_part: &str,
    target_namespace: Option<&str>,
    source_files: &HashMap<PathBuf, SourceFile, S>,
    key_separator: &str,
    namespace_separator: Option<&str>,
    default_namespace: Option<&str>,
) -> Vec<Location> {
    let base_key = get_plural_base_key(key_part);

    source_files
        .values()
        .flat_map(|source_file| {
            let usages = analyze_source(db, *source_file, key_separator.to_string());
            let uri = source_file.uri(db);

            usages.into_iter().filter_map(move |usage| {
                let (usage_ns, usage_key_part) =
                    resolve_usage_namespace(db, usage, namespace_separator, default_namespace);

                let is_key_match =
                    usage_key_part == key_part || base_key.is_some_and(|bk| usage_key_part == bk);

                if !is_key_match {
                    return None;
                }

                // When target has a namespace, verify the usage resolves to the same one
                if let Some(target_ns) = target_namespace
                    && usage_ns.as_deref().is_none_or(|ns| ns != target_ns)
                {
                    return None;
                }

                let Ok(parsed_uri) = uri.parse() else {
                    tracing::warn!("Failed to parse URI: {}", uri);
                    return None;
                };
                Some(Location { uri: parsed_uri, range: usage.range(db).into() })
            })
        })
        .collect()
}

#[cfg(test)]
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

    #[rstest]
    fn test_find_references_single_file() {
        let db = I18nDatabaseImpl::default();

        let source_code = r#"
            const msg1 = t("common.hello");
            const msg2 = t("common.goodbye");
            const msg3 = t("common.hello");
        "#;
        let source_file = SourceFile::new(
            &db,
            "file:///test.ts".to_string(),
            source_code.to_string(),
            ProgrammingLanguage::TypeScript,
        );

        let mut source_files = HashMap::new();
        source_files.insert(PathBuf::from("/test.ts"), source_file);

        let locations = find_references(&db, "common.hello", None, &source_files, ".", None, None);

        assert_that!(locations.len(), eq(2));

        for location in &locations {
            assert_that!(location.uri.path(), eq("/test.ts"));
        }
    }

    #[rstest]
    fn test_find_references_multiple_files() {
        let db = I18nDatabaseImpl::default();

        let source_code1 = r#"const msg = t("common.hello");"#;
        let source_file1 = SourceFile::new(
            &db,
            "file:///test1.ts".to_string(),
            source_code1.to_string(),
            ProgrammingLanguage::TypeScript,
        );

        let source_code2 = r#"const msg = t("common.hello");"#;
        let source_file2 = SourceFile::new(
            &db,
            "file:///test2.ts".to_string(),
            source_code2.to_string(),
            ProgrammingLanguage::TypeScript,
        );

        let mut source_files = HashMap::new();
        source_files.insert(PathBuf::from("/test1.ts"), source_file1);
        source_files.insert(PathBuf::from("/test2.ts"), source_file2);

        let locations = find_references(&db, "common.hello", None, &source_files, ".", None, None);

        assert_that!(locations.len(), eq(2));
    }

    #[rstest]
    fn test_find_references_no_match() {
        let db = I18nDatabaseImpl::default();

        let source_code = r#"const msg = t("common.hello");"#;
        let source_file = SourceFile::new(
            &db,
            "file:///test.ts".to_string(),
            source_code.to_string(),
            ProgrammingLanguage::TypeScript,
        );

        let mut source_files = HashMap::new();
        source_files.insert(PathBuf::from("/test.ts"), source_file);

        let locations =
            find_references(&db, "common.nonexistent", None, &source_files, ".", None, None);

        assert_that!(locations, is_empty());
    }

    #[rstest]
    fn test_find_references_empty_files() {
        let db = I18nDatabaseImpl::default();

        let source_files = HashMap::new();
        let locations = find_references(&db, "common.hello", None, &source_files, ".", None, None);

        assert_that!(locations, is_empty());
    }

    #[rstest]
    fn test_find_references_plural_suffix() {
        let db = I18nDatabaseImpl::default();

        let source_code = r#"
            const msg1 = t("items");
            const msg2 = t("items_one");
        "#;
        let source_file = SourceFile::new(
            &db,
            "file:///test.ts".to_string(),
            source_code.to_string(),
            ProgrammingLanguage::TypeScript,
        );

        let mut source_files = HashMap::new();
        source_files.insert(PathBuf::from("/test.ts"), source_file);

        let locations = find_references(&db, "items_one", None, &source_files, ".", None, None);

        assert_that!(locations.len(), eq(2));
    }

    #[rstest]
    fn test_find_references_ordinal_plural_suffix() {
        let db = I18nDatabaseImpl::default();

        let source_code = r#"const msg = t("place");"#;
        let source_file = SourceFile::new(
            &db,
            "file:///test.ts".to_string(),
            source_code.to_string(),
            ProgrammingLanguage::TypeScript,
        );

        let mut source_files = HashMap::new();
        source_files.insert(PathBuf::from("/test.ts"), source_file);

        let locations =
            find_references(&db, "place_ordinal_one", None, &source_files, ".", None, None);

        assert_that!(locations.len(), eq(1));
    }

    #[rstest]
    fn test_find_references_base_key_no_extra_matches() {
        let db = I18nDatabaseImpl::default();

        let source_code = r#"
            const msg1 = t("items");
            const msg2 = t("other");
        "#;
        let source_file = SourceFile::new(
            &db,
            "file:///test.ts".to_string(),
            source_code.to_string(),
            ProgrammingLanguage::TypeScript,
        );

        let mut source_files = HashMap::new();
        source_files.insert(PathBuf::from("/test.ts"), source_file);

        let locations = find_references(&db, "items", None, &source_files, ".", None, None);

        assert_that!(locations.len(), eq(1));
    }
}
