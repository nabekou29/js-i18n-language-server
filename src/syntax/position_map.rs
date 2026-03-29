//! Position mapping for embedded-template languages.

use std::borrow::Cow;

use tower_lsp::lsp_types::{
    Position,
    Range,
};

/// Maps positions in a virtual document to positions in the original source file.
///
/// Used by embedded-template languages (e.g., Svelte) where JS/TS code is
/// extracted into a virtual document for tree-sitter parsing. The map allows
/// remapping ranges back to their original file coordinates.
#[derive(Debug, Default)]
pub struct PositionMap {
    entries: Vec<PositionMapEntry>,
}

#[derive(Debug)]
pub(crate) struct PositionMapEntry {
    pub virtual_line_start: u32,
    pub virtual_line_count: u32,
    pub original_line: u32,
    /// Column offset: `original_col` = `virtual_col` + `column_offset`.
    pub column_offset: i32,
}

impl PositionMap {
    /// Remap a `Range` from virtual document coordinates to original file coordinates.
    #[must_use]
    pub fn remap(&self, range: Range) -> Range {
        Range { start: self.remap_position(range.start), end: self.remap_position(range.end) }
    }

    fn remap_position(&self, pos: Position) -> Position {
        for entry in self.entries.iter().rev() {
            if pos.line >= entry.virtual_line_start
                && pos.line < entry.virtual_line_start + entry.virtual_line_count
            {
                let line_offset = pos.line - entry.virtual_line_start;
                #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
                return Position {
                    line: entry.original_line + line_offset,
                    character: (i64::from(pos.character) + i64::from(entry.column_offset)) as u32,
                };
            }
        }
        pos
    }

    pub(crate) fn push(&mut self, entry: PositionMapEntry) {
        self.entries.push(entry);
    }
}

/// Result of preprocessing a source file for analysis.
///
/// For non-embedded languages, `source` is the original text and `position_map` is `None`.
/// For embedded languages (e.g., Svelte), `source` is the extracted virtual document
/// and `position_map` maps virtual positions back to original file positions.
#[derive(Debug)]
pub struct SourcePreprocessed<'a> {
    pub source: Cow<'a, str>,
    pub position_map: Option<PositionMap>,
}
