---
paths: "src/input/**"
---

# Input Module Rules

## Structure

- `source.rs` - SourceFile representation (JS/TS files)
- `translation.rs` - Translation file parsing (JSON)

## Translation File Parsing

### Key Extraction
The `extract_keys_from_node()` function handles:
- Nested objects with separator (e.g., `common.hello`)
- Arrays with index notation (e.g., `items[0]`)
- Mixed structures

Split into focused helpers:
- `extract_array_elements()` - Array node handling
- `extract_pair()` - Key-value pair handling

### Position Tracking
Track both key and value positions:
- `key_ranges` - For go-to-definition on keys
- `value_ranges` - For editing values

## Salsa Integration

`Translation` and `SourceFile` are Salsa tracked structs:
```rust
#[salsa::tracked]
pub struct Translation<'db> {
    #[id]
    pub file_path: String,
    pub language: String,
    pub keys: HashMap<String, String>,
    // ...
}
```

Changes automatically invalidate dependent queries.
