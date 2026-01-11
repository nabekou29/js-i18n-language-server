---
paths: "src/syntax/**"
---

# Syntax Module Rules

## Structure

- `analyzer/extractor.rs` - Extract translation function calls using tree-sitter queries
- `analyzer/scope.rs` - Scope management for translation functions
- `analyzer/query_loader.rs` - Load tree-sitter queries from .scm files
- `analyzer/types.rs` - Type definitions for analysis results

## Tree-sitter Queries

Query files are in `queries/`:
- `get_trans_fn.scm` - Match useTranslation, getFixedT patterns
- `call_trans_fn.scm` - Match t(), i18next.t() calls

### Query Capture Names
Defined in `CaptureName` enum:
- `@get_trans_fn_name` - Translation function variable name
- `@call_trans_fn_name` - Called function name
- `@trans_key` - Translation key string
- `@namespace`, `@key_prefix` - Scope context

## Position Conversion

Use `SourceRange::from_node()` for tree-sitter to LSP position conversion:
```rust
// Good
let range = SourceRange::from_node(&node);

// Avoid manual conversion
let range = SourceRange {
    start: SourcePosition { line: node.start_position().row as u32, ... },
    ...
};
```

## Supported Libraries

- i18next: `useTranslation`, `getFixedT`, `t()`
- next-intl: `useTranslations`, `getTranslations`
- Global functions: `i18next.t()`, `i18n.t()`
