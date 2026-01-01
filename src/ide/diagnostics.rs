//! 診断メッセージ生成モジュール

use std::collections::HashSet;

use tower_lsp::lsp_types::{
    Diagnostic,
    DiagnosticSeverity,
    NumberOrString,
    Position,
    Range,
};

use crate::db::I18nDatabase;
use crate::input::source::SourceFile;
use crate::input::translation::Translation;
use crate::syntax::analyze_source;

/// 診断生成のオプション
#[derive(Debug, Clone, Default)]
pub struct DiagnosticOptions {
    /// 翻訳が必須の言語（None の場合はすべての言語が必須）
    pub required_languages: Option<HashSet<String>>,
    /// 翻訳が任意の言語（これらの言語は診断対象外）
    pub optional_languages: Option<HashSet<String>>,
}

/// チェック対象の言語を決定する
///
/// # Arguments
/// * `all_languages` - 翻訳ファイルから検出されたすべての言語
/// * `options` - 診断生成オプション
///
/// # Returns
/// チェック対象の言語セット
#[must_use]
#[allow(clippy::implicit_hasher, clippy::option_if_let_else)]
pub fn determine_target_languages<'a>(
    all_languages: &'a HashSet<String>,
    options: &DiagnosticOptions,
) -> HashSet<&'a str> {
    if let Some(ref required) = options.required_languages {
        // required_languages が指定されている場合、それらのみチェック
        all_languages.iter().filter(|lang| required.contains(*lang)).map(String::as_str).collect()
    } else if let Some(ref optional) = options.optional_languages {
        // optional_languages が指定されている場合、それら以外をチェック
        all_languages.iter().filter(|lang| !optional.contains(*lang)).map(String::as_str).collect()
    } else {
        // どちらも指定されていない場合、すべての言語をチェック
        all_languages.iter().map(String::as_str).collect()
    }
}

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
/// * `options` - 診断生成オプション
///
/// # Returns
/// 診断メッセージのリストを返します（存在しないキーに対する警告）
pub fn generate_diagnostics(
    db: &dyn I18nDatabase,
    source_file: SourceFile,
    translations: &[Translation],
    options: &DiagnosticOptions,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    tracing::debug!("Generating diagnostics for source file '{}'", source_file.uri(db));

    // キー使用箇所を解析
    let key_usages = analyze_source(db, source_file);

    // 各言語の翻訳キーセットを構築
    let language_keys: Vec<(String, HashSet<String>)> = translations
        .iter()
        .map(|t| {
            let lang = t.language(db);
            let keys: HashSet<String> = t.keys(db).keys().cloned().collect();
            (lang, keys)
        })
        .collect();

    // チェック対象の言語を決定
    let all_languages: HashSet<String> = translations.iter().map(|t| t.language(db)).collect();
    let target_languages = determine_target_languages(&all_languages, options);

    // 各キー使用箇所をチェック
    for usage in key_usages {
        let key = usage.key(db).text(db);

        // 空のキーはスキップ（補完中の状態）
        if key.is_empty() {
            continue;
        }

        // 各言語でキーが存在するかチェックし、不足している言語を収集
        let missing_languages: Vec<String> = language_keys
            .iter()
            .filter(|(lang, _)| target_languages.contains(lang.as_str()))
            .filter(|(_, keys)| !keys.contains(key))
            .map(|(lang, _)| lang.clone())
            .collect();

        // 不足している言語がある場合、診断メッセージを追加
        if !missing_languages.is_empty() {
            let range = usage.range(db);

            let message =
                format!("Translation key '{}' missing for: {}", key, missing_languages.join(", "));

            diagnostics.push(Diagnostic {
                range: Range {
                    start: Position { line: range.start.line, character: range.start.character },
                    end: Position { line: range.end.line, character: range.end.character },
                },
                severity: Some(DiagnosticSeverity::WARNING),
                code: Some(NumberOrString::String("missing-translation".to_string())),
                code_description: None,
                source: Some("js-i18n".to_string()),
                message,
                related_information: None,
                tags: None,
                data: Some(serde_json::json!({
                    "key": key,
                    "missing_languages": missing_languages
                })),
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
        let options = DiagnosticOptions::default();
        let diagnostics = generate_diagnostics(&db, source_file, &[translation], &options);

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
        // code と data が正しく設定されていることを確認
        expect_that!(
            diagnostics,
            each(field!(
                Diagnostic.code,
                some(eq(&NumberOrString::String("missing-translation".to_string())))
            ))
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
        let options = DiagnosticOptions::default();
        let diagnostics = generate_diagnostics(&db, source_file, &[translation], &options);

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
        let options = DiagnosticOptions::default();
        let diagnostics =
            generate_diagnostics(&db, source_file, &[translation_en, translation_ja], &options);

        // 各言語で不足しているキーがあるため、診断メッセージが生成される
        // - common.hello は ja で不足
        // - errors.notFound は en で不足
        expect_that!(diagnostics, len(eq(2)));
        expect_that!(
            diagnostics,
            contains(all![
                field!(Diagnostic.message, contains_substring("common.hello")),
                field!(Diagnostic.message, contains_substring("ja"))
            ])
        );
        expect_that!(
            diagnostics,
            contains(all![
                field!(Diagnostic.message, contains_substring("errors.notFound")),
                field!(Diagnostic.message, contains_substring("en"))
            ])
        );
    }

    #[googletest::test]
    fn test_determine_target_languages_with_required() {
        let all_languages: HashSet<String> =
            ["en", "ja", "zh"].iter().map(|s| s.to_string()).collect();
        let options = DiagnosticOptions {
            required_languages: Some(["en", "ja"].iter().map(|s| s.to_string()).collect()),
            optional_languages: None,
        };

        let target = determine_target_languages(&all_languages, &options);

        expect_that!(target, len(eq(2)));
        expect_that!(target, contains(eq(&"en")));
        expect_that!(target, contains(eq(&"ja")));
        expect_that!(target, not(contains(eq(&"zh"))));
    }

    #[googletest::test]
    fn test_determine_target_languages_with_optional() {
        let all_languages: HashSet<String> =
            ["en", "ja", "zh"].iter().map(|s| s.to_string()).collect();
        let options = DiagnosticOptions {
            required_languages: None,
            optional_languages: Some(["zh"].iter().map(|s| s.to_string()).collect()),
        };

        let target = determine_target_languages(&all_languages, &options);

        expect_that!(target, len(eq(2)));
        expect_that!(target, contains(eq(&"en")));
        expect_that!(target, contains(eq(&"ja")));
        expect_that!(target, not(contains(eq(&"zh"))));
    }
}
