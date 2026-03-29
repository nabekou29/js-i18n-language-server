//! Extract JavaScript/TypeScript regions from `.vue` files.
//!
//! Vue SFC files mix HTML template, JS/TS script, and CSS. This module extracts
//! JS/TS code from `<script>` blocks and template expressions, building a virtual
//! document that can be parsed by tree-sitter TypeScript.

use super::position_map::{
    PositionMap,
    PositionMapEntry,
};

/// Result of extracting JS/TS from a `.vue` file.
#[derive(Debug)]
pub struct VueExtraction {
    /// Synthesized JS/TS source for tree-sitter parsing.
    pub virtual_doc: String,
    /// Maps virtual document positions back to original `.vue` file positions.
    pub position_map: PositionMap,
}

/// Extract JS/TS regions from a Vue SFC source file.
#[must_use]
pub fn extract(source: &str) -> VueExtraction {
    let mut virtual_doc = String::new();
    let mut position_map = PositionMap::default();
    let mut virtual_line: u32 = 0;

    let lines: Vec<&str> = source.lines().collect();

    // Phase 1: Extract <script> and <script setup> blocks
    extract_script_blocks(&lines, &mut virtual_doc, &mut position_map, &mut virtual_line);

    // Phase 2: Extract template expressions from non-script, non-style, non-i18n regions
    extract_template_regions(&lines, &mut virtual_doc, &mut position_map, &mut virtual_line);

    VueExtraction { virtual_doc, position_map }
}

/// Phase 1: Extract content from `<script>` and `<script setup>` blocks.
fn extract_script_blocks(
    lines: &[&str],
    virtual_doc: &mut String,
    position_map: &mut PositionMap,
    virtual_line: &mut u32,
) {
    let mut in_script = false;
    let mut in_style = false;
    let mut in_i18n = false;

    for (line_idx, line) in lines.iter().enumerate() {
        #[allow(clippy::cast_possible_truncation)]
        let line_num = line_idx as u32;
        let trimmed = line.trim();

        if !in_script && !in_style && !in_i18n {
            if is_script_open_tag(trimmed) {
                in_script = true;
                if let Some(after_tag) = trimmed.split_once('>').map(|(_, rest)| rest)
                    && !after_tag.is_empty()
                    && !after_tag.starts_with("</script")
                {
                    let tag_prefix_len = line.find('>').map_or(0, |i| i + 1);
                    push_line(
                        after_tag,
                        line_num,
                        tag_prefix_len,
                        virtual_doc,
                        position_map,
                        virtual_line,
                    );
                }
                continue;
            }
            if trimmed.starts_with("<style") {
                in_style = true;
                continue;
            }
            if is_i18n_block_open(trimmed) {
                in_i18n = true;
                continue;
            }
        }

        if in_style {
            if trimmed.starts_with("</style") {
                in_style = false;
            }
            continue;
        }

        if in_i18n {
            if is_i18n_block_close(trimmed) {
                in_i18n = false;
            }
            continue;
        }

        if in_script {
            if trimmed.starts_with("</script") {
                in_script = false;
                continue;
            }
            push_line(line, line_num, 0, virtual_doc, position_map, virtual_line);
        }
    }
}

