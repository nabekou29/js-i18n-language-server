
## [0.2.0] - 2026-02-01

### Bug Fixes

- Use --prepend for changelog to preserve manual entries
- Prevent duplicate translation entries during workspace indexing
- Replace blocking_lock with async lock to prevent deadlock

### Documentation

- Add i18n.getKeyAtPosition to LSP features

### Features

- Add i18n.getKeyAtPosition command
- Add i18n.getCurrentLanguage command and reorder commands by category
- Rewrite editTranslation to accept value and write directly
- Add i18n.getTranslationValue command
- Add language fallback resolution to getCurrentLanguage
- Add i18n/decorationsChanged custom notification
- Restore edit translation code actions with experimental capability gate
- Add maxWidth for display-width-based truncation
- Make maxWidth required with default 32, maxLength optional

### Refactor

- Rely on didChange for state sync after applyEdit
- Extract helpers and eliminate duplicated code
- Extract TruncateOption enum for truncation parameters

# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-01-26

Initial release of js-i18n-language-server.

### Features

- LSP support for JavaScript/TypeScript i18n libraries (i18next, next-intl, etc.)
- Completion for translation keys with multi-language preview
- Hover to display translations across all languages
- Go to Definition for translation keys (source → JSON, JSON → source)
- Diagnostics for missing translation keys
- Code Actions for quick fixes
- File watcher for translation file changes
- Configurable translation file patterns and namespaces
