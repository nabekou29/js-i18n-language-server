pub mod analyzer;
pub mod position_map;
pub mod svelte;
pub mod vue;

use std::borrow::Cow;

use crate::db::I18nDatabase;
use crate::input::source::{
    ProgrammingLanguage,
    SourceFile,
};
use crate::interned::TransKey;
use crate::ir::key_usage::KeyUsage;
use crate::syntax::position_map::SourcePreprocessed;
use crate::types::{
    SourcePosition,
    SourceRange,
};

/// Preprocess source text for analysis.
///
/// Embedded-template languages (Svelte) extract JS/TS regions into a virtual document.
/// Other languages pass through unchanged, avoiding allocation via `Cow::Borrowed`.
pub(crate) fn preprocess(text: &str, language: ProgrammingLanguage) -> SourcePreprocessed<'_> {
    match language {
        ProgrammingLanguage::Svelte => {
            let extraction = svelte::extract(text);
            SourcePreprocessed {
                source: Cow::Owned(extraction.virtual_doc),
                position_map: Some(extraction.position_map),
            }
        }
        ProgrammingLanguage::Vue => {
            let extraction = vue::extract(text);
            SourcePreprocessed {
                source: Cow::Owned(extraction.virtual_doc),
                position_map: Some(extraction.position_map),
            }
        }
        _ => SourcePreprocessed { source: Cow::Borrowed(text), position_map: None },
    }
}

/// Analyzes a source file and extracts key usages.
///
/// Uses a unified pipeline: preprocess → parse → remap positions.
/// Embedded-template languages (Svelte) extract JS/TS regions first;
/// other languages pass through unchanged.
#[salsa::tracked]
#[allow(clippy::needless_pass_by_value)]
pub fn analyze_source(
    db: &dyn I18nDatabase,
    file: SourceFile,
    key_separator: String,
) -> Vec<KeyUsage<'_>> {
    let text = file.text(db);
    let language = file.language(db);

    let preprocessed = preprocess(text, language);
    let tree_sitter_lang = language.tree_sitter_language();
    let queries = analyzer::query_loader::load_queries(language);

    let trans_fn_calls = analyzer::extractor::analyze_trans_fn_calls(
        &preprocessed.source,
        &tree_sitter_lang,
        language,
        queries,
        &key_separator,
    )
    .unwrap_or_default();

    trans_fn_calls
        .into_iter()
        .map(|call| {
            let key = TransKey::new(db, call.key);
            let range: SourceRange = preprocessed
                .position_map
                .as_ref()
                .map_or_else(|| call.arg_key_node.into(), |pm| pm.remap(call.arg_key_node).into());
            KeyUsage::new(db, key, range, call.namespace, call.namespaces)
        })
        .collect()
}

/// Finds a key usage (with namespace context) at a specific position.
#[salsa::tracked]
#[allow(clippy::needless_pass_by_value)]
pub fn key_usage_at_position(
    db: &dyn I18nDatabase,
    file: SourceFile,
    position: SourcePosition,
    key_separator: String,
) -> Option<KeyUsage<'_>> {
    let usages = analyze_source(db, file, key_separator);
    usages.into_iter().find(|usage| usage.range(db).contains(position))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::expect_used, clippy::panic)]
mod tests {
    use googletest::prelude::*;
    use rstest::*;

    use super::*;
    use crate::db::I18nDatabaseImpl;
    use crate::input::source::ProgrammingLanguage;

    #[rstest]
    fn analyze_source_typescript() {
        let db = I18nDatabaseImpl::default();
        let source = r#"const { t } = useTranslation(); t("hello");"#;
        let file = SourceFile::new(
            &db,
            "test.ts".to_string(),
            source.to_string(),
            ProgrammingLanguage::TypeScript,
        );

        let usages = analyze_source(&db, file, ".".to_string());
        assert_that!(usages.len(), eq(1));
        assert_that!(usages[0].key(&db).text(&db), eq("hello"));
    }

    #[rstest]
    fn analyze_source_svelte_script_block() {
        let db = I18nDatabaseImpl::default();
        let source = "<script>\n  import { _ } from 'svelte-i18n';\n  $_('greeting');\n</script>";
        let file = SourceFile::new(
            &db,
            "test.svelte".to_string(),
            source.to_string(),
            ProgrammingLanguage::Svelte,
        );

        let usages = analyze_source(&db, file, ".".to_string());
        assert_that!(usages.len(), eq(1));
        assert_that!(usages[0].key(&db).text(&db), eq("greeting"));
    }

