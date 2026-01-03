//! References implementation

use std::collections::HashMap;
use std::path::PathBuf;

use tower_lsp::lsp_types::Location;

use crate::db::I18nDatabase;
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
/// # Returns
/// List of locations where the key is used
pub fn find_references<S: std::hash::BuildHasher>(
    db: &dyn I18nDatabase,
    key: TransKey<'_>,
    source_files: &HashMap<PathBuf, SourceFile, S>,
    key_separator: &str,
) -> Vec<Location> {
    let key_text = key.text(db);
    let mut locations = Vec::new();

    // Iterate through all source files
    for source_file in source_files.values() {
        // Get key usages for this file (cached by Salsa)
        let usages = analyze_source(db, *source_file, key_separator.to_string());

        // Filter usages that match the target key
        for usage in usages {
            let usage_key = usage.key(db);
            if usage_key.text(db) == key_text {
                // Convert to LSP Location
                let range = usage.range(db);
                let uri = source_file.uri(db);

                // URI のパースに失敗した場合はスキップ
                let Ok(parsed_uri) = uri.parse() else {
                    tracing::warn!("Failed to parse URI: {}", uri);
                    continue;
                };

                locations.push(Location {
                    uri: parsed_uri,
                    range: tower_lsp::lsp_types::Range {
                        start: tower_lsp::lsp_types::Position {
                            line: range.start.line,
                            character: range.start.character,
                        },
                        end: tower_lsp::lsp_types::Position {
                            line: range.end.line,
                            character: range.end.character,
                        },
                    },
                });
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
}