/// Phase 2: Extract template expressions from markup regions.
fn extract_template_regions(
    lines: &[&str],
    virtual_doc: &mut String,
    position_map: &mut PositionMap,
    virtual_line: &mut u32,
) {
    let mut in_script = false;
    let mut in_style = false;
    let mut in_i18n = false;

    for (line_idx, line) in lines.iter().enumerate() {
        #[allow(clippy::cast_possible_truncation)]
        let line_num = line_idx as u32;
        let trimmed = line.trim();

        // Track regions to skip
        if is_script_open_tag(trimmed) {
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
        if is_i18n_block_open(trimmed) {
            in_i18n = true;
        }
        if in_i18n && is_i18n_block_close(trimmed) {
            in_i18n = false;
            continue;
        }
        if in_script || in_style || in_i18n {
            continue;
        }

        // Extract from template line
        extract_template_line(line, line_num, virtual_doc, position_map, virtual_line);
    }
}

/// Extract expressions from a single template line.
///
/// Handles:
/// - `{{ expr }}` mustache interpolation
/// - `:attr="expr"` / `v-bind:attr="expr"` attribute bindings
/// - `v-if="expr"`, `v-show="expr"`, `v-for="... in expr"`, `@event="expr"` directives
/// - `<i18n-t keypath="key">` / `<I18nT keypath="key">` / `<i18n path="key">`
/// - `v-t="'key'"` / `v-t="{ path: 'key' }"` directive
fn extract_template_line(
    line: &str,
    line_num: u32,
    virtual_doc: &mut String,
    position_map: &mut PositionMap,
    virtual_line: &mut u32,
) {
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Mustache interpolation: {{ expr }}
        if chars.get(i).copied() == Some('{')
            && chars.get(i + 1).copied() == Some('{')
            && let Some(close) = find_mustache_close(&chars, i + 2)
        {
            let byte_start = char_offset_to_byte(line, i + 2);
            let byte_end = char_offset_to_byte(line, close);
            let expr = line[byte_start..byte_end].trim();
            if !expr.is_empty() {
                let whitespace_prefix =
                    line[byte_start..byte_end].chars().take_while(|c| c.is_whitespace()).count();
                #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
                let col_offset = (byte_start + whitespace_prefix) as i32;
                push_expression(
                    expr,
                    line_num,
                    col_offset,
                    virtual_doc,
                    position_map,
                    virtual_line,
                );
            }
            i = close + 2; // skip }}
            continue;
        }

        // HTML tag attributes: look for directive/binding attributes
        if chars.get(i).copied() == Some('<') && chars.get(i + 1).is_some_and(|c| c.is_alphabetic())
        {
            let tag_end = find_tag_end(&chars, i);
            let byte_start = char_offset_to_byte(line, i);
            let byte_end = char_offset_to_byte(line, tag_end.unwrap_or(chars.len()));
            let tag_content = &line[byte_start..byte_end];

            extract_tag_attributes(
                tag_content,
                byte_start,
                line_num,
                virtual_doc,
                position_map,
                virtual_line,
            );

            i = tag_end.unwrap_or(chars.len());
            continue;
        }

        i += 1;
    }
}

/// Extract directive/binding attributes from an HTML tag.
fn extract_tag_attributes(
    tag_content: &str,
    tag_byte_offset: usize,
    line_num: u32,
    virtual_doc: &mut String,
    position_map: &mut PositionMap,
    virtual_line: &mut u32,
) {
    // Handle <i18n-t keypath="key"> / <I18nT keypath="key"> / <i18n path="key">
    extract_i18n_component(
        tag_content,
        tag_byte_offset,
        line_num,
        virtual_doc,
        position_map,
        virtual_line,
    );

    // Handle v-t directive
    extract_v_t_directive(
        tag_content,
        tag_byte_offset,
        line_num,
        virtual_doc,
        position_map,
        virtual_line,
    );

    // Handle Vue directive/binding expressions: :attr="expr", v-if="expr", @event="expr"
    let mut pos = 0;
    let bytes = tag_content.as_bytes();
    while pos < bytes.len() {
        // Look for directive attributes
        let is_directive = matches!(bytes.get(pos), Some(b':' | b'@'))
            || (bytes.get(pos) == Some(&b'v')
                && bytes.get(pos + 1) == Some(&b'-')
                && !tag_content[pos..].starts_with("v-t=")
                && !tag_content[pos..].starts_with("v-t "));

        if is_directive {
            // Find ="..."
            if let Some(eq_pos) = tag_content[pos..].find('=') {
                let abs_eq = pos + eq_pos;
                if bytes.get(abs_eq + 1) == Some(&b'"')
                    && let Some(close_quote) = tag_content[abs_eq + 2..].find('"')
                {
                    let expr_start = abs_eq + 2;
                    let expr_end = abs_eq + 2 + close_quote;
                    let expr = &tag_content[expr_start..expr_end];
                    if !expr.is_empty() {
                        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
                        let col_offset = (tag_byte_offset + expr_start) as i32;
                        push_expression(
                            expr,
                            line_num,
                            col_offset,
                            virtual_doc,
                            position_map,
                            virtual_line,
                        );
                    }
                    pos = expr_end + 1;
                    continue;
                }
            }
        }
        pos += 1;
    }
}

