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
