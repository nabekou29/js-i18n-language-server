pub mod analyzer;

use crate::db::I18nDatabase;
use crate::input::source::SourceFile;
use crate::interned::TransKey;
use crate::ir::key_usage::KeyUsage;
use crate::types::{
    SourcePosition,
    SourceRange,
};

/// ソースファイルを解析してキー使用箇所を抽出
#[salsa::tracked]
#[allow(clippy::needless_pass_by_value)] // Salsa tracked 関数では所有型が必要
pub fn analyze_source(
    db: &dyn I18nDatabase,
    file: SourceFile,
    key_separator: String,
) -> Vec<KeyUsage<'_>> {
    let text = file.text(db);
    let language = file.language(db);
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
            KeyUsage::new(db, key, range)
        })
        .collect()
}

/// 特定位置にあるキーを取得（Salsa クエリ）
#[salsa::tracked]
#[allow(clippy::needless_pass_by_value)] // Salsa tracked 関数では所有型が必要
pub fn key_at_position(
    db: &dyn I18nDatabase,
    file: SourceFile,
    position: SourcePosition,
    key_separator: String,
) -> Option<TransKey<'_>> {
    let usages = analyze_source(db, file, key_separator);

    for usage in usages {
        if usage.range(db).contains(position) {
            return Some(usage.key(db));
        }
    }

    None
}
