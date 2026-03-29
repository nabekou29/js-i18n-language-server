//! Extract JavaScript/TypeScript regions from `.svelte` files.
//!
//! Svelte files mix HTML, JS, and CSS. This module extracts JS/TS code from
//! `<script>` blocks and template expressions, building a virtual document
//! that can be parsed by tree-sitter TypeScript.

use super::position_map::{
    PositionMap,
    PositionMapEntry,
};

/// Result of extracting JS/TS from a `.svelte` file.
#[derive(Debug)]
pub struct SvelteExtraction {
    /// Synthesized JS/TS source for tree-sitter parsing.
    pub virtual_doc: String,
    /// Maps virtual document positions back to original `.svelte` file positions.
    pub position_map: PositionMap,
}

/// Extract JS/TS regions from a Svelte source file.
#[must_use]
pub fn extract(source: &str) -> SvelteExtraction {
    let mut virtual_doc = String::new();
    let mut position_map = PositionMap::default();
    let mut virtual_line: u32 = 0;

    let lines: Vec<&str> = source.lines().collect();
    let mut in_script = false;
    let mut in_style = false;
    let mut script_start_line: u32 = 0;

    // Phase 1: Extract <script> blocks
    for (line_idx, line) in lines.iter().enumerate() {
        #[allow(clippy::cast_possible_truncation)]
        let line_num = line_idx as u32;
        let trimmed = line.trim();

        if !in_script && !in_style {
            if trimmed.starts_with("<script") && trimmed.contains('>') {
                in_script = true;
                // Script content starts on the next line (or same line after >)
                if let Some(after_tag) = trimmed.split_once('>').map(|(_, rest)| rest)
                    && !after_tag.is_empty()
                    && !after_tag.starts_with("</script")
                {
                    virtual_doc.push_str(after_tag);
                    virtual_doc.push('\n');
                    let tag_prefix_len = line.find('>').map_or(0, |i| i + 1);
                    position_map.push(PositionMapEntry {
                        virtual_line_start: virtual_line,
                        virtual_line_count: 1,
                        original_line: line_num,
                        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
                        column_offset: tag_prefix_len as i32,
                    });
                    virtual_line += 1;
                }
                script_start_line = line_num + 1;
                continue;
            }
            if trimmed.starts_with("<style") {
                in_style = true;
                continue;
            }
        }

        if in_style {
            if trimmed.starts_with("</style") {
                in_style = false;
            }
            continue;
        }

        if in_script {
            if trimmed.starts_with("</script") {
                in_script = false;
                continue;
            }
            virtual_doc.push_str(line);
            virtual_doc.push('\n');
            position_map.push(PositionMapEntry {
                virtual_line_start: virtual_line,
                virtual_line_count: 1,
                original_line: line_num,
                column_offset: 0,
            });
            virtual_line += 1;
        }
    }

    // Phase 2: Extract template expressions from non-script, non-style regions
    let _ = script_start_line; // suppress unused warning
    in_script = false;
    in_style = false;

    for (line_idx, line) in lines.iter().enumerate() {
        #[allow(clippy::cast_possible_truncation)]
        let line_num = line_idx as u32;
        let trimmed = line.trim();

        // Track script/style regions to skip
        if trimmed.starts_with("<script") && trimmed.contains('>') {
            in_script = true;
        }
        if in_script && trimmed.starts_with("</script") {
            in_script = false;
            continue;
        }
        if trimmed.starts_with("<style") {
            in_style = true;
        }
        if in_style && trimmed.starts_with("</style") {
            in_style = false;
            continue;
        }
        if in_script || in_style {
            continue;
        }

        // Scan for { ... } expressions in template
        extract_expressions_from_line(
            line,
            line_num,
            &mut virtual_doc,
            &mut position_map,
            &mut virtual_line,
        );
    }

    SvelteExtraction { virtual_doc, position_map }
}

/// Scan a template line for `{...}` expressions and extract them.
fn extract_expressions_from_line(
    line: &str,
    line_num: u32,
    virtual_doc: &mut String,
    position_map: &mut PositionMap,
    virtual_line: &mut u32,
) {
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars.get(i).copied() != Some('{') {
            i += 1;
            continue;
        }

        let content_start = i + 1;
        let rest = &line[content_start..];
        let trimmed_rest = rest.trim_start();

        // Skip Svelte control flow: {#if}, {:else}, {/if}
        if trimmed_rest.starts_with('#')
            || trimmed_rest.starts_with(':')
            || trimmed_rest.starts_with('/')
        {
            i = find_closing_brace(&chars, i).map_or(i + 1, |close| close + 1);
            continue;
        }

        // Handle @-directives
        if trimmed_rest.starts_with('@') {
            i = handle_at_directive(
                line,
                &chars,
                i,
                line_num,
                virtual_doc,
                position_map,
                virtual_line,
            );
            continue;
        }

        // Regular expression: extract content between { and }
        if let Some(close) = find_closing_brace(&chars, i) {
            push_expression(
                &line[content_start..close],
                line_num,
                content_start,
                virtual_doc,
                position_map,
                virtual_line,
            );
            i = close + 1;
        } else {
            i += 1;
        }
    }
}

