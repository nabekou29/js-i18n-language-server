---
paths: "src/config/**"
---

# Config Module Rules

## Structure

- `types.rs` - `I18nSettings` struct with all configuration fields
- `loader.rs` - Load `.js-i18n.json` files
- `manager.rs` - `ConfigManager` for workspace config state
- `matcher.rs` - `FileMatcher` for glob pattern matching

## Adding New Settings

1. Add field to `I18nSettings` in `types.rs` with `#[serde(default)]`
2. Use `Option<T>` for optional settings with `None` as default
3. Document with `///` comments for IDE hover support

```rust
/// Description of the setting
#[serde(default)]
pub new_setting: Option<String>,
```

## FileMatcher

Uses `globset` for efficient multi-pattern matching:

```rust
let matcher = FileMatcher::new(&["**/*.json", "!node_modules/**"])?;
if matcher.is_match(path) { ... }
```
