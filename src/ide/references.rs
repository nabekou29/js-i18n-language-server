//! References implementation

use std::collections::HashMap;
use std::path::PathBuf;

use tower_lsp::lsp_types::Location;

use crate::db::I18nDatabase;
use crate::ide::plural::get_plural_base_key;
use crate::input::source::SourceFile;
use crate::interned::TransKey;
use crate::syntax::analyze_source;

/// Find all references to a translation key across all source files
///
/// # Arguments
/// * `db` - Salsa database
/// * `key` - The translation key to search for
/// * `source_files` - Map of all source files (`PathBuf` -> `SourceFile`)
/// * `key_separator` - キーの区切り文字
///
/// # plural suffix 対応
/// キーが plural suffix を持つ場合（例: `items_one`）、ベースキー（`items`）での
/// 呼び出しも参照として検出します。i18next では `t("items", { count: n })` と
/// 呼び出すと内部で `items_one` などに解決されるためです。
///
/// # Returns
/// List of locations where the key is used
pub fn find_references<S: std::hash::BuildHasher>(
    db: &dyn I18nDatabase,
    key: TransKey<'_>,
    source_files: &HashMap<PathBuf, SourceFile, S>,
    key_separator: &str,
) -> Vec<Location> {
    let key_text = key.text(db);
    let base_key = get_plural_base_key(key_text);
    let mut locations = Vec::new();

    // Iterate through all source files
    for source_file in source_files.values() {
        // Get key usages for this file (cached by Salsa)
        let usages = analyze_source(db, *source_file, key_separator.to_string());

        // Filter usages that match the target key
        for usage in usages {
            let usage_key = usage.key(db);
            let usage_key_text = usage_key.text(db);

            // 完全一致、または plural のベースキーが一致
            let is_match =
                usage_key_text == key_text || base_key.is_some_and(|bk| usage_key_text == bk);

            if is_match {
                // Convert to LSP Location
                let range = usage.range(db);
                let uri = source_file.uri(db);

                // URI のパースに失敗した場合はスキップ
                let Ok(parsed_uri) = uri.parse() else {
                    tracing::warn!("Failed to parse URI: {}", uri);
                    continue;
                };

                locations.push(Location { uri: parsed_uri, range: range.into() });
            }
        }
    }

    locations
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;

    use googletest::prelude::*;

    use super::*;
    use crate::db::I18nDatabaseImpl;
    use crate::input::source::{
        ProgrammingLanguage,
        SourceFile,
    };
    use crate::interned::TransKey;

    #[googletest::test]
    fn test_find_references_single_file() {
        let db = I18nDatabaseImpl::default();

        // テスト用のソースコードを作成（同じキーを複数回使用）
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

        // "common.hello" キーを作成
        let key = TransKey::new(&db, "common.hello".to_string());

        // 参照を検索
        let locations = find_references(&db, key, &source_files, ".");

        // "common.hello" は2回使用されている
        expect_that!(locations.len(), eq(2));

        // すべての Location が同じファイルを指している
        for location in &locations {
            expect_that!(location.uri.path(), eq("/test.ts"));
        }
    }

    #[googletest::test]
    fn test_find_references_multiple_files() {
        let db = I18nDatabaseImpl::default();

        // ファイル1
        let source_code1 = r#"const msg = t("common.hello");"#;
        let source_file1 = SourceFile::new(
            &db,
            "file:///test1.ts".to_string(),
            source_code1.to_string(),
            ProgrammingLanguage::TypeScript,
        );

        // ファイル2
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

        // "common.hello" キーを作成
        let key = TransKey::new(&db, "common.hello".to_string());

        // 参照を検索
        let locations = find_references(&db, key, &source_files, ".");

        // 両方のファイルで使用されている
        expect_that!(locations.len(), eq(2));
    }

    #[googletest::test]
    fn test_find_references_no_match() {
        let db = I18nDatabaseImpl::default();

        // テスト用のソースコードを作成
        let source_code = r#"const msg = t("common.hello");"#;
        let source_file = SourceFile::new(
            &db,
            "file:///test.ts".to_string(),
            source_code.to_string(),
            ProgrammingLanguage::TypeScript,
        );

        let mut source_files = HashMap::new();
        source_files.insert(PathBuf::from("/test.ts"), source_file);

        // 存在しないキーを検索
        let key = TransKey::new(&db, "common.nonexistent".to_string());

        // 参照を検索
        let locations = find_references(&db, key, &source_files, ".");

        // 一致なし
        expect_that!(locations, is_empty());
    }

    #[googletest::test]
    fn test_find_references_empty_files() {
        let db = I18nDatabaseImpl::default();

        // 空のソースファイル
        let source_files = HashMap::new();

        // キーを作成
        let key = TransKey::new(&db, "common.hello".to_string());

        // 参照を検索
        let locations = find_references(&db, key, &source_files, ".");

        // 一致なし
        expect_that!(locations, is_empty());
    }

    #[googletest::test]
    fn test_find_references_plural_suffix() {
        let db = I18nDatabaseImpl::default();

        // ソースコードでは t("items") と呼び出し
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

        // "items_one" キーで参照を検索
        let key = TransKey::new(&db, "items_one".to_string());
        let locations = find_references(&db, key, &source_files, ".");

        // t("items") と t("items_one") の両方がヒットする
        expect_that!(locations.len(), eq(2));
    }

    #[googletest::test]
    fn test_find_references_ordinal_plural_suffix() {
        let db = I18nDatabaseImpl::default();

        // ソースコードでは t("place") と呼び出し
        let source_code = r#"const msg = t("place");"#;
        let source_file = SourceFile::new(
            &db,
            "file:///test.ts".to_string(),
            source_code.to_string(),
            ProgrammingLanguage::TypeScript,
        );

        let mut source_files = HashMap::new();
        source_files.insert(PathBuf::from("/test.ts"), source_file);

        // "place_ordinal_one" キーで参照を検索
        let key = TransKey::new(&db, "place_ordinal_one".to_string());
        let locations = find_references(&db, key, &source_files, ".");

        // t("place") がヒットする
        expect_that!(locations.len(), eq(1));
    }

    #[googletest::test]
    fn test_find_references_base_key_no_extra_matches() {
        let db = I18nDatabaseImpl::default();

        // ソースコードでは t("items") と t("other") を呼び出し
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

        // "items" キーで参照を検索（plural suffix なし）
        let key = TransKey::new(&db, "items".to_string());
        let locations = find_references(&db, key, &source_files, ".");

        // t("items") のみがヒット（t("other") はヒットしない）
        expect_that!(locations.len(), eq(1));
    }
}