/// Extract `<i18n-t keypath="key">`, `<I18nT keypath="key">`, `<i18n path="key">` components.
/// Synthesizes `$t('key')` calls for detected keys.
fn extract_i18n_component(
    tag_content: &str,
    tag_byte_offset: usize,
    line_num: u32,
    virtual_doc: &mut String,
    position_map: &mut PositionMap,
    virtual_line: &mut u32,
) {
    let tag_name = tag_content.split_whitespace().next().unwrap_or("").trim_start_matches('<');

    let attr_name = match tag_name {
        "i18n-t" | "I18nT" => "keypath",
        "i18n" => "path",
        _ => return,
    };

    let search = format!("{attr_name}=\"");
    if let Some(attr_pos) = tag_content.find(&search) {
        let value_start = attr_pos + search.len();
        if let Some(close_quote) = tag_content[value_start..].find('"') {
            let key = &tag_content[value_start..value_start + close_quote];
            if !key.is_empty() {
                // Synthesize $t('key') call
                let synthetic = format!("$t('{key}')");
                #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
                let col_offset = (tag_byte_offset + value_start) as i32;
                push_expression(
                    &synthetic,
                    line_num,
                    col_offset,
                    virtual_doc,
                    position_map,
                    virtual_line,
                );
            }
        }
    }
}

/// Extract `v-t="'key'"` or `v-t="{ path: 'key' }"` directive.
/// Synthesizes `$t('key')` calls for detected keys.
fn extract_v_t_directive(
    tag_content: &str,
    tag_byte_offset: usize,
    line_num: u32,
    virtual_doc: &mut String,
    position_map: &mut PositionMap,
    virtual_line: &mut u32,
) {
    // Match v-t="..." attribute
    let search = "v-t=\"";
    let Some(vt_pos) = tag_content.find(search) else { return };
    let value_start = vt_pos + search.len();
    let Some(close_quote) = tag_content[value_start..].find('"') else { return };
    let value = &tag_content[value_start..value_start + close_quote];

    let trimmed = value.trim();
    if trimmed.is_empty() {
        return;
    }

    // String syntax: v-t="'key'"
    if let Some(key) = extract_string_literal(trimmed) {
        let synthetic = format!("$t('{key}')");
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
        let col_offset = (tag_byte_offset + value_start) as i32;
        push_expression(&synthetic, line_num, col_offset, virtual_doc, position_map, virtual_line);
        return;
    }

    // Object syntax: v-t="{ path: 'key' }"
    if trimmed.starts_with('{')
        && let Some(key) = extract_object_path_value(trimmed)
    {
        let synthetic = format!("$t('{key}')");
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
        let col_offset = (tag_byte_offset + value_start) as i32;
        push_expression(&synthetic, line_num, col_offset, virtual_doc, position_map, virtual_line);
    }
}

/// Extract a string literal value from `'key'` or `"key"`.
fn extract_string_literal(s: &str) -> Option<&str> {
    let s = s.trim();
    if (s.starts_with('\'') && s.ends_with('\'')) || (s.starts_with('"') && s.ends_with('"')) {
        Some(&s[1..s.len() - 1])
    } else {
        None
    }
}

/// Extract the `path` property value from an object literal like `{ path: 'key' }`.
fn extract_object_path_value(s: &str) -> Option<&str> {
    // Simple parsing for `path: 'value'` or `path: "value"` within the object
    let path_patterns = ["path:", "path :"];
    for pattern in path_patterns {
        if let Some(idx) = s.find(pattern) {
            let after = s[idx + pattern.len()..].trim();
            return extract_string_literal(after.split([',', '}']).next().unwrap_or("").trim());
        }
    }
    None
}

/// Check if a trimmed line is a `<script>` or `<script setup>` opening tag.
fn is_script_open_tag(trimmed: &str) -> bool {
    trimmed.starts_with("<script") && trimmed.contains('>')
}

/// Check if a trimmed line opens an `<i18n>` custom block (not `<i18n-t>` component).
fn is_i18n_block_open(trimmed: &str) -> bool {
    if !trimmed.starts_with("<i18n") {
        return false;
    }
    // <i18n>, <i18n , <i18n\t — but NOT <i18n-t>, <i18n-d>, <i18n-n>
    let after = &trimmed[5..];
    after.is_empty() || after.starts_with('>') || after.starts_with(' ') || after.starts_with('/')
}

/// Check if a trimmed line closes an `<i18n>` custom block.
fn is_i18n_block_close(trimmed: &str) -> bool {
    trimmed.starts_with("</i18n>") || trimmed.starts_with("</i18n ")
}

fn push_line(
    content: &str,
    line_num: u32,
    byte_offset: usize,
    virtual_doc: &mut String,
    position_map: &mut PositionMap,
    virtual_line: &mut u32,
) {
    virtual_doc.push_str(content);
    virtual_doc.push('\n');
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    let column_offset = byte_offset as i32;
    position_map.push(PositionMapEntry {
        virtual_line_start: *virtual_line,
        virtual_line_count: 1,
        original_line: line_num,
        column_offset,
    });
    *virtual_line += 1;
}

