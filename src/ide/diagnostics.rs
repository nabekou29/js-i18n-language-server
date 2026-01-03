//! 診断メッセージ生成モジュール

use std::collections::HashSet;

use tower_lsp::lsp_types::{
    Diagnostic,
    DiagnosticSeverity,
    DiagnosticTag,
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
/// # 逆方向 prefix マッチ
/// `t('nested')` で `nested.key` が存在する場合、`nested` は有効なキーとみなします。
/// これにより、オブジェクト全体を取得するパターンに対応します。
///
/// # Arguments
/// * `db` - Salsa データベース
/// * `source_file` - チェック対象のソースファイル
/// * `translations` - 利用可能な翻訳データのリスト
/// * `options` - 診断生成オプション
/// * `key_separator` - キーの区切り文字（例: "."）
///
/// # Returns
/// 診断メッセージのリストを返します（存在しないキーに対する警告）
pub fn generate_diagnostics(
    db: &dyn I18nDatabase,
    source_file: SourceFile,
    translations: &[Translation],
    options: &DiagnosticOptions,
    key_separator: &str,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    tracing::debug!("Generating diagnostics for source file '{}'", source_file.uri(db));

    // キー使用箇所を解析
    let key_usages = analyze_source(db, source_file, key_separator.to_string());

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
        // 逆方向 prefix マッチ: nested.key がある場合、nested も有効とみなす
        let missing_languages: Vec<String> = language_keys
            .iter()
            .filter(|(lang, _)| target_languages.contains(lang.as_str()))
            .filter(|(_, keys)| !key_exists_or_has_children(key, keys, key_separator))
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

/// 翻訳ファイルの未使用キー診断を生成
///
/// ソースコードで使用されていない翻訳キーに対して診断メッセージを生成します。
///
/// # ネストしたキーの扱い
/// `hoge.fuga.piyo` というキーがある場合、`hoge.fuga` が使用されていれば
/// `hoge.fuga.piyo` も使用されているとみなす（prefix マッチ）。
/// これは `t('hoge.fuga')` でオブジェクトをまとめて取得するケースに対応。
///
/// # Arguments
/// * `db` - Salsa データベース
/// * `translation` - 診断対象の翻訳ファイル
/// * `source_files` - すべてのソースファイル
/// * `key_separator` - キーの区切り文字（例: "."）
///
/// # Returns
/// 未使用キーに対する診断メッセージのリスト
pub fn generate_unused_key_diagnostics(
    db: &dyn I18nDatabase,
    translation: Translation,
    source_files: &[SourceFile],
    key_separator: &str,
) -> Vec<Diagnostic> {
    // 1. すべてのソースファイルから使用されているキーを集計
    let mut used_keys: HashSet<String> = HashSet::new();
    for source_file in source_files {
        let key_usages = analyze_source(db, *source_file, key_separator.to_string());
        for usage in key_usages {
            used_keys.insert(usage.key(db).text(db).clone());
        }
    }

    // 2. 翻訳ファイル内の全キーと比較
    let all_keys = translation.keys(db);
    let key_ranges = translation.key_ranges(db);

    // 3. 未使用キーに対して Diagnostic を生成
    let mut diagnostics = Vec::new();
    for key in all_keys.keys() {
        // キーが使用されているかチェック（prefix マッチを含む）
        let is_used = is_key_used(key, &used_keys, key_separator);

        if !is_used && let Some(range) = key_ranges.get(key) {
            diagnostics.push(Diagnostic {
                range: Range {
                    start: Position { line: range.start.line, character: range.start.character },
                    end: Position { line: range.end.line, character: range.end.character },
                },
                severity: Some(DiagnosticSeverity::HINT),
                code: Some(NumberOrString::String("unused-translation-key".to_string())),
                code_description: None,
                source: Some("js-i18n".to_string()),
                message: format!("Translation key '{key}' is not used in any source files"),
                related_information: None,
                tags: Some(vec![DiagnosticTag::UNNECESSARY]),
                data: None,
            });
        }
    }

    diagnostics
}

/// キーが使用されているかチェック（prefix マッチを含む）
///
/// # Examples
/// - `key = "hoge.fuga.piyo"`, `used_keys = {"hoge.fuga"}`
///   → `"hoge.fuga"` が `"hoge.fuga.piyo"` の prefix なので `true` を返す
/// - `key = "hoge.fuga"`, `used_keys = {"hoge.fuga"}`
///   → 完全一致なので `true` を返す
/// - `key = "other.key"`, `used_keys = {"hoge.fuga"}`
///   → マッチしないので `false` を返す
fn is_key_used(key: &str, used_keys: &HashSet<String>, separator: &str) -> bool {
    // 完全一致
    if used_keys.contains(key) {
        return true;
    }

    // prefix マッチ: used_keys のいずれかが key の prefix であるかチェック
    // 例: key = "hoge.fuga.piyo", used_key = "hoge.fuga"
    //     → "hoge.fuga.piyo".starts_with("hoge.fuga.")
    for used_key in used_keys {
        let prefix = format!("{used_key}{separator}");
        if key.starts_with(&prefix) {
            return true;
        }
    }

    false
}

/// 指定したキーが prefix となるキーが存在するかチェック（逆方向 prefix マッチ）
///
/// # Examples
/// - `search_key = "nested"`, `available_keys = {"nested.key"}` → `true`
/// - `search_key = "nested"`, `available_keys = {"other.key"}` → `false`
fn has_keys_with_prefix(
    search_key: &str,
    available_keys: &HashSet<String>,
    separator: &str,
) -> bool {
    let prefix = format!("{search_key}{separator}");
    available_keys.iter().any(|key| key.starts_with(&prefix))
}

/// キーが存在するか、または prefix として子キーが存在するかをチェック
///
/// 完全一致を優先し、一致しない場合は逆方向 prefix マッチを行う。
/// これにより `t('nested')` で `nested.key` がある場合も有効なキーとみなす。
fn key_exists_or_has_children(
    key: &str,
    available_keys: &HashSet<String>,
    separator: &str,
) -> bool {
    // 完全一致
    if available_keys.contains(key) {
        return true;
    }
    // 逆方向 prefix マッチ
    has_keys_with_prefix(key, available_keys, separator)
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
        let diagnostics = generate_diagnostics(&db, source_file, &[translation], &options, ".");

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
        let diagnostics = generate_diagnostics(&db, source_file, &[translation], &options, ".");

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
        let diagnostics = generate_diagnostics(
            &db,
            source_file,
            &[translation_en, translation_ja],
            &options,
            ".",
        );

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

    #[googletest::test]
    fn test_is_key_used_exact_match() {
        let used_keys: HashSet<String> =
            ["common.hello", "common.goodbye"].iter().map(|s| s.to_string()).collect();

        expect_that!(is_key_used("common.hello", &used_keys, "."), eq(true));
        expect_that!(is_key_used("common.goodbye", &used_keys, "."), eq(true));
        expect_that!(is_key_used("common.missing", &used_keys, "."), eq(false));
    }

    #[googletest::test]
    fn test_is_key_used_prefix_match() {
        // t('hoge.fuga') が使われている場合、hoge.fuga.piyo は使用済みとみなす
        let used_keys: HashSet<String> = ["hoge.fuga"].iter().map(|s| s.to_string()).collect();

        expect_that!(is_key_used("hoge.fuga", &used_keys, "."), eq(true));
        expect_that!(is_key_used("hoge.fuga.piyo", &used_keys, "."), eq(true));
        expect_that!(is_key_used("hoge.fuga.piyo.deep", &used_keys, "."), eq(true));
        // hoge.fugaX は prefix マッチしない（hoge.fuga. で始まらない）
        expect_that!(is_key_used("hoge.fugaX", &used_keys, "."), eq(false));
        expect_that!(is_key_used("other.key", &used_keys, "."), eq(false));
    }

    #[googletest::test]
    fn test_is_key_used_with_custom_separator() {
        let used_keys: HashSet<String> = ["hoge:fuga"].iter().map(|s| s.to_string()).collect();

        expect_that!(is_key_used("hoge:fuga", &used_keys, ":"), eq(true));
        expect_that!(is_key_used("hoge:fuga:piyo", &used_keys, ":"), eq(true));
        // ドットは区切り文字ではないので prefix マッチしない
        expect_that!(is_key_used("hoge:fuga.piyo", &used_keys, ":"), eq(false));
    }

    #[googletest::test]
    fn test_generate_unused_key_diagnostics_basic() {
        let db = I18nDatabaseImpl::default();

        // ソースコード: common.hello のみ使用
        let source_code = r#"const msg = t("common.hello");"#;
        let source_file = SourceFile::new(
            &db,
            "test.ts".to_string(),
            source_code.to_string(),
            ProgrammingLanguage::TypeScript,
        );

        // 翻訳ファイル: common.hello と common.unused を含む
        let mut keys = HashMap::new();
        keys.insert("common.hello".to_string(), "Hello".to_string());
        keys.insert("common.unused".to_string(), "Unused".to_string());

        let mut key_ranges = HashMap::new();
        key_ranges.insert(
            "common.hello".to_string(),
            crate::types::SourceRange {
                start: crate::types::SourcePosition { line: 1, character: 2 },
                end: crate::types::SourcePosition { line: 1, character: 16 },
            },
        );
        key_ranges.insert(
            "common.unused".to_string(),
            crate::types::SourceRange {
                start: crate::types::SourcePosition { line: 2, character: 2 },
                end: crate::types::SourcePosition { line: 2, character: 17 },
            },
        );

        let translation = Translation::new(
            &db,
            "en".to_string(),
            "en.json".to_string(),
            keys,
            String::new(),
            key_ranges,
            HashMap::new(),
        );

        let diagnostics = generate_unused_key_diagnostics(&db, translation, &[source_file], ".");

        // common.unused のみが未使用として検出される
        expect_that!(diagnostics, len(eq(1)));
        expect_that!(
            diagnostics,
            contains(all![
                field!(Diagnostic.message, contains_substring("common.unused")),
                field!(Diagnostic.severity, some(eq(&DiagnosticSeverity::HINT))),
                field!(
                    Diagnostic.code,
                    some(eq(&NumberOrString::String("unused-translation-key".to_string())))
                ),
                field!(Diagnostic.tags, some(contains(eq(&DiagnosticTag::UNNECESSARY))))
            ])
        );
    }

    #[googletest::test]
    fn test_generate_unused_key_diagnostics_with_prefix_match() {
        let db = I18nDatabaseImpl::default();

        // ソースコード: hoge.fuga を使用（オブジェクト全体を取得するパターン）
        let source_code = r#"const obj = t("hoge.fuga");"#;
        let source_file = SourceFile::new(
            &db,
            "test.ts".to_string(),
            source_code.to_string(),
            ProgrammingLanguage::TypeScript,
        );

        // 翻訳ファイル: ネストしたキーを含む
        let mut keys = HashMap::new();
        keys.insert("hoge.fuga".to_string(), "Parent".to_string());
        keys.insert("hoge.fuga.piyo".to_string(), "Child".to_string());
        keys.insert("other.key".to_string(), "Other".to_string());

        let mut key_ranges = HashMap::new();
        key_ranges.insert(
            "hoge.fuga".to_string(),
            crate::types::SourceRange {
                start: crate::types::SourcePosition { line: 1, character: 2 },
                end: crate::types::SourcePosition { line: 1, character: 12 },
            },
        );
        key_ranges.insert(
            "hoge.fuga.piyo".to_string(),
            crate::types::SourceRange {
                start: crate::types::SourcePosition { line: 2, character: 2 },
                end: crate::types::SourcePosition { line: 2, character: 18 },
            },
        );
        key_ranges.insert(
            "other.key".to_string(),
            crate::types::SourceRange {
                start: crate::types::SourcePosition { line: 3, character: 2 },
                end: crate::types::SourcePosition { line: 3, character: 13 },
            },
        );

        let translation = Translation::new(
            &db,
            "en".to_string(),
            "en.json".to_string(),
            keys,
            String::new(),
            key_ranges,
            HashMap::new(),
        );

        let diagnostics = generate_unused_key_diagnostics(&db, translation, &[source_file], ".");

        // hoge.fuga と hoge.fuga.piyo は使用済み（prefix マッチ）
        // other.key のみが未使用として検出される
        expect_that!(diagnostics, len(eq(1)));
        expect_that!(
            diagnostics,
            contains(field!(Diagnostic.message, contains_substring("other.key")))
        );
    }

    #[googletest::test]
    fn test_has_keys_with_prefix() {
        let keys: HashSet<String> =
            ["nested.key", "nested.foo", "other.key"].iter().map(|s| s.to_string()).collect();

        // "nested" は "nested.key" や "nested.foo" の prefix
        expect_that!(has_keys_with_prefix("nested", &keys, "."), eq(true));
        // "other" は "other.key" の prefix
        expect_that!(has_keys_with_prefix("other", &keys, "."), eq(true));
        // "missing" には子キーがない
        expect_that!(has_keys_with_prefix("missing", &keys, "."), eq(false));
        // "nest" は "nested.key" の prefix ではない（ドットが必要）
        expect_that!(has_keys_with_prefix("nest", &keys, "."), eq(false));
    }

    #[googletest::test]
    fn test_key_exists_or_has_children() {
        let keys: HashSet<String> =
            ["nested.key", "nested.foo", "single"].iter().map(|s| s.to_string()).collect();

        // 完全一致
        expect_that!(key_exists_or_has_children("single", &keys, "."), eq(true));
        expect_that!(key_exists_or_has_children("nested.key", &keys, "."), eq(true));

        // 逆方向 prefix マッチ（子キーが存在する）
        expect_that!(key_exists_or_has_children("nested", &keys, "."), eq(true));

        // 存在しないキー（完全一致も prefix マッチもしない）
        expect_that!(key_exists_or_has_children("missing", &keys, "."), eq(false));
        // "singl" は "single" の prefix ではない（ドットが必要）
        expect_that!(key_exists_or_has_children("singl", &keys, "."), eq(false));
    }

    #[googletest::test]
    fn test_generate_diagnostics_with_reverse_prefix_match() {
        let db = I18nDatabaseImpl::default();

        // t('nested') を使用（nested キーは存在せず、nested.key のみ存在）
        let source_code = r#"const msg = t("nested");"#;
        let source_file = SourceFile::new(
            &db,
            "test.ts".to_string(),
            source_code.to_string(),
            ProgrammingLanguage::TypeScript,
        );

        // 翻訳ファイル: nested.key と nested.foo のみ存在（nested キーは存在しない）
        let mut keys = HashMap::new();
        keys.insert("nested.key".to_string(), "Key Value".to_string());
        keys.insert("nested.foo".to_string(), "Foo Value".to_string());

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
        let diagnostics = generate_diagnostics(&db, source_file, &[translation], &options, ".");

        // nested.key が存在するため、nested は有効なキーとみなされ、診断は生成されない
        expect_that!(diagnostics, is_empty());
    }

    #[googletest::test]
    fn test_generate_diagnostics_no_reverse_prefix_match() {
        let db = I18nDatabaseImpl::default();

        // t('missing') を使用（missing キーも missing.* も存在しない）
        let source_code = r#"const msg = t("missing");"#;
        let source_file = SourceFile::new(
            &db,
            "test.ts".to_string(),
            source_code.to_string(),
            ProgrammingLanguage::TypeScript,
        );

        // 翻訳ファイル: 関係のないキーのみ
        let mut keys = HashMap::new();
        keys.insert("other.key".to_string(), "Other Value".to_string());

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
        let diagnostics = generate_diagnostics(&db, source_file, &[translation], &options, ".");

        // missing キーも missing.* も存在しないため、診断が生成される
        expect_that!(diagnostics, len(eq(1)));
        expect_that!(
            diagnostics,
            contains(field!(Diagnostic.message, contains_substring("missing")))
        );
    }
}
