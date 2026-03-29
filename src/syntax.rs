pub mod analyzer;
pub mod position_map;
pub mod svelte;

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
