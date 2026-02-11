//! Diagnostic generation module

use std::collections::HashSet;

use tower_lsp::lsp_types::{
    Diagnostic,
    DiagnosticTag,
    NumberOrString,
};

use crate::config::Severity;
use crate::db::I18nDatabase;
use crate::ide::key_match::is_child_key;
use crate::ide::namespace::{
    filter_translations_by_namespace,
    resolve_namespace,
};
use crate::ide::plural::{
    get_plural_base_key,
    has_plural_variants,
};
use crate::input::source::SourceFile;
use crate::input::translation::Translation;
use crate::syntax::analyze_source;
use crate::syntax::analyzer::extractor::parse_key_with_namespace;

#[derive(Debug, Clone)]
pub struct DiagnosticOptions {
    pub enabled: bool,
    pub severity: Severity,
    /// Languages that require translations (None = all languages required)
    pub required_languages: Option<HashSet<String>>,
    /// Languages excluded from diagnostics
    pub optional_languages: Option<HashSet<String>>,
}

impl Default for DiagnosticOptions {
    fn default() -> Self {
        Self {
            enabled: true,
            severity: Severity::Warning,
            required_languages: None,
            optional_languages: None,
        }
    }
}

/// Determines which languages to check for missing translations.
///
/// Priority: `required_languages` > `optional_languages` > all languages
#[must_use]
#[allow(clippy::implicit_hasher, clippy::option_if_let_else)]
pub fn determine_target_languages<'a>(
    all_languages: &'a HashSet<String>,
    options: &DiagnosticOptions,
) -> HashSet<&'a str> {
    if let Some(ref required) = options.required_languages {
        all_languages.iter().filter(|lang| required.contains(*lang)).map(String::as_str).collect()
    } else if let Some(ref optional) = options.optional_languages {
        all_languages.iter().filter(|lang| !optional.contains(*lang)).map(String::as_str).collect()
    } else {
        all_languages.iter().map(String::as_str).collect()
    }
}

/// Generates diagnostics for missing translation keys in source file.
///
/// Supports reverse prefix matching: `t('nested')` is valid if `nested.key` exists,
/// allowing object retrieval patterns.
/// Filters translations by namespace when `namespace_separator` is set.
pub fn generate_diagnostics(
    db: &dyn I18nDatabase,
    source_file: SourceFile,
    translations: &[Translation],
    options: &DiagnosticOptions,
    key_separator: &str,
    namespace_separator: Option<&str>,
    default_namespace: Option<&str>,
) -> Vec<Diagnostic> {
    if !options.enabled {
        return Vec::new();
    }

    let mut diagnostics = Vec::new();

    tracing::debug!("Generating diagnostics for source file '{}'", source_file.uri(db));

    let key_usages = analyze_source(db, source_file, key_separator.to_string());

    for usage in key_usages {
        let full_key = usage.key(db).text(db);

        // Empty key = completion in progress
        if full_key.is_empty() {
            continue;
        }

        let (explicit_ns, key_part) = parse_key_with_namespace(full_key, namespace_separator);
        let declared_ns = usage.namespace(db);
        let declared_nss = usage.namespaces(db);

        let filtered = filter_translations_by_namespace(
            db,
            translations,
            explicit_ns.as_deref(),
            declared_ns.as_deref(),
            declared_nss.as_deref(),
            default_namespace,
        );

        let language_keys: Vec<(String, HashSet<String>)> = filtered
            .iter()
            .map(|t| {
                let lang = t.language(db);
                let keys: HashSet<String> = t.keys(db).keys().cloned().collect();
                (lang, keys)
            })
            .collect();

        let all_languages: HashSet<String> = filtered.iter().map(|t| t.language(db)).collect();
        let target_languages = determine_target_languages(&all_languages, options);

        let missing_languages: Vec<String> = language_keys
            .iter()
            .filter(|(lang, _)| target_languages.contains(lang.as_str()))
            .filter(|(_, keys)| !key_exists_or_has_children(&key_part, keys, key_separator))
            .map(|(lang, _)| lang.clone())
            .collect();

        if !missing_languages.is_empty() {
            let range = usage.range(db);

            let message = format!(
                "Translation key '{}' missing for: {}",
                full_key,
                missing_languages.join(", ")
            );

            diagnostics.push(Diagnostic {
                range: range.into(),
                severity: Some(options.severity.to_lsp()),
                code: Some(NumberOrString::String("missing-translation".to_string())),
                code_description: None,
                source: Some("js-i18n".to_string()),
                message,
                related_information: None,
                tags: None,
                data: Some(serde_json::json!({
                    "key": full_key,
                    "missing_languages": missing_languages
                })),
            });
        }
    }

    diagnostics
}

