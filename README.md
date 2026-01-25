# js-i18n-language-server

[![CI](https://github.com/nabekou29/js-i18n-language-server/actions/workflows/ci.yml/badge.svg)](https://github.com/nabekou29/js-i18n-language-server/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/nabekou29/js-i18n-language-server/graph/badge.svg)](https://codecov.io/gh/nabekou29/js-i18n-language-server)

Language Server for JavaScript/TypeScript i18n libraries.

Provides IDE features (completion, hover, diagnostics, etc.) for translation keys in i18next, react-i18next, and next-intl projects.

## Installation

### npm

```bash
npm install -g js-i18n-language-server
```

### Cargo

```bash
cargo install --git https://github.com/nabekou29/js-i18n-language-server
```

### Binary

Download from [GitHub Releases](https://github.com/nabekou29/js-i18n-language-server/releases).

## Configuration

Create `.js-i18n.json` in your project root:

```json
{
  "translationFiles": {
    "filePattern": "**/locales/**/*.json"
  },
  "includePatterns": ["src/**/*.{ts,tsx}"],
  "excludePatterns": ["node_modules/**"]
}
```

## Documentation

- [Configuration Reference](./docs/configuration.md) - All configuration options
- [LSP Features](./docs/lsp-features.md) - Standard methods and custom commands
- [Supported Syntax](./docs/supported-syntax.md) - Recognized code patterns

## License

MIT
