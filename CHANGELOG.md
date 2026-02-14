
## [0.4.4] - 2026-02-14

### Documentation

- Add icon and editor extension links to README

### Features

- Ignore didChangeConfiguration when .js-i18n.json exists


## [0.4.3] - 2026-02-14

### Bug Fixes

- Prevent Salsa boxcar panic from stale IDs after reset_state
- Include namespace-unresolved usages in references and rename
- Skip non-file URI schemes in execute command handlers

### Refactor

- Change default log level to info and add startup log


## [0.4.2] - 2026-02-12

### Bug Fixes

- Filter translations by namespace in missing/unused key diagnostics
- Add namespace filtering to hover, goto_definition, references, rename, and decorations

### Refactor

- Extract resolve_usage_namespace and simplify filter_by_namespace


## [0.4.1] - 2026-02-11

### Bug Fixes

- Remove unused workspaceFolders support
- Use initialize params for workspace folders in multi-server setups
- Use markdown-compatible line breaks in hover content
- Show delete key as Quick Fix for unused translation keys
- Skip non-file URI schemes in document sync handlers

### Features

- Add serverInfo to initialize response
- Add --log-level CLI arg and JS_I18N_LOG env var


## [0.4.0] - 2026-02-08

### Bug Fixes

- Resolve all clippy lint errors

### Documentation

- Condense comments.md rules

### Features

- Add translation key rename support via LSP textDocument/rename
- Add delete translation key action
- Support rename and delete key code action from JSON translation files
- Add plural fallback to decoration values

### Refactor

- Extract shared helpers and fix lock ordering


## [0.3.0] - 2026-02-07

### Documentation

- Fix CHANGELOG.md
- Remove virtualText.maxLength/maxWidth from configuration and LSP docs
- Update configuration docs for new diagnostics structure
- Add rationale for #[rstest] over #[googletest::test] in test rules

### Features

- Add i18n.getAvailableLanguages command
- Add enabled/severity support to missing translation diagnostics
- Add ignorePatterns to unused translation diagnostics
- Add configurable severity to unused translation diagnostics

### Refactor

- Replace #[googletest::test]/expect_that! with #[rstest]/assert_that!


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
