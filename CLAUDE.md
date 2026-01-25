# js-i18n-language-server

Rust LSP for JavaScript/TypeScript i18n (i18next, next-intl, etc.)

## Commands

Use `mise run <task>` for all commands. See `mise.toml` for available tasks.

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