/// Handle `{@const ...}`, `{@html ...}`, `{@debug}` directives.
/// Returns the next index to continue scanning from.
fn handle_at_directive(
    line: &str,
    chars: &[char],
    open_pos: usize,
    line_num: u32,
    virtual_doc: &mut String,
    position_map: &mut PositionMap,
    virtual_line: &mut u32,
) -> usize {
    let content_start = open_pos + 1;
    let Some(close) = find_closing_brace(chars, open_pos) else {
        return open_pos + 1;
    };

    let inner_trimmed = line[content_start..close].trim();

    if inner_trimmed.starts_with("@debug") {
        return close + 1;
    }

    // {@const x = expr} → extract "x = expr"
    // {@html expr} → extract "expr"
    let expr = if let Some(after) = inner_trimmed.strip_prefix("@const ") {
        after.trim()
    } else if let Some(after) = inner_trimmed.strip_prefix("@html ") {
        after.trim()
    } else {
        return close + 1;
    };

    if !expr.is_empty() {
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
        let col_offset = line.find(expr).map_or(open_pos as i32 + 1, |p| p as i32);
        push_expression_raw(expr, line_num, col_offset, virtual_doc, position_map, virtual_line);
    }
    close + 1
}

/// Push a trimmed expression to the virtual document.
fn push_expression(
    raw_expr: &str,
    line_num: u32,
    content_start: usize,
    virtual_doc: &mut String,
    position_map: &mut PositionMap,
    virtual_line: &mut u32,
) {
    let trimmed = raw_expr.trim();
    if trimmed.is_empty() {
        return;
    }
    let whitespace_prefix = raw_expr.chars().take_while(|c| c.is_whitespace()).count();
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    let col_offset = (content_start + whitespace_prefix) as i32;
    push_expression_raw(trimmed, line_num, col_offset, virtual_doc, position_map, virtual_line);
}

fn push_expression_raw(
    expr: &str,
    line_num: u32,
    column_offset: i32,
    virtual_doc: &mut String,
    position_map: &mut PositionMap,
    virtual_line: &mut u32,
) {
    virtual_doc.push_str(expr);
    virtual_doc.push('\n');
    position_map.push(PositionMapEntry {
        virtual_line_start: *virtual_line,
        virtual_line_count: 1,
        original_line: line_num,
        column_offset,
    });
    *virtual_line += 1;
}

