//! 診断メッセージ生成モジュール

use std::collections::HashSet;

use tower_lsp::lsp_types::{
    Diagnostic,
    DiagnosticSeverity,
    Position,
    Range,
};

use crate::db::I18nDatabase;
use crate::input::source::SourceFile;
use crate::input::translation::Translation;
use crate::syntax::analyze_source;

/// ソースファイルの診断メッセージを生成
///
/// ソースコード内で使用されている翻訳キーが、
/// 実際の翻訳ファイルに存在するかをチェックし、
/// 存在しない場合は診断メッセージを生成します。
///
/// # Arguments
/// * `db` - Salsa データベース
/// * `source_file` - チェック対象のソースファイル
/// * `translations` - 利用可能な翻訳データのリスト
///
/// # Returns
/// 診断メッセージのリストを返します（存在しないキーに対する警告）
pub fn generate_diagnostics(
    db: &dyn I18nDatabase,
    source_file: SourceFile,
    translations: &[Translation],
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    tracing::debug!("Generating diagnostics for source file '{}'", source_file.uri(db));
    // キー使用箇所を解析
    let key_usages = analyze_source(db, source_file);

    // 全翻訳ファイルから利用可能なキーを収集
    let mut all_keys = HashSet::new();
    for translation in translations {
        all_keys.extend(translation.keys(db).keys().cloned());
    }

    // 各キー使用箇所をチェック
    for usage in key_usages {
        let key = usage.key(db).text(db);

        // 空のキーはスキップ（補完中の状態）
        if key.is_empty() {
            continue;
        }

        // キーが存在しない場合、診断メッセージを追加
        if !all_keys.contains(key) {
            let range = usage.range(db);

            diagnostics.push(Diagnostic {
                range: Range {
                    start: Position { line: range.start.line, character: range.start.character },
                    end: Position { line: range.end.line, character: range.end.character },
                },
                severity: Some(DiagnosticSeverity::WARNING),
                code: None,
                code_description: None,
                source: Some("js-i18n".to_string()),
                message: format!("Translation key '{key}' not found"),
                related_information: None,
                tags: None,
                data: None,
            });
        }
    }

    diagnostics
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use googletest::prelude::*;

    use super::*;
    use crate::db::I18nDatabaseImpl;
    use crate::input::source::{
        ProgrammingLanguage,
        SourceFile,
    };
    use crate::input::translation::Translation;

    #[googletest::test]
    fn test_generate_diagnostics_with_missing_key() {
        let db = I18nDatabaseImpl::default();

        // テスト用のソースコードを作成
        let source_code = r#"
            const msg = t("common.hello");
            const msg2 = t("common.missing");
        "#;
        let source_file = SourceFile::new(
            &db,
            "test.ts".to_string(),
            source_code.to_string(),
            ProgrammingLanguage::TypeScript,
        );

        // テスト用の翻訳データを作成
        let mut keys = HashMap::new();
        keys.insert("common.hello".to_string(), "Hello".to_string());
        keys.insert("common.goodbye".to_string(), "Goodbye".to_string());

        let translation = Translation::new(
            &db,
            "en".to_string(),
            "en.json".to_string(),
            keys,
            String::new(),
            HashMap::new(),
            HashMap::new(),
        );

        // 診断メッセージを生成
        let diagnostics = generate_diagnostics(&db, source_file, &[translation]);

        // "common.missing" キーが存在しないため診断メッセージが生成されることを確認
        expect_that!(diagnostics, not(is_empty()));
        expect_that!(
            diagnostics,
            contains(field!(Diagnostic.message, contains_substring("common.missing")))
        );
        expect_that!(
            diagnostics,
            each(field!(Diagnostic.severity, some(eq(&DiagnosticSeverity::WARNING))))
        );
    }

    #[googletest::test]
    fn test_generate_diagnostics_all_keys_exist() {
        let db = I18nDatabaseImpl::default();

        // テスト用のソースコードを作成（全てのキーが存在）
        let source_code = r#"
            const msg = t("common.hello");
            const msg2 = t("common.goodbye");
        "#;
        let source_file = SourceFile::new(
            &db,
            "test.ts".to_string(),
            source_code.to_string(),
            ProgrammingLanguage::TypeScript,
        );

        // テスト用の翻訳データを作成
        let mut keys = HashMap::new();
        keys.insert("common.hello".to_string(), "Hello".to_string());
        keys.insert("common.goodbye".to_string(), "Goodbye".to_string());

        let translation = Translation::new(
            &db,
            "en".to_string(),
            "en.json".to_string(),
            keys,
            String::new(),
            HashMap::new(),
            HashMap::new(),
        );

        // 診断メッセージを生成
        let diagnostics = generate_diagnostics(&db, source_file, &[translation]);

        // 全てのキーが存在するため、診断メッセージは生成されない
        expect_that!(diagnostics, is_empty());
    }

    #[googletest::test]
    fn test_generate_diagnostics_multiple_translations() {
        let db = I18nDatabaseImpl::default();

        // テスト用のソースコードを作成
        let source_code = r#"
            const msg = t("common.hello");
            const msg2 = t("errors.notFound");
        "#;
        let source_file = SourceFile::new(
            &db,
            "test.ts".to_string(),
            source_code.to_string(),
            ProgrammingLanguage::TypeScript,
        );

        // テスト用の翻訳データを作成（複数言語）
        let mut keys_en = HashMap::new();
        keys_en.insert("common.hello".to_string(), "Hello".to_string());

        let mut keys_ja = HashMap::new();
        keys_ja.insert("errors.notFound".to_string(), "見つかりません".to_string());

        let translation_en = Translation::new(
            &db,
            "en".to_string(),
            "en.json".to_string(),
            keys_en,
            String::new(),
            HashMap::new(),
            HashMap::new(),
        );
        let translation_ja = Translation::new(
            &db,
            "ja".to_string(),
            "ja.json".to_string(),
            keys_ja,
            String::new(),
            HashMap::new(),
            HashMap::new(),
        );

        // 診断メッセージを生成
        let diagnostics = generate_diagnostics(&db, source_file, &[translation_en, translation_ja]);

        // 両方の翻訳ファイルの和集合でチェックされるため、診断メッセージは生成されない
        expect_that!(diagnostics, is_empty());
    }
}
