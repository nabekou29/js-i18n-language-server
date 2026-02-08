//! Core types used throughout the project.

use tower_lsp::lsp_types;

/// A range in source code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceRange {
    pub start: SourcePosition,
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

/// A position in source code (0-indexed).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourcePosition {
    pub line: u32,
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

impl From<tree_sitter::Point> for SourcePosition {
    #[allow(clippy::cast_possible_truncation)]
    fn from(point: tree_sitter::Point) -> Self {
        Self { line: point.row as u32, character: point.column as u32 }
    }
}

impl SourceRange {
    #[must_use]
    pub fn from_node(node: &tree_sitter::Node<'_>) -> Self {
        Self { start: node.start_position().into(), end: node.end_position().into() }
    }

    /// Returns an `lsp_types::Range` with surrounding quotes excluded
    /// (shrunk by 1 character on each side).
    #[must_use]
    pub const fn to_unquoted_range(&self) -> lsp_types::Range {
        lsp_types::Range {
            start: lsp_types::Position {
                line: self.start.line,
                character: self.start.character + 1,
            },
            end: lsp_types::Position {
                line: self.end.line,
                character: self.end.character.saturating_sub(1),
            },
        }
    }

    /// Checks if a position is within this range.
    #[must_use]
    pub const fn contains(&self, position: SourcePosition) -> bool {
        if position.line < self.start.line {
            return false;
        }
        if position.line == self.start.line && position.character < self.start.character {
            return false;
        }
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

    const fn pos(line: u32, character: u32) -> SourcePosition {
        SourcePosition { line, character }
    }

    const fn range(start_line: u32, start_char: u32, end_line: u32, end_char: u32) -> SourceRange {
        SourceRange { start: pos(start_line, start_char), end: pos(end_line, end_char) }
    }

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

    #[rstest]
    fn to_unquoted_range_shrinks_by_one() {
        let r = range(5, 10, 5, 20);
        let content = r.to_unquoted_range();
        assert_that!(content.start.line, eq(5));
        assert_that!(content.start.character, eq(11));
        assert_that!(content.end.line, eq(5));
        assert_that!(content.end.character, eq(19));
    }

    #[rstest]
    fn to_unquoted_range_saturates_at_zero() {
        let r = range(3, 0, 3, 5);
        let content = r.to_unquoted_range();
        assert_that!(content.start.character, eq(1));
        assert_that!(content.end.character, eq(4));
    }

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
