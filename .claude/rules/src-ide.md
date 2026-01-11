---
paths: "src/ide/**"
---

# IDE Module Rules

## Structure

- `backend.rs` - LSP Backend, state management, handler dispatch
- `handlers.rs` - LSP request/notification handlers
- `completion.rs` - Completion provider
- `hover.rs` - Hover information
- `diagnostics.rs` - Missing key diagnostics
- `code_actions.rs` - Quick fixes and refactorings
- `goto_definition.rs` - Go to definition in JSON files
- `references.rs` - Find references
- `virtual_text.rs` - Inline translation display

## Patterns

### Async Lock Ordering
Always acquire locks in consistent order to avoid deadlocks:
1. `config_manager`
2. `db`
3. `source_files` / `translations`

### Helper Extraction
Extract repeated patterns into helper methods:
- `get_diagnostic_config()` for diagnostic settings
- `reset_state()` for workspace reset
- `send_progress_begin/end()` for progress notifications

### Functional Style
Prefer iterator chains over loops:
```rust
// Good
translations.iter()
    .filter(|t| condition)
    .find_map(|t| t.keys(db).get(key).cloned())

// Avoid
for t in translations {
    if condition { ... }
}
```
