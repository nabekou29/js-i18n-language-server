pub mod analyzer;
pub mod svelte;

use crate::db::I18nDatabase;
use crate::input::source::{
    ProgrammingLanguage,
    SourceFile,
};
use crate::interned::TransKey;
use crate::ir::key_usage::KeyUsage;
use crate::types::{
    SourcePosition,
    SourceRange,
};

/// Analyzes a source file and extracts key usages.
#[salsa::tracked]
#[allow(clippy::needless_pass_by_value)]
pub fn analyze_source(
    db: &dyn I18nDatabase,
    file: SourceFile,
    key_separator: String,
) -> Vec<KeyUsage<'_>> {
    let text = file.text(db);
    let language = file.language(db);

    if language == ProgrammingLanguage::Svelte {
        return analyze_svelte_source(db, text, &key_separator);
    }

    let tree_sitter_lang = language.tree_sitter_language();
    let queries = analyzer::query_loader::load_queries(language);

    let trans_fn_calls = analyzer::extractor::analyze_trans_fn_calls(
        text,
        &tree_sitter_lang,
        queries,
        &key_separator,
    )
    .unwrap_or_default();

    trans_fn_calls
        .into_iter()
        .map(|call| {
            let key = TransKey::new(db, call.key);
            let range: SourceRange = call.arg_key_node.into();
            KeyUsage::new(db, key, range, call.namespace, call.namespaces)
        })
        .collect()
}

/// Svelte-specific analysis: extract JS/TS regions, parse, and remap positions.
fn analyze_svelte_source<'db>(
    db: &'db dyn I18nDatabase,
    text: &str,
    key_separator: &str,
) -> Vec<KeyUsage<'db>> {
    let extraction = svelte::extract(text);
    let ts_lang = ProgrammingLanguage::TypeScript.tree_sitter_language();
    let queries = analyzer::query_loader::load_queries(ProgrammingLanguage::Svelte);

    let trans_fn_calls = analyzer::extractor::analyze_trans_fn_calls(
        &extraction.virtual_doc,
        &ts_lang,
        queries,
        key_separator,
    )
    .unwrap_or_default();

    trans_fn_calls
        .into_iter()
        .map(|call| {
            let key = TransKey::new(db, call.key);
            let remapped_range = extraction.position_map.remap(call.arg_key_node);
            let range: SourceRange = remapped_range.into();
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