fn push_expression(
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

/// Find `}}` closing mustache, handling nested braces and strings.
fn find_mustache_close(chars: &[char], start: usize) -> Option<usize> {
    let mut i = start;
    let mut in_string = false;
    let mut string_char = ' ';

    while let Some(&c) = chars.get(i) {
        if i + 1 >= chars.len() {
            break;
        }

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

        if c == '}' && chars.get(i + 1).copied() == Some('}') {
            return Some(i);
        }

        i += 1;
    }
    None
}

/// Find the end of an HTML tag (the `>` character), handling quoted attributes.
fn find_tag_end(chars: &[char], start: usize) -> Option<usize> {
    let mut i = start + 1;
    let mut in_quote = false;
    let mut quote_char = ' ';

    while let Some(&c) = chars.get(i) {
        if in_quote {
            if c == quote_char {
                in_quote = false;
            }
            i += 1;
            continue;
        }

        if c == '"' || c == '\'' {
            in_quote = true;
            quote_char = c;
            i += 1;
            continue;
        }

        if c == '>' {
            return Some(i + 1);
        }

        i += 1;
    }
    None
}

/// Convert a character offset to a byte offset.
fn char_offset_to_byte(s: &str, char_offset: usize) -> usize {
    s.char_indices().nth(char_offset).map_or(s.len(), |(byte_idx, _)| byte_idx)
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
        let vue = "\
<script>
import { useI18n } from 'vue-i18n'
const { t } = useI18n()
t('hello')
</script>
<template><p>{{ $t('world') }}</p></template>";
        let result = extract(vue);

        assert_that!(result.virtual_doc, contains_substring("t('hello')"));
        assert_that!(result.virtual_doc, contains_substring("useI18n"));
    }

    #[rstest]
    fn extract_script_setup() {
        let vue = "<script setup lang=\"ts\">\nconst { t } = useI18n()\nt('typed')\n</script>";
        let result = extract(vue);

        assert_that!(result.virtual_doc, contains_substring("t('typed')"));
    }

    #[rstest]
    fn extract_both_script_blocks() {
        let vue = "\
<script>
export default { name: 'MyComponent' }
</script>
<script setup>
const { t } = useI18n()
t('hello')
</script>";
        let result = extract(vue);

        assert_that!(result.virtual_doc, contains_substring("export default"));
        assert_that!(result.virtual_doc, contains_substring("t('hello')"));
    }

    #[rstest]
    fn extract_no_script_block() {
        let vue = "<template><p>{{ $t('only_template') }}</p></template>";
        let result = extract(vue);

        assert_that!(result.virtual_doc, contains_substring("$t('only_template')"));
    }

    #[rstest]
    fn extract_script_position_mapping() {
        let vue = "<script>\n  t('key')\n</script>";
        let result = extract(vue);

        let virtual_range = Range {
            start: Position { line: 0, character: 2 },
            end: Position { line: 0, character: 10 },
        };
        let remapped = result.position_map.remap(virtual_range);
        assert_that!(remapped.start.line, eq(1));
        assert_that!(remapped.start.character, eq(2));
    }

    // --- Template expression extraction ---

    #[rstest]
    fn extract_mustache_basic() {
        let vue = "<template><p>{{ $t('greeting') }}</p></template>";
        let result = extract(vue);

        assert_that!(result.virtual_doc, contains_substring("$t('greeting')"));
    }

    #[rstest]
    fn extract_mustache_with_values() {
        let vue = "<template><p>{{ $t('welcome', { name: 'World' }) }}</p></template>";
        let result = extract(vue);

        assert_that!(result.virtual_doc, contains_substring("$t('welcome'"));
    }

    #[rstest]
    fn extract_mustache_ternary() {
        let vue = "<template><p>{{ condition ? $t('a') : $t('b') }}</p></template>";
        let result = extract(vue);

        assert_that!(result.virtual_doc, contains_substring("$t('a')"));
        assert_that!(result.virtual_doc, contains_substring("$t('b')"));
    }

    // --- Directive/binding extraction ---

    #[rstest]
    fn extract_v_bind_shorthand() {
        let vue = "<template><input :placeholder=\"$t('form.placeholder')\" /></template>";
        let result = extract(vue);

        assert_that!(result.virtual_doc, contains_substring("$t('form.placeholder')"));
    }

    #[rstest]
    fn extract_v_if_directive() {
        let vue = "<template><span v-if=\"$te('optional')\">text</span></template>";
        let result = extract(vue);

        assert_that!(result.virtual_doc, contains_substring("$te('optional')"));
    }

    #[rstest]
    fn extract_v_show_directive() {
        let vue = "<template><span v-show=\"$te('visible')\">text</span></template>";
        let result = extract(vue);

        assert_that!(result.virtual_doc, contains_substring("$te('visible')"));
    }

    #[rstest]
    fn extract_event_handler() {
        let vue = "<template><button @click=\"alert($t('msg'))\">Click</button></template>";
        let result = extract(vue);

        assert_that!(result.virtual_doc, contains_substring("alert($t('msg'))"));
    }

    // --- i18n-t component extraction ---

    #[rstest]
    fn extract_i18n_t_keypath() {
        let vue = "<template><i18n-t keypath=\"terms\" tag=\"p\"></i18n-t></template>";
        let result = extract(vue);

        assert_that!(result.virtual_doc, contains_substring("$t('terms')"));
    }

    #[rstest]
    fn extract_i18n_t_pascal_case() {
        let vue = "<template><I18nT keypath=\"terms\" tag=\"p\"></I18nT></template>";
        let result = extract(vue);

        assert_that!(result.virtual_doc, contains_substring("$t('terms')"));
    }

    #[rstest]
    fn extract_i18n_v8_path() {
        let vue = "<template><i18n path=\"terms\" tag=\"p\"></i18n></template>";
        let result = extract(vue);

        assert_that!(result.virtual_doc, contains_substring("$t('terms')"));
    }

    // --- v-t directive extraction ---

    #[rstest]
    fn extract_v_t_string_syntax() {
        let vue = "<template><p v-t=\"'message.hello'\"></p></template>";
        let result = extract(vue);

        assert_that!(result.virtual_doc, contains_substring("$t('message.hello')"));
    }

    #[rstest]
    fn extract_v_t_object_syntax() {
        let vue = "<template><p v-t=\"{ path: 'message.hello', args: { name: userName } }\"></p></template>";
        let result = extract(vue);

        assert_that!(result.virtual_doc, contains_substring("$t('message.hello')"));
    }

    // --- Skipping non-template regions ---

    #[rstest]
    fn extract_ignores_style_block() {
        let vue = "\
<style>
  .foo { color: red; }
</style>
<template><p>{{ $t('key') }}</p></template>";
        let result = extract(vue);

        assert_that!(result.virtual_doc, contains_substring("$t('key')"));
        assert_that!(result.virtual_doc, not(contains_substring("color")));
    }

    #[rstest]
    fn extract_ignores_i18n_block() {
        let vue = "\
<i18n>
{ \"en\": { \"title\": \"Hello\" } }
</i18n>
<template><p>{{ $t('title') }}</p></template>";
        let result = extract(vue);

        assert_that!(result.virtual_doc, contains_substring("$t('title')"));
        assert_that!(result.virtual_doc, not(contains_substring("Hello")));
    }

    // --- Combined extraction ---

    #[rstest]
    fn extract_script_and_template_combined() {
        let vue = "\
<script setup>
import { useI18n } from 'vue-i18n'
const { t } = useI18n()
const msg = t('script_key')
</script>
<template><p>{{ $t('template_key') }}</p></template>";
        let result = extract(vue);

        assert_that!(result.virtual_doc, contains_substring("t('script_key')"));
        assert_that!(result.virtual_doc, contains_substring("$t('template_key')"));
    }

    #[rstest]
    fn extract_full_vue_component() {
        let vue = "\
<script setup lang=\"ts\">
import { useI18n } from 'vue-i18n'
const { t, te } = useI18n()
</script>

<template>
  <div>
    <h1>{{ $t('page.title') }}</h1>
    <input :placeholder=\"$t('form.placeholder')\" />
    <span v-if=\"$te('optional')\">{{ $t('optional') }}</span>
    <i18n-t keypath=\"terms\" tag=\"p\">
      <template #link><a href=\"/tos\">{{ $t('tos') }}</a></template>
    </i18n-t>
    <p v-t=\"'message.hello'\"></p>
  </div>
</template>

<style scoped>
.title { color: red; }
</style>";
        let result = extract(vue);

        assert_that!(result.virtual_doc, contains_substring("useI18n"));
        assert_that!(result.virtual_doc, contains_substring("$t('page.title')"));
        assert_that!(result.virtual_doc, contains_substring("$t('form.placeholder')"));
        assert_that!(result.virtual_doc, contains_substring("$te('optional')"));
        assert_that!(result.virtual_doc, contains_substring("$t('terms')"));
        assert_that!(result.virtual_doc, contains_substring("$t('message.hello')"));
        assert_that!(result.virtual_doc, not(contains_substring("color")));
    }
}