/// Generates diagnostics for unused translation keys.
///
/// Supports prefix matching: if `hoge.fuga` is used, `hoge.fuga.piyo` is considered used
/// (for object retrieval patterns like `t('hoge.fuga')`).
/// Filters by namespace when `namespace_separator` is set.
#[allow(clippy::too_many_arguments)]
pub fn generate_unused_key_diagnostics(
    db: &dyn I18nDatabase,
    translation: Translation,
    source_files: &[SourceFile],
    key_separator: &str,
    ignore_patterns: &[String],
    severity: Severity,
    namespace_separator: Option<&str>,
    default_namespace: Option<&str>,
) -> Vec<Diagnostic> {
    let translation_ns = translation.namespace(db);
    let mut used_keys: HashSet<String> = HashSet::new();
    for source_file in source_files {
        let key_usages = analyze_source(db, *source_file, key_separator.to_string());
        for usage in key_usages {
            let full_key = usage.key(db).text(db);
            let (explicit_ns, key_part) = parse_key_with_namespace(full_key, namespace_separator);
            let declared_ns = usage.namespace(db);
            let declared_nss = usage.namespaces(db);

            let resolved_ns = resolve_namespace(
                explicit_ns.as_deref(),
                declared_ns.as_deref(),
                declared_nss.as_deref(),
                default_namespace,
            );

            let ns_matches = match (&resolved_ns, &translation_ns) {
                (None, _) | (_, None) => true,
                (Some(rns), Some(tns)) => rns == tns,
            };

            if ns_matches {
                used_keys.insert(key_part);
            }
        }
    }

    let ignore_matcher = build_ignore_matcher(ignore_patterns);

    let all_keys = translation.keys(db);
    let key_ranges = translation.key_ranges(db);

    let mut diagnostics = Vec::new();
    for key in all_keys.keys() {
        if ignore_matcher.as_ref().is_some_and(|m| m.is_match(key)) {
            continue;
        }

        let is_used = is_key_used(key, &used_keys, key_separator);

        if !is_used && let Some(range) = key_ranges.get(key) {
            diagnostics.push(Diagnostic {
                range: (*range).into(),
                severity: Some(severity.to_lsp()),
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

fn build_ignore_matcher(patterns: &[String]) -> Option<globset::GlobSet> {
    if patterns.is_empty() {
        return None;
    }
    let mut builder = globset::GlobSetBuilder::new();
    for pattern in patterns {
        match globset::Glob::new(pattern) {
            Ok(glob) => {
                builder.add(glob);
            }
            Err(e) => {
                tracing::warn!("Invalid ignore pattern '{pattern}': {e}");
            }
        }
    }
    builder.build().ok()
}

/// Checks if a key is used (including prefix match and plural suffix).
///
/// # Match order
/// 1. Exact match
/// 2. Prefix match: an element in `used_keys` is a prefix of `key` (supports array notation)
/// 3. Plural suffix match: `key` has a plural suffix and its base key is used
///
/// # Examples
/// - `key = "hoge.fuga.piyo"`, `used_keys = {"hoge.fuga"}` -> true (prefix match)
/// - `key = "items[0]"`, `used_keys = {"items"}` -> true (array prefix match)
/// - `key = "items_one"`, `used_keys = {"items"}` -> true (plural base key used)
pub(crate) fn is_key_used(key: &str, used_keys: &HashSet<String>, separator: &str) -> bool {
    if used_keys.contains(key) {
        return true;
    }

    let is_prefix_match = used_keys.iter().any(|used_key| is_child_key(key, used_key, separator));
    if is_prefix_match {
        return true;
    }

    get_plural_base_key(key).is_some_and(|base_key| used_keys.contains(base_key))
}

/// Checks if any key starts with the given prefix (reverse prefix match).
///
/// # Examples
/// - `search_key = "nested"`, `available_keys = {"nested.key"}` -> true
/// - `search_key = "items"`, `available_keys = {"items[0]"}` -> true (array notation)
/// - `search_key = "nested"`, `available_keys = {"other.key"}` -> false
fn has_keys_with_prefix(
    search_key: &str,
    available_keys: &HashSet<String>,
    separator: &str,
) -> bool {
    available_keys.iter().any(|key| is_child_key(key, search_key, separator))
}

/// Checks if a key exists, has child keys, or has plural variants.
///
/// # Match order
/// 1. Exact match
/// 2. Plural variants exist (e.g., `items_one`, `items_other`)
/// 3. Reverse prefix match (e.g., `nested.key` exists for `nested`)
///
/// This validates:
/// - `t('items')` when `items_one`, `items_other` exist
/// - `t('nested')` when `nested.key` exists
fn key_exists_or_has_children(
    key: &str,
    available_keys: &HashSet<String>,
    separator: &str,
) -> bool {
    if available_keys.contains(key) {
        return true;
    }
    if has_plural_variants(key, available_keys) {
        return true;
    }
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
    use rstest::*;
    use tower_lsp::lsp_types::DiagnosticSeverity;

    use super::*;
    use crate::db::I18nDatabaseImpl;
    use crate::input::source::{
        ProgrammingLanguage,
        SourceFile,
    };
    use crate::input::translation::Translation;
    use crate::test_utils::create_translation_with_namespace;

    #[rstest]
    fn test_generate_diagnostics_with_missing_key() {
        let db = I18nDatabaseImpl::default();

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

        let mut keys = HashMap::new();
        keys.insert("common.hello".to_string(), "Hello".to_string());
        keys.insert("common.goodbye".to_string(), "Goodbye".to_string());

        let translation = Translation::new(
            &db,
            "en".to_string(),
            None,
            "en.json".to_string(),
            keys,
            String::new(),
            HashMap::new(),
            HashMap::new(),
        );

        let options = DiagnosticOptions::default();
        let diagnostics =
            generate_diagnostics(&db, source_file, &[translation], &options, ".", None, None);

        assert_that!(diagnostics, not(is_empty()));
        assert_that!(
            diagnostics,
            contains(field!(Diagnostic.message, contains_substring("common.missing")))
        );
        assert_that!(
            diagnostics,
            each(field!(Diagnostic.severity, some(eq(&DiagnosticSeverity::WARNING))))
        );
        assert_that!(
            diagnostics,
            each(field!(
                Diagnostic.code,
                some(eq(&NumberOrString::String("missing-translation".to_string())))
            ))
        );
    }

    #[rstest]
    fn test_generate_diagnostics_all_keys_exist() {
        let db = I18nDatabaseImpl::default();

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

        let mut keys = HashMap::new();
        keys.insert("common.hello".to_string(), "Hello".to_string());
        keys.insert("common.goodbye".to_string(), "Goodbye".to_string());

        let translation = Translation::new(
            &db,
            "en".to_string(),
            None,
            "en.json".to_string(),
            keys,
            String::new(),
            HashMap::new(),
            HashMap::new(),
        );

        let options = DiagnosticOptions::default();
        let diagnostics =
            generate_diagnostics(&db, source_file, &[translation], &options, ".", None, None);

        assert_that!(diagnostics, is_empty());
    }

    #[rstest]
    fn test_generate_diagnostics_multiple_translations() {
        let db = I18nDatabaseImpl::default();

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

        let mut keys_en = HashMap::new();
        keys_en.insert("common.hello".to_string(), "Hello".to_string());

        let mut keys_ja = HashMap::new();
        keys_ja.insert("errors.notFound".to_string(), "見つかりません".to_string());

        let translation_en = Translation::new(
            &db,
            "en".to_string(),
            None,
            "en.json".to_string(),
            keys_en,
            String::new(),
            HashMap::new(),
            HashMap::new(),
        );
        let translation_ja = Translation::new(
            &db,
            "ja".to_string(),
            None,
            "ja.json".to_string(),
            keys_ja,
            String::new(),
            HashMap::new(),
            HashMap::new(),
        );

        let options = DiagnosticOptions::default();
        let diagnostics = generate_diagnostics(
            &db,
            source_file,
            &[translation_en, translation_ja],
            &options,
            ".",
            None,
            None,
        );

        // common.hello is missing in ja, errors.notFound is missing in en
        assert_that!(diagnostics, len(eq(2)));
        assert_that!(
            diagnostics,
            contains(all![
                field!(Diagnostic.message, contains_substring("common.hello")),
                field!(Diagnostic.message, contains_substring("ja"))
            ])
        );
        assert_that!(
            diagnostics,
            contains(all![
                field!(Diagnostic.message, contains_substring("errors.notFound")),
                field!(Diagnostic.message, contains_substring("en"))
            ])
        );
    }

    #[rstest]
    fn test_determine_target_languages_with_required() {
        let all_languages: HashSet<String> =
            ["en", "ja", "zh"].iter().map(|s| s.to_string()).collect();
        let options = DiagnosticOptions {
            required_languages: Some(["en", "ja"].iter().map(|s| s.to_string()).collect()),
            ..DiagnosticOptions::default()
        };

        let target = determine_target_languages(&all_languages, &options);

        assert_that!(target, len(eq(2)));
        assert_that!(target, contains(eq(&"en")));
        assert_that!(target, contains(eq(&"ja")));
        assert_that!(target, not(contains(eq(&"zh"))));
    }

    #[rstest]
    fn test_determine_target_languages_with_optional() {
        let all_languages: HashSet<String> =
            ["en", "ja", "zh"].iter().map(|s| s.to_string()).collect();
        let options = DiagnosticOptions {
            optional_languages: Some(["zh"].iter().map(|s| s.to_string()).collect()),
            ..DiagnosticOptions::default()
        };

        let target = determine_target_languages(&all_languages, &options);

        assert_that!(target, len(eq(2)));
        assert_that!(target, contains(eq(&"en")));
        assert_that!(target, contains(eq(&"ja")));
        assert_that!(target, not(contains(eq(&"zh"))));
    }

    #[rstest]
    fn test_generate_diagnostics_disabled() {
        let db = I18nDatabaseImpl::default();

        let source_file = SourceFile::new(
            &db,
            "test.ts".to_string(),
            r#"const msg = t("missing.key");"#.to_string(),
            ProgrammingLanguage::TypeScript,
        );
        let translation = Translation::new(
            &db,
            "en".to_string(),
            None,
            "en.json".to_string(),
            HashMap::new(),
            String::new(),
            HashMap::new(),
            HashMap::new(),
        );

        let options = DiagnosticOptions { enabled: false, ..DiagnosticOptions::default() };
        let diagnostics =
            generate_diagnostics(&db, source_file, &[translation], &options, ".", None, None);

        assert_that!(diagnostics, is_empty());
    }

    #[rstest]
    fn test_generate_diagnostics_custom_severity() {
        let db = I18nDatabaseImpl::default();

        let source_file = SourceFile::new(
            &db,
            "test.ts".to_string(),
            r#"const msg = t("missing.key");"#.to_string(),
            ProgrammingLanguage::TypeScript,
        );
        let translation = Translation::new(
            &db,
            "en".to_string(),
            None,
            "en.json".to_string(),
            HashMap::new(),
            String::new(),
            HashMap::new(),
            HashMap::new(),
        );

        let options =
            DiagnosticOptions { severity: Severity::Error, ..DiagnosticOptions::default() };
        let diagnostics =
            generate_diagnostics(&db, source_file, &[translation], &options, ".", None, None);

        assert_that!(diagnostics, not(is_empty()));
        assert_that!(
            diagnostics,
            each(field!(Diagnostic.severity, some(eq(&DiagnosticSeverity::ERROR))))
        );
    }

    #[rstest]
    fn test_is_key_used_exact_match() {
        let used_keys: HashSet<String> =
            ["common.hello", "common.goodbye"].iter().map(|s| s.to_string()).collect();

        assert_that!(is_key_used("common.hello", &used_keys, "."), eq(true));
        assert_that!(is_key_used("common.goodbye", &used_keys, "."), eq(true));
        assert_that!(is_key_used("common.missing", &used_keys, "."), eq(false));
    }

    #[rstest]
    fn test_is_key_used_prefix_match() {
        // When t('hoge.fuga') is used, hoge.fuga.piyo is considered used
        let used_keys: HashSet<String> = ["hoge.fuga"].iter().map(|s| s.to_string()).collect();

        assert_that!(is_key_used("hoge.fuga", &used_keys, "."), eq(true));
        assert_that!(is_key_used("hoge.fuga.piyo", &used_keys, "."), eq(true));
        assert_that!(is_key_used("hoge.fuga.piyo.deep", &used_keys, "."), eq(true));
        // hoge.fugaX does not prefix match (doesn't start with hoge.fuga.)
        assert_that!(is_key_used("hoge.fugaX", &used_keys, "."), eq(false));
        assert_that!(is_key_used("other.key", &used_keys, "."), eq(false));
    }

    #[rstest]
    fn test_is_key_used_with_custom_separator() {
        let used_keys: HashSet<String> = ["hoge:fuga"].iter().map(|s| s.to_string()).collect();

        assert_that!(is_key_used("hoge:fuga", &used_keys, ":"), eq(true));
        assert_that!(is_key_used("hoge:fuga:piyo", &used_keys, ":"), eq(true));
        // Dot is not the separator, so no prefix match
        assert_that!(is_key_used("hoge:fuga.piyo", &used_keys, ":"), eq(false));
    }

    #[rstest]
    fn test_generate_unused_key_diagnostics_basic() {
        let db = I18nDatabaseImpl::default();

        let source_code = r#"const msg = t("common.hello");"#;
        let source_file = SourceFile::new(
            &db,
            "test.ts".to_string(),
            source_code.to_string(),
            ProgrammingLanguage::TypeScript,
        );

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
            None,
            "en.json".to_string(),
            keys,
            String::new(),
            key_ranges,
            HashMap::new(),
        );

        let diagnostics = generate_unused_key_diagnostics(
            &db,
            translation,
            &[source_file],
            ".",
            &[],
            Severity::Hint,
            None,
            None,
        );

        assert_that!(diagnostics, len(eq(1)));
        assert_that!(
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

    #[rstest]
    fn test_generate_unused_key_diagnostics_with_prefix_match() {
        let db = I18nDatabaseImpl::default();

        // Source uses hoge.fuga (fetches entire object)
        let source_code = r#"const obj = t("hoge.fuga");"#;
        let source_file = SourceFile::new(
            &db,
            "test.ts".to_string(),
            source_code.to_string(),
            ProgrammingLanguage::TypeScript,
        );

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
            None,
            "en.json".to_string(),
            keys,
            String::new(),
            key_ranges,
            HashMap::new(),
        );

        let diagnostics = generate_unused_key_diagnostics(
            &db,
            translation,
            &[source_file],
            ".",
            &[],
            Severity::Hint,
            None,
            None,
        );

        // hoge.fuga and hoge.fuga.piyo are used (prefix match)
        // Only other.key is detected as unused
        assert_that!(diagnostics, len(eq(1)));
        assert_that!(
            diagnostics,
            contains(field!(Diagnostic.message, contains_substring("other.key")))
        );
    }

    #[rstest]
    fn test_generate_unused_key_diagnostics_with_ignore_patterns() {
        let db = I18nDatabaseImpl::default();

        let source_code = r#"const msg = t("used.key");"#;
        let source_file = SourceFile::new(
            &db,
            "test.ts".to_string(),
            source_code.to_string(),
            ProgrammingLanguage::TypeScript,
        );

        let mut keys = HashMap::new();
        keys.insert("used.key".to_string(), "Used".to_string());
        keys.insert("debug.info".to_string(), "Debug Info".to_string());
        keys.insert("debug.warn".to_string(), "Debug Warn".to_string());
        keys.insert("other.unused".to_string(), "Other".to_string());

        let mut key_ranges = HashMap::new();
        for key in keys.keys() {
            key_ranges.insert(
                key.clone(),
                crate::types::SourceRange {
                    start: crate::types::SourcePosition { line: 1, character: 0 },
                    end: crate::types::SourcePosition { line: 1, character: 10 },
                },
            );
        }

        let translation = Translation::new(
            &db,
            "en".to_string(),
            None,
            "en.json".to_string(),
            keys,
            String::new(),
            key_ranges,
            HashMap::new(),
        );

        let ignore_patterns = vec!["debug.*".to_string()];
        let diagnostics = generate_unused_key_diagnostics(
            &db,
            translation,
            &[source_file],
            ".",
            &ignore_patterns,
            Severity::Hint,
            None,
            None,
        );

        // debug.info and debug.warn are ignored, only other.unused is reported
        assert_that!(diagnostics, len(eq(1)));
        assert_that!(
            diagnostics,
            contains(field!(Diagnostic.message, contains_substring("other.unused")))
        );
    }

    #[rstest]
    fn test_generate_unused_key_diagnostics_custom_severity() {
        let db = I18nDatabaseImpl::default();

        let source_code = r#"const msg = t("used.key");"#;
        let source_file = SourceFile::new(
            &db,
            "test.ts".to_string(),
            source_code.to_string(),
            ProgrammingLanguage::TypeScript,
        );

        let mut keys = HashMap::new();
        keys.insert("used.key".to_string(), "Used".to_string());
        keys.insert("unused.key".to_string(), "Unused".to_string());

        let mut key_ranges = HashMap::new();
        for key in keys.keys() {
            key_ranges.insert(
                key.clone(),
                crate::types::SourceRange {
                    start: crate::types::SourcePosition { line: 1, character: 0 },
                    end: crate::types::SourcePosition { line: 1, character: 10 },
                },
            );
        }

        let translation = Translation::new(
            &db,
            "en".to_string(),
            None,
            "en.json".to_string(),
            keys,
            String::new(),
            key_ranges,
            HashMap::new(),
        );

        let diagnostics = generate_unused_key_diagnostics(
            &db,
            translation,
            &[source_file],
            ".",
            &[],
            Severity::Warning,
            None,
            None,
        );

        assert_that!(diagnostics, len(eq(1)));
        assert_that!(
            diagnostics,
            each(field!(Diagnostic.severity, some(eq(&DiagnosticSeverity::WARNING))))
        );
    }

    #[rstest]
    fn test_has_keys_with_prefix() {
        let keys: HashSet<String> =
            ["nested.key", "nested.foo", "other.key"].iter().map(|s| s.to_string()).collect();

        assert_that!(has_keys_with_prefix("nested", &keys, "."), eq(true));
        assert_that!(has_keys_with_prefix("other", &keys, "."), eq(true));
        assert_that!(has_keys_with_prefix("missing", &keys, "."), eq(false));
        // "nest" is not a prefix of "nested.key" (requires separator)
        assert_that!(has_keys_with_prefix("nest", &keys, "."), eq(false));
    }

    #[rstest]
    fn test_key_exists_or_has_children() {
        let keys: HashSet<String> =
            ["nested.key", "nested.foo", "single"].iter().map(|s| s.to_string()).collect();

        assert_that!(key_exists_or_has_children("single", &keys, "."), eq(true));
        assert_that!(key_exists_or_has_children("nested.key", &keys, "."), eq(true));

        // Reverse prefix match (child keys exist)
        assert_that!(key_exists_or_has_children("nested", &keys, "."), eq(true));

        assert_that!(key_exists_or_has_children("missing", &keys, "."), eq(false));
        // "singl" is not a prefix of "single" (requires separator)
        assert_that!(key_exists_or_has_children("singl", &keys, "."), eq(false));
    }

    #[rstest]
    fn test_generate_diagnostics_with_reverse_prefix_match() {
        let db = I18nDatabaseImpl::default();

        // Uses t('nested') where nested key doesn't exist but nested.key does
        let source_code = r#"const msg = t("nested");"#;
        let source_file = SourceFile::new(
            &db,
            "test.ts".to_string(),
            source_code.to_string(),
            ProgrammingLanguage::TypeScript,
        );

        let mut keys = HashMap::new();
        keys.insert("nested.key".to_string(), "Key Value".to_string());
        keys.insert("nested.foo".to_string(), "Foo Value".to_string());

        let translation = Translation::new(
            &db,
            "en".to_string(),
            None,
            "en.json".to_string(),
            keys,
            String::new(),
            HashMap::new(),
            HashMap::new(),
        );

        let options = DiagnosticOptions::default();
        let diagnostics =
            generate_diagnostics(&db, source_file, &[translation], &options, ".", None, None);

        // Since nested.key exists, nested is valid (no diagnostics)
        assert_that!(diagnostics, is_empty());
    }

    #[rstest]
    fn test_generate_diagnostics_no_reverse_prefix_match() {
        let db = I18nDatabaseImpl::default();

        // Uses t('missing') where neither missing nor missing.* exist
        let source_code = r#"const msg = t("missing");"#;
        let source_file = SourceFile::new(
            &db,
            "test.ts".to_string(),
            source_code.to_string(),
            ProgrammingLanguage::TypeScript,
        );

        let mut keys = HashMap::new();
        keys.insert("other.key".to_string(), "Other Value".to_string());

        let translation = Translation::new(
            &db,
            "en".to_string(),
            None,
            "en.json".to_string(),
            keys,
            String::new(),
            HashMap::new(),
            HashMap::new(),
        );

        let options = DiagnosticOptions::default();
        let diagnostics =
            generate_diagnostics(&db, source_file, &[translation], &options, ".", None, None);

        assert_that!(diagnostics, len(eq(1)));
        assert_that!(
            diagnostics,
            contains(field!(Diagnostic.message, contains_substring("missing")))
        );
    }

    #[rstest]
    fn test_is_key_used_with_array_prefix() {
        // When t('items') is used, items[0] is considered used
        let used_keys: HashSet<String> = ["items"].iter().map(|s| s.to_string()).collect();

        assert_that!(is_key_used("items", &used_keys, "."), eq(true));
        assert_that!(is_key_used("items[0]", &used_keys, "."), eq(true));
        assert_that!(is_key_used("items[0].name", &used_keys, "."), eq(true));
        assert_that!(is_key_used("items[1]", &used_keys, "."), eq(true));
        // itemsX does not match (no separator/bracket)
        assert_that!(is_key_used("itemsX", &used_keys, "."), eq(false));
    }

    #[rstest]
    fn test_has_keys_with_prefix_with_array() {
        let keys: HashSet<String> =
            ["items[0]", "items[1]", "other.key"].iter().map(|s| s.to_string()).collect();

        assert_that!(has_keys_with_prefix("items", &keys, "."), eq(true));
        assert_that!(has_keys_with_prefix("other", &keys, "."), eq(true));
        assert_that!(has_keys_with_prefix("missing", &keys, "."), eq(false));
    }

    #[rstest]
    fn test_key_exists_or_has_children_with_array() {
        let keys: HashSet<String> =
            ["items[0]", "items[1]", "single"].iter().map(|s| s.to_string()).collect();

        assert_that!(key_exists_or_has_children("single", &keys, "."), eq(true));
        // Array children exist
        assert_that!(key_exists_or_has_children("items", &keys, "."), eq(true));
        assert_that!(key_exists_or_has_children("missing", &keys, "."), eq(false));
    }

    // ===== Namespace-aware diagnostics tests =====

    #[rstest]
    fn test_generate_diagnostics_with_namespace_separator() {
        let db = I18nDatabaseImpl::default();

        // Source uses namespaced key "common:hello"
        let source_code = r#"const msg = t("common:hello");"#;
        let source_file = SourceFile::new(
            &db,
            "test.ts".to_string(),
            source_code.to_string(),
            ProgrammingLanguage::TypeScript,
        );

        let common_en = create_translation_with_namespace(
            &db,
            "en",
            Some("common"),
            "/locales/en/common.json",
            HashMap::from([("hello".to_string(), "Hello".to_string())]),
        );
        let common_ja = create_translation_with_namespace(
            &db,
            "ja",
            Some("common"),
            "/locales/ja/common.json",
            HashMap::from([("hello".to_string(), "こんにちは".to_string())]),
        );
        let errors_en = create_translation_with_namespace(
            &db,
            "en",
            Some("errors"),
            "/locales/en/errors.json",
            HashMap::from([("notFound".to_string(), "Not Found".to_string())]),
        );

        let translations = vec![common_en, common_ja, errors_en];
        let options = DiagnosticOptions::default();

        // With namespace_separator=":", "common:hello" should only check common namespace
        let diagnostics =
            generate_diagnostics(&db, source_file, &translations, &options, ".", Some(":"), None);

        // "hello" exists in both en/common and ja/common → no diagnostics
        assert_that!(diagnostics, is_empty());
    }

    #[rstest]
    fn test_generate_diagnostics_namespace_key_missing_in_correct_ns() {
        let db = I18nDatabaseImpl::default();

        // Source uses namespaced key "errors:missing"
        let source_code = r#"const msg = t("errors:missing");"#;
        let source_file = SourceFile::new(
            &db,
            "test.ts".to_string(),
            source_code.to_string(),
            ProgrammingLanguage::TypeScript,
        );

        let errors_en = create_translation_with_namespace(
            &db,
            "en",
            Some("errors"),
            "/locales/en/errors.json",
            HashMap::from([("notFound".to_string(), "Not Found".to_string())]),
        );
        let common_en = create_translation_with_namespace(
            &db,
            "en",
            Some("common"),
            "/locales/en/common.json",
            HashMap::from([("missing".to_string(), "Missing".to_string())]),
        );

        let translations = vec![errors_en, common_en];
        let options = DiagnosticOptions::default();

        let diagnostics =
            generate_diagnostics(&db, source_file, &translations, &options, ".", Some(":"), None);

        // "missing" does NOT exist in errors namespace → should report missing for en
        assert_that!(diagnostics, len(eq(1)));
        assert_that!(
            diagnostics,
            contains(field!(Diagnostic.message, contains_substring("errors:missing")))
        );
    }

    #[rstest]
    fn test_generate_diagnostics_with_declared_namespace() {
        let db = I18nDatabaseImpl::default();

        // useTranslation("common") + t("hello") → namespace from scope
        let source_code = r#"
            const { t } = useTranslation("common");
            const msg = t("hello");
        "#;
        let source_file = SourceFile::new(
            &db,
            "test.ts".to_string(),
            source_code.to_string(),
            ProgrammingLanguage::TypeScript,
        );

        let common_en = create_translation_with_namespace(
            &db,
            "en",
            Some("common"),
            "/locales/en/common.json",
            HashMap::from([("hello".to_string(), "Hello".to_string())]),
        );
        let errors_en = create_translation_with_namespace(
            &db,
            "en",
            Some("errors"),
            "/locales/en/errors.json",
            HashMap::from([("notFound".to_string(), "Not Found".to_string())]),
        );

        let translations = vec![common_en, errors_en];
        let options = DiagnosticOptions::default();

        // namespace_separator is None but declared namespace from useTranslation is used
        let diagnostics =
            generate_diagnostics(&db, source_file, &translations, &options, ".", None, None);

        // "hello" exists in common → no diagnostics
        assert_that!(diagnostics, is_empty());
    }

    #[rstest]
    fn test_generate_diagnostics_with_default_namespace() {
        let db = I18nDatabaseImpl::default();

        // t("hello") without explicit namespace, default_namespace="common"
        let source_code = r#"const msg = t("hello");"#;
        let source_file = SourceFile::new(
            &db,
            "test.ts".to_string(),
            source_code.to_string(),
            ProgrammingLanguage::TypeScript,
        );

        let common_en = create_translation_with_namespace(
            &db,
            "en",
            Some("common"),
            "/locales/en/common.json",
            HashMap::from([("hello".to_string(), "Hello".to_string())]),
        );
        let errors_en = create_translation_with_namespace(
            &db,
            "en",
            Some("errors"),
            "/locales/en/errors.json",
            HashMap::from([("notFound".to_string(), "Not Found".to_string())]),
        );

        let translations = vec![common_en, errors_en];
        let options = DiagnosticOptions::default();

        let diagnostics = generate_diagnostics(
            &db,
            source_file,
            &translations,
            &options,
            ".",
            Some(":"),
            Some("common"), // default namespace
        );

        // "hello" exists in common (default namespace) → no diagnostics
        assert_that!(diagnostics, is_empty());
    }

    #[rstest]
    fn test_generate_unused_key_diagnostics_with_namespace() {
        let db = I18nDatabaseImpl::default();

        // Source uses "common:hello" (namespace separator = ":")
        let source_code = r#"const msg = t("common:hello");"#;
        let source_file = SourceFile::new(
            &db,
            "test.ts".to_string(),
            source_code.to_string(),
            ProgrammingLanguage::TypeScript,
        );

        let mut keys = HashMap::new();
        keys.insert("hello".to_string(), "Hello".to_string());
        keys.insert("unused".to_string(), "Unused".to_string());

        let mut key_ranges = HashMap::new();
        for key in keys.keys() {
            key_ranges.insert(
                key.clone(),
                crate::types::SourceRange {
                    start: crate::types::SourcePosition { line: 1, character: 0 },
                    end: crate::types::SourcePosition { line: 1, character: 10 },
                },
            );
        }

        let translation = Translation::new(
            &db,
            "en".to_string(),
            Some("common".to_string()),
            "/locales/en/common.json".to_string(),
            keys,
            String::new(),
            key_ranges,
            HashMap::new(),
        );

        let diagnostics = generate_unused_key_diagnostics(
            &db,
            translation,
            &[source_file],
            ".",
            &[],
            Severity::Hint,
            Some(":"),
            None,
        );

        // "hello" is used (common:hello matches common namespace), "unused" is unused
        assert_that!(diagnostics, len(eq(1)));
        assert_that!(
            diagnostics,
            contains(field!(Diagnostic.message, contains_substring("unused")))
        );
    }

    #[rstest]
    fn test_generate_unused_key_diagnostics_cross_namespace() {
        let db = I18nDatabaseImpl::default();

        // Source uses "common:hello" — targeting common namespace only
        let source_code = r#"const msg = t("common:hello");"#;
        let source_file = SourceFile::new(
            &db,
            "test.ts".to_string(),
            source_code.to_string(),
            ProgrammingLanguage::TypeScript,
        );

        let mut keys = HashMap::new();
        keys.insert("hello".to_string(), "Hello in errors".to_string());

        let mut key_ranges = HashMap::new();
        key_ranges.insert(
            "hello".to_string(),
            crate::types::SourceRange {
                start: crate::types::SourcePosition { line: 1, character: 0 },
                end: crate::types::SourcePosition { line: 1, character: 10 },
            },
        );

        // Translation is in "errors" namespace, not "common"
        let translation = Translation::new(
            &db,
            "en".to_string(),
            Some("errors".to_string()),
            "/locales/en/errors.json".to_string(),
            keys,
            String::new(),
            key_ranges,
            HashMap::new(),
        );

        let diagnostics = generate_unused_key_diagnostics(
            &db,
            translation,
            &[source_file],
            ".",
            &[],
            Severity::Hint,
            Some(":"),
            None,
        );

        // "hello" in errors namespace is unused because source targets common namespace
        assert_that!(diagnostics, len(eq(1)));
        assert_that!(
            diagnostics,
            contains(field!(Diagnostic.message, contains_substring("hello")))
        );
    }
}