    #[rstest]
    fn analyze_source_svelte_template_expression() {
        let db = I18nDatabaseImpl::default();
        let source = "<p>{$_('template.key')}</p>";
        let file = SourceFile::new(
            &db,
            "test.svelte".to_string(),
            source.to_string(),
            ProgrammingLanguage::Svelte,
        );

        let usages = analyze_source(&db, file, ".".to_string());
        assert_that!(usages.len(), eq(1));
        assert_that!(usages[0].key(&db).text(&db), eq("template.key"));
    }

    #[rstest]
    fn analyze_source_svelte_remaps_positions() {
        let db = I18nDatabaseImpl::default();
        // Line 0: <script>
        // Line 1:   $_('key')
        // Line 2: </script>
        let source = "<script>\n  $_('key')\n</script>";
        let file = SourceFile::new(
            &db,
            "test.svelte".to_string(),
            source.to_string(),
            ProgrammingLanguage::Svelte,
        );

        let usages = analyze_source(&db, file, ".".to_string());
        assert_that!(usages.len(), eq(1));

        // The key 'key' should be remapped to original line 1 (inside <script>)
        let range = usages[0].range(&db);
        assert_that!(range.start.line, eq(1));
    }

    #[rstest]
    fn analyze_source_vue_script_block() {
        let db = I18nDatabaseImpl::default();
        let source = "<script setup>\nimport { useI18n } from 'vue-i18n'\nconst { t } = useI18n()\nt('greeting')\n</script>";
        let file = SourceFile::new(
            &db,
            "test.vue".to_string(),
            source.to_string(),
            ProgrammingLanguage::Vue,
        );

        let usages = analyze_source(&db, file, ".".to_string());
        assert_that!(usages.len(), eq(1));
        assert_that!(usages[0].key(&db).text(&db), eq("greeting"));
    }

    #[rstest]
    fn analyze_source_vue_template_expression() {
        let db = I18nDatabaseImpl::default();
        let source = "<template><p>{{ $t('template.key') }}</p></template>";
        let file = SourceFile::new(
            &db,
            "test.vue".to_string(),
            source.to_string(),
            ProgrammingLanguage::Vue,
        );

        let usages = analyze_source(&db, file, ".".to_string());
        assert_that!(usages.len(), eq(1));
        assert_that!(usages[0].key(&db).text(&db), eq("template.key"));
    }

    #[rstest]
    fn analyze_source_vue_remaps_positions() {
        let db = I18nDatabaseImpl::default();
        let source = "<script>\n  $t('key')\n</script>";
        let file = SourceFile::new(
            &db,
            "test.vue".to_string(),
            source.to_string(),
            ProgrammingLanguage::Vue,
        );

        let usages = analyze_source(&db, file, ".".to_string());
        assert_that!(usages.len(), eq(1));

        let range = usages[0].range(&db);
        assert_that!(range.start.line, eq(1));
    }

    #[rstest]
    fn analyze_source_vue_te_and_tm() {
        let db = I18nDatabaseImpl::default();
        let source = "<script setup>\nimport { useI18n } from 'vue-i18n'\nconst { t, te, tm } = useI18n()\nt('greeting')\nte('optional')\ntm('message')\n</script>";
        let file = SourceFile::new(
            &db,
            "test.vue".to_string(),
            source.to_string(),
            ProgrammingLanguage::Vue,
        );

        let usages = analyze_source(&db, file, ".".to_string());
        let keys: Vec<String> = usages.iter().map(|u| u.key(&db).text(&db).to_string()).collect();
        assert_that!(keys, contains_each![eq("greeting"), eq("optional"), eq("message")]);
    }

    #[rstest]
    fn preprocess_vue_extracts_virtual_doc() {
        let text = "<script>\n  $t('key')\n</script>";
        let result = preprocess(text, ProgrammingLanguage::Vue);

        assert!(result.position_map.is_some());
        assert!(matches!(result.source, Cow::Owned(_)));
        assert!(result.source.contains("$t('key')"));
    }

    #[rstest]
    fn preprocess_non_svelte_borrows() {
        let text = "const t = useTranslation();";
        let result = preprocess(text, ProgrammingLanguage::TypeScript);

        assert!(result.position_map.is_none());
        // Cow::Borrowed — no allocation
        assert!(matches!(result.source, Cow::Borrowed(_)));
        assert_eq!(&*result.source, text);
    }

    #[rstest]
    fn preprocess_svelte_extracts_virtual_doc() {
        let text = "<script>\n  $_('key')\n</script>";
        let result = preprocess(text, ProgrammingLanguage::Svelte);

        assert!(result.position_map.is_some());
        assert!(matches!(result.source, Cow::Owned(_)));
        assert!(result.source.contains("$_('key')"));
    }
}
