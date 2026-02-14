<div align="center">
    <img src="docs/images/icon.png" width="128" height="128" alt="js-i18n-language-server">
    <h1>js-i18n-language-server</h1>
</div>

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE) [![CI](https://github.com/nabekou29/js-i18n-language-server/actions/workflows/ci.yml/badge.svg)](https://github.com/nabekou29/js-i18n-language-server/actions/workflows/ci.yml) [![codecov](https://codecov.io/gh/nabekou29/js-i18n-language-server/graph/badge.svg)](https://codecov.io/gh/nabekou29/js-i18n-language-server) [![VS Code](https://img.shields.io/badge/VS%20Code-007ACC?logo=visualstudiocode&logoColor=white)](https://github.com/nabekou29/vscode-js-i18n) [![Neovim](https://img.shields.io/badge/Neovim-57A143?logo=neovim&logoColor=white)](https://github.com/nabekou29/js-i18n.nvim)

Language Server for JavaScript/TypeScript i18n libraries.

Provides IDE features (completion, hover, diagnostics, etc.) for translation keys in i18next, react-i18next, and next-intl projects.

## Editor Extensions

- <img src="https://skillicons.dev/icons?i=vscode" align="center" width="32px"/> **VS Code**: [nabekou29/vscode-js-i18n](https://github.com/nabekou29/vscode-js-i18n)
- <img src="https://skillicons.dev/icons?i=neovim" align="center" width="32px"/> **Neovim**: [nabekou29/js-i18n.nvim](https://github.com/nabekou29/js-i18n.nvim)

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
