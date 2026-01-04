//! プロジェクト全体で使用される基本型定義

use tower_lsp::lsp_types;

/// ソースコード内の範囲を表す
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceRange {
    /// 開始位置
    pub start: SourcePosition,
    /// 終了位置
    pub end: SourcePosition,
}

impl From<lsp_types::Range> for SourceRange {
    fn from(range: lsp_types::Range) -> Self {
        Self { start: range.start.into(), end: range.end.into() }
    }
}

impl From<SourceRange> for lsp_types::Range {
    fn from(range: SourceRange) -> Self {
        Self { start: range.start.into(), end: range.end.into() }
    }
}

/// ソースコード内の位置を表す
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourcePosition {
    /// 行（0-indexed）
    pub line: u32,
    /// 文字位置（0-indexed）
    pub character: u32,
}

impl From<lsp_types::Position> for SourcePosition {
    fn from(position: lsp_types::Position) -> Self {
        Self { line: position.line, character: position.character }
    }
}

impl From<SourcePosition> for lsp_types::Position {
    fn from(position: SourcePosition) -> Self {
        Self { line: position.line, character: position.character }
    }
}

impl SourceRange {
    /// 指定した位置が範囲内にあるかをチェック
    ///
    /// # Arguments
    /// * `position` - チェックする位置
    ///
    /// # Returns
    /// 位置が範囲内にあれば `true`、そうでなければ `false`
    #[must_use]
    pub const fn contains(&self, position: SourcePosition) -> bool {
        // 開始位置より前の場合
        if position.line < self.start.line {
            return false;
        }
        if position.line == self.start.line && position.character < self.start.character {
            return false;
        }

        // 終了位置より後の場合
        if position.line > self.end.line {
            return false;
        }
        if position.line == self.end.line && position.character > self.end.character {
            return false;
        }

        true
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use googletest::prelude::*;
    use rstest::rstest;

    use super::*;

    /// ヘルパー関数: SourcePosition を作成
    const fn pos(line: u32, character: u32) -> SourcePosition {
        SourcePosition { line, character }
    }

    /// ヘルパー関数: SourceRange を作成
    const fn range(start_line: u32, start_char: u32, end_line: u32, end_char: u32) -> SourceRange {
        SourceRange { start: pos(start_line, start_char), end: pos(end_line, end_char) }
    }

    // SourceRange::contains の境界条件テスト
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
    fn test_contains(
        #[case] position: SourcePosition,
        #[case] range: SourceRange,
        #[case] expected: bool,
    ) {
        assert_that!(range.contains(position), eq(expected));
    }

    // 同一行内での境界テスト
    #[rstest]
    #[case::same_line_before(pos(1, 4), range(1, 5, 1, 10), false)]
    #[case::same_line_at_start(pos(1, 5), range(1, 5, 1, 10), true)]
    #[case::same_line_middle(pos(1, 7), range(1, 5, 1, 10), true)]
    #[case::same_line_at_end(pos(1, 10), range(1, 5, 1, 10), true)]
    #[case::same_line_after(pos(1, 11), range(1, 5, 1, 10), false)]
    fn test_contains_same_line(
        #[case] position: SourcePosition,
        #[case] range: SourceRange,
        #[case] expected: bool,
    ) {
        assert_that!(range.contains(position), eq(expected));
    }
}
