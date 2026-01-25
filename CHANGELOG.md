# Changelog

All notable changes to this project will be documented in this file.
## [0.1.0] - 2026-01-25

### Bug Fixes

- Skip empty keys during validation
- Update default translation file pattern for nested structures
- Add delay before Progress End to avoid race condition
- Use channel-based Progress notification to fix race condition
- Move flag setting inside lock to prevent race condition
- Queue file updates during indexing to prevent data loss
- Resolve clippy warnings for static docs and function length
- Resolve clippy pedantic lint errors
- Resolve clippy lint warnings across test modules
- Use YAML frontmatter for path filtering
- Support complex object options in t() calls
- Remove unnecessary raw string hashes for nightly clippy
- Resolve clippy lints for nightly 1.95.0
- Correct release.toml format for cargo-release v0.25
- Use usage field for release task arguments
- Move MISE_ENV to job level for all steps

### Documentation

- Add custom settings project documentation
- Resolve TODO comments with proper documentation
- Add CLAUDE.md with path-specific rules for minimal startup context
- Expand testing rules with in-source test patterns
- Refactor rules for minimal startup context
- Add code comment policy
- Restructure documentation for developers and integrators
- Add initial CHANGELOG for v0.1.0 release

### Features

- Implement initial js-i18n-language-server
- Add configuration management system
- Implement workspace indexing system
- Add salsa-based architecture foundation
- Add IDE feature module stubs
- Define validation error types and error messages
- Implement complete configuration management and Backend integration
- Add Send trait and Default implementation to Salsa database
- Integrate Salsa database for incremental analysis
- Implement workspace reindexing and incremental updates
- Add Translation input and JSON flattening utility
- Add translation file indexing to workspace indexer
- Implement diagnostic generation for missing translation keys
- Add heuristic language detection from file paths
- Implement hover feature to display translations
- Add immediate diagnostics on file open
- Add references support for JSON translation files
- Add configurable indexing thread pool size
- Add Go to Definition support for translation keys
- Support Go to Definition from translation files
- Add completion support for translation keys
- Display multiple language translations in completion
- Support empty arguments t() for completion
- Add file watcher for translation files
- Add value position tracking for translation files
- Support hover in translation files
- Add required_languages and optional_languages settings
- Check translations per language with configurable filtering
- Add code action generation module
- Implement code action and execute command handlers
- Add VirtualTextConfig for translation decoration settings
- Add virtual_text module for translation decorations
- Add i18n.getDecorations command for virtual text support
- Send unused key diagnostics at initialization and on file changes
- Add primaryLanguages and currentLanguage settings
- Add tree-sitter queries for i18n patterns
- Add multi-query support and expand i18n pattern handling
- Add configurable key_separator support
- Add custom settings project for key_separator testing
- Implement config file change handling with progress notification
- Add SourceRange::contains and LSP type conversions
- Add collect_sorted_languages utility
- Add language priority sorting
- Add FileMatcher for centralized pattern matching
- Add delete_keys_from_json_text for key removal
- Add deleteUnusedKeys command for bulk cleanup
- Support global translation functions and method calls
- Add plural suffix support for i18next
- Add default_namespace setting
- Detect namespace from file path
- Add namespace support to type definitions
- Add array namespace and ns option support
- Add namespace fields to KeyUsage
- Add namespace filtering module
- Support array notation in child key matching
- Make glob patterns relative to config file directory
- Support real-time translation updates in editor

### Performance

- Implement true parallel execution with blocking thread pool
- Add query cache and reduce default thread count

### Refactor

- Consolidate error types and use LSP types
- Move analyzer to syntax module with salsa integration
- Improve workspace indexing implementation
- Remove unused Translation struct
- Extract common diagnostic logic into helper method
- Prioritize translations and limit concurrency to prevent cpu starvation
- Simplify logging by removing timing instrumentation
- Split capture names for get and call contexts
- Replace let chain with nested if-let
- Replace let chains with nested if-let expressions
- Use config-based pattern for file watching
- Extract common helper methods to reduce code duplication
- Extract ServerState from Backend
- Extract LSP handlers into dedicated modules
- Add explanatory comments to clippy allow attributes
- Add CaptureName enum, batch locks, and key_separator support
- Reduce clone() calls and shorten lock hold times
- Replace verbose trace logs with tracing::instrument
- Replace rolling file appender with configurable logging
- Restructure with separate projects per library
- Convert create_diagnostic_options to associated function
- Remove unnecessary dead_code allow
- Remove unused load_from_package_json function
- Use collect_sorted_languages for language resolution
- Use SourceRange type conversions
- Consolidate position_in_range into SourceRange::contains
- Add From trait and helper for tree-sitter conversion
- Split long function into smaller focused units
- Extract helper methods and simplify reindex_workspace
- Extract helpers for file edit operations
- Extract helpers and use or_else chain
- Introduce LanguagePriority enum for type-safe sorting
- Use functional style and idiomatic Rust patterns
- Simplify sorting logic and imports
- Migrate from mod.rs to modern Rust module style
- Simplify implementation
- Add #[deprecated] attribute to workspace_root()
- Simplify by removing config_dir abstraction
- Split config into development and CI environments

### Styling

- Translate comments to English

### Testing

- Improve test assertions with googletest matchers
- Migrate tests to googletest matchers
- Add comprehensive tests for detect_language_from_path
- Add unit tests for generate_hover_content
- Add boundary condition and config tests
- Add comprehensive workspace indexer tests
- Add comprehensive completion tests
- Add test for config_dir != workspace_root scenario

### Build

- Add salsa dependency and update configuration
- Add tree-sitter-json dependency
- Add num_cpus dependency for CPU core detection

### Deps

- Add jsonc-parser for CST-based JSON manipulation


