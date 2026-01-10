# js-i18n-language-server

Rust LSP for JavaScript/TypeScript i18n (i18next, next-intl, etc.)

## Commands

```bash
cargo build          # Build
cargo test           # Run tests (265 tests)
cargo clippy         # Lint
cargo +nightly fmt   # Format
```

## Architecture

```
src/
  ide/         # LSP features (completion, hover, diagnostics, code_actions)
  syntax/      # tree-sitter parsing and analysis
  input/       # Source file and translation file handling
  config/      # Settings management
  indexer/     # Workspace indexing
```

## Key Technologies

- **tower-lsp**: LSP server framework
- **tree-sitter**: Incremental parsing (JS/TS/JSON)
- **salsa**: Incremental computation (caching)
- **jsonc-parser**: JSON with comments (CST manipulation)

## Conventions

- Rust 2024 edition with strict lints (see Cargo.toml)
- No `mod.rs` - use modern module style (`foo.rs` + `foo/`)
- Prefer functional style: `iter()`, `filter()`, `map()`, `find_map()`
