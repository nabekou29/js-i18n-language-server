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
        if position_in_range(position, usage.range(db)) {
            return Some(usage.key(db));
        }
    }

    None
}

/// 位置が範囲内にあるかをチェック
const fn position_in_range(position: SourcePosition, range: SourceRange) -> bool {
    // 開始位置より前の場合
    if position.line < range.start.line {
        return false;
    }
    if position.line == range.start.line && position.character < range.start.character {
        return false;
    }

    // 終了位置より後の場合
    if position.line > range.end.line {
        return false;
    }
    if position.line == range.end.line && position.character > range.end.character {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    /// テスト用ヘルパー: `SourcePosition` を作成
    const fn pos(line: u32, character: u32) -> SourcePosition {
        SourcePosition { line, character }
    }

    /// テスト用ヘルパー: `SourceRange` を作成
    const fn range(start_line: u32, start_char: u32, end_line: u32, end_char: u32) -> SourceRange {
        SourceRange { start: pos(start_line, start_char), end: pos(end_line, end_char) }
    }

    // position_in_range の境界条件テスト
    // 範囲: (1, 5) から (2, 10) を使用

    #[rstest]
    #[case::before_start_line(pos(0, 5), range(1, 5, 2, 10), false)]
    #[case::before_start_char(pos(1, 4), range(1, 5, 2, 10), false)]
    #[case::at_start(pos(1, 5), range(1, 5, 2, 10), true)]
    #[case::after_start_same_line(pos(1, 6), range(1, 5, 2, 10), true)]
    #[case::middle_line(pos(1, 10), range(1, 5, 2, 10), true)]
    #[case::end_line_before_end_char(pos(2, 5), range(1, 5, 2, 10), true)]
    #[case::at_end(pos(2, 10), range(1, 5, 2, 10), true)]
    #[case::after_end_char(pos(2, 11), range(1, 5, 2, 10), false)]
    #[case::after_end_line(pos(3, 0), range(1, 5, 2, 10), false)]
    fn test_position_in_range(
        #[case] position: SourcePosition,
        #[case] range: SourceRange,
        #[case] expected: bool,
    ) {
        assert_eq!(position_in_range(position, range), expected);
    }

    // 同一行内での境界テスト
    #[rstest]
    #[case::same_line_before(pos(1, 4), range(1, 5, 1, 10), false)]
    #[case::same_line_at_start(pos(1, 5), range(1, 5, 1, 10), true)]
    #[case::same_line_middle(pos(1, 7), range(1, 5, 1, 10), true)]
    #[case::same_line_at_end(pos(1, 10), range(1, 5, 1, 10), true)]
    #[case::same_line_after(pos(1, 11), range(1, 5, 1, 10), false)]
    fn test_position_in_range_same_line(
        #[case] position: SourcePosition,
        #[case] range: SourceRange,
        #[case] expected: bool,
    ) {
        assert_eq!(position_in_range(position, range), expected);
    }
}