/// Find the matching closing brace, handling nested braces and string literals.
fn find_closing_brace(chars: &[char], open_pos: usize) -> Option<usize> {
    let mut depth: u32 = 1;
    let mut i = open_pos + 1;
    let mut in_string = false;
    let mut string_char = ' ';

    while let Some(&c) = chars.get(i) {
        if in_string {
            if c == string_char && chars.get(i.wrapping_sub(1)).copied() != Some('\\') {
                in_string = false;
            }
            i += 1;
            continue;
        }

        if c == '\'' || c == '"' || c == '`' {
            in_string = true;
            string_char = c;
            i += 1;
            continue;
        }

        if c == '{' {
            depth += 1;
        } else if c == '}' {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::expect_used, clippy::panic)]
mod tests {
    use googletest::prelude::*;
    use rstest::*;
    use tower_lsp::lsp_types::{
        Position,
        Range,
    };

    use super::*;

    // --- Script block extraction ---

    #[rstest]
    fn extract_script_block_basic() {
        let svelte = "\
<script>
  import { _ } from 'svelte-i18n';
  const msg = $_('hello');
</script>
<p>{$_('world')}</p>";
        let result = extract(svelte);

        assert_that!(result.virtual_doc, contains_substring("$_('hello')"));
        assert_that!(result.virtual_doc, contains_substring("import { _ }"));
    }

    #[rstest]
    fn extract_script_lang_ts() {
        let svelte = "<script lang=\"ts\">\n  const x: string = $_('typed');\n</script>";
        let result = extract(svelte);

        assert_that!(result.virtual_doc, contains_substring("$_('typed')"));
    }

    #[rstest]
    fn extract_multiple_script_blocks() {
        let svelte = "\
<script module>
  export const preload = true;
</script>
<script>
  import { _ } from 'svelte-i18n';
  $_('hello');
</script>";
        let result = extract(svelte);

        assert_that!(result.virtual_doc, contains_substring("export const preload"));
        assert_that!(result.virtual_doc, contains_substring("$_('hello')"));
    }

    #[rstest]
    fn extract_no_script_block() {
        let svelte = "<p>{$_('only_template')}</p>";
        let result = extract(svelte);

        assert_that!(result.virtual_doc, contains_substring("$_('only_template')"));
    }

    #[rstest]
    fn extract_script_position_mapping() {
        let svelte = "<script>\n  $_('key')\n</script>";
        let result = extract(svelte);

        // Line 1 in original (0-indexed) = "$_('key')" is on virtual line 0
        // The position map should remap virtual line 0 → original line 1
        let virtual_range = Range {
            start: Position { line: 0, character: 2 },
            end: Position { line: 0, character: 12 },
        };
        let remapped = result.position_map.remap(virtual_range);
        assert_that!(remapped.start.line, eq(1));
        assert_that!(remapped.start.character, eq(2));
    }

    // --- Template expression extraction ---

    #[rstest]
    fn extract_template_basic() {
        let svelte = "<p>{$_('greeting')}</p>";
        let result = extract(svelte);

        assert_that!(result.virtual_doc, contains_substring("$_('greeting')"));
    }

    #[rstest]
    fn extract_template_with_values() {
        let svelte = "<p>{$_('welcome', { values: { name: 'World' } })}</p>";
        let result = extract(svelte);

        assert_that!(result.virtual_doc, contains_substring("$_('welcome'"));
    }

    #[rstest]
    fn extract_template_ternary() {
        let svelte = "<p>{condition ? $_('a') : $_('b')}</p>";
        let result = extract(svelte);

        assert_that!(result.virtual_doc, contains_substring("$_('a')"));
        assert_that!(result.virtual_doc, contains_substring("$_('b')"));
    }

    #[rstest]
    fn extract_template_event_handler() {
        let svelte = "<button onclick={() => alert($_('msg'))}>Click</button>";
        let result = extract(svelte);

        assert_that!(result.virtual_doc, contains_substring("$_('msg')"));
    }

    #[rstest]
    fn extract_template_attribute_value() {
        let svelte = "<img title={$_('tooltip')} />";
        let result = extract(svelte);

        assert_that!(result.virtual_doc, contains_substring("$_('tooltip')"));
    }

    #[rstest]
    fn extract_template_skips_control_flow() {
        let svelte = "{#if condition}\n  <p>{$_('key')}</p>\n{/if}";
        let result = extract(svelte);

        assert_that!(result.virtual_doc, contains_substring("$_('key')"));
        // Control flow tokens should not be in virtual doc
        assert_that!(result.virtual_doc, not(contains_substring("#if")));
        assert_that!(result.virtual_doc, not(contains_substring("/if")));
    }

    #[rstest]
    fn extract_template_at_const() {
        let svelte = "{@const label = $_('key')}";
        let result = extract(svelte);

        assert_that!(result.virtual_doc, contains_substring("$_('key')"));
    }

    #[rstest]
    fn extract_template_at_html() {
        let svelte = "{@html $_('richContent')}";
        let result = extract(svelte);

        assert_that!(result.virtual_doc, contains_substring("$_('richContent')"));
    }

    #[rstest]
    fn extract_template_skips_at_debug() {
        let svelte = "{@debug}\n<p>{$_('key')}</p>";
        let result = extract(svelte);

        assert_that!(result.virtual_doc, contains_substring("$_('key')"));
        assert_that!(result.virtual_doc, not(contains_substring("@debug")));
    }

    #[rstest]
    fn extract_template_non_i18n_expression() {
        let svelte = "<p>{count + 1}</p><p>{$_('key')}</p>";
        let result = extract(svelte);

        // Both are extracted; tree-sitter queries will filter
        assert_that!(result.virtual_doc, contains_substring("$_('key')"));
        assert_that!(result.virtual_doc, contains_substring("count + 1"));
    }

    #[rstest]
    fn extract_template_multiple_expressions() {
        let svelte = "\
<h1>{$_('title')}</h1>
<p>{$t('description')}</p>
<span>{$format('note')}</span>";
        let result = extract(svelte);

        assert_that!(result.virtual_doc, contains_substring("$_('title')"));
        assert_that!(result.virtual_doc, contains_substring("$t('description')"));
        assert_that!(result.virtual_doc, contains_substring("$format('note')"));
    }

    #[rstest]
    fn extract_template_position_mapping() {
        let svelte = "<p>{$_('key')}</p>";
        let result = extract(svelte);

        // $_('key') starts at column 4 in original (after "<p>{")
        // In virtual doc it starts at column 0
        let virtual_range = Range {
            start: Position { line: 0, character: 0 },
            end: Position { line: 0, character: 10 },
        };
        let remapped = result.position_map.remap(virtual_range);
        assert_that!(remapped.start.line, eq(0));
        assert_that!(remapped.start.character, eq(4));
    }

    #[rstest]
    fn extract_ignores_style_block() {
        let svelte = "\
<style>
  .foo { color: red; }
</style>
<p>{$_('key')}</p>";
        let result = extract(svelte);

        assert_that!(result.virtual_doc, contains_substring("$_('key')"));
        assert_that!(result.virtual_doc, not(contains_substring("color")));
    }

    #[rstest]
    fn extract_script_and_template_combined() {
        let svelte = "\
<script>
  import { _ } from 'svelte-i18n';
  const msg = $_('script_key');
</script>
<p>{$_('template_key')}</p>";
        let result = extract(svelte);

        assert_that!(result.virtual_doc, contains_substring("$_('script_key')"));
        assert_that!(result.virtual_doc, contains_substring("$_('template_key')"));
    }
}
