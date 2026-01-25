# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/nabekou29/js-i18n-language-server/releases/tag/v0.1.0) - 2026-01-25

### Added

- *(ide)* support real-time translation updates in editor
- *(config)* make glob patterns relative to config file directory
- *(ide)* support array notation in child key matching
- *(ide)* add namespace filtering module
- *(ir)* add namespace fields to KeyUsage
- *(queries)* add array namespace and ns option support
- *(syntax)* add namespace support to type definitions
- *(translation)* detect namespace from file path
- *(config)* add default_namespace setting
- *(ide)* add plural suffix support for i18next
- *(analyzer)* support global translation functions and method calls
- *(handlers)* add deleteUnusedKeys command for bulk cleanup
- *(code_actions)* add delete_keys_from_json_text for key removal
- *(config)* add FileMatcher for centralized pattern matching
- *(hover)* add language priority sorting
- *(backend)* add collect_sorted_languages utility
- *(types)* add SourceRange::contains and LSP type conversions
- *(config)* implement config file change handling with progress notification
- *(playground)* add custom settings project for key_separator testing
- *(syntax)* add configurable key_separator support
- *(analyzer)* add multi-query support and expand i18n pattern handling
- *(queries)* add tree-sitter queries for i18n patterns
- *(config)* add primaryLanguages and currentLanguage settings
- *(diagnostics)* send unused key diagnostics at initialization and on file changes
- *(lsp)* add i18n.getDecorations command for virtual text support
- *(ide)* add virtual_text module for translation decorations
- *(config)* add VirtualTextConfig for translation decoration settings
- *(lsp)* implement code action and execute command handlers
- *(code-actions)* add code action generation module
- *(diagnostics)* check translations per language with configurable filtering
- *(config)* add required_languages and optional_languages settings
- *(hover)* support hover in translation files
- *(translation)* add value position tracking for translation files
- *(backend)* add file watcher for translation files
- *(completion)* support empty arguments t() for completion
- *(ide)* display multiple language translations in completion
- *(ide)* add completion support for translation keys
- *(ide)* support Go to Definition from translation files
- *(ide)* add Go to Definition support for translation keys
- *(config)* add configurable indexing thread pool size
- *(ide)* add references support for JSON translation files
- *(ide)* add immediate diagnostics on file open
- *(ide)* implement hover feature to display translations
- *(input)* add heuristic language detection from file paths
- *(ide)* implement diagnostic generation for missing translation keys
- *(indexer)* add translation file indexing to workspace indexer
- *(input)* add Translation input and JSON flattening utility
- *(ide,indexer)* implement workspace reindexing and incremental updates
- *(indexer)* integrate Salsa database for incremental analysis
- *(db)* add Send trait and Default implementation to Salsa database
- *(config)* implement complete configuration management and Backend integration
- *(config)* define validation error types and error messages
- *(ide)* add IDE feature module stubs
- add salsa-based architecture foundation
- *(indexer)* implement workspace indexing system
- *(config)* add configuration management system
- *(core)* implement initial js-i18n-language-server

### Fixed

- resolve clippy lints for nightly 1.95.0
- remove unnecessary raw string hashes for nightly clippy
- *(query)* support complex object options in t() calls
- *(rules)* use YAML frontmatter for path filtering
- resolve clippy lint warnings across test modules
- resolve clippy pedantic lint errors
- resolve clippy warnings for static docs and function length
- *(indexer)* queue file updates during indexing to prevent data loss
- *(indexer)* move flag setting inside lock to prevent race condition
- *(lifecycle)* use channel-based Progress notification to fix race condition
- *(lifecycle)* add delay before Progress End to avoid race condition
- *(config)* update default translation file pattern for nested structures
- *(diagnostics)* skip empty keys during validation

### Other

- disable crates.io publish in release-plz
- add release-plz for automated releases
- add release workflow for multi-platform builds
- *(mise)* pin rust nightly to 2026-01-24 (1.95.0)
- use mise-action for consistent toolchain versions
- *(mise)* align task commands with CI configuration
- add GitHub Actions workflow with coverage
- *(config)* simplify by removing config_dir abstraction
- *(indexer)* add test for config_dir != workspace_root scenario
- *(matcher)* add #[deprecated] attribute to workspace_root()
- restructure documentation for developers and integrators
- bump version to 0.0.1 for initial release
- *(mise)* include queries in build-install sources
- fix clippy lints
- translate comments to English
- *(rules)* add code comment policy
- *(namespace)* simplify implementation
- *(rules)* refactor rules for minimal startup context
- *(playground)* add namespace demo
- *(rules)* expand testing rules with in-source test patterns
- add CLAUDE.md with path-specific rules for minimal startup context
- migrate from mod.rs to modern Rust module style
- *(ide)* simplify sorting logic and imports
- *(ide)* use functional style and idiomatic Rust patterns
- *(hover)* introduce LanguagePriority enum for type-safe sorting
- *(goto_definition)* extract helpers and use or_else chain
- *(execute_command)* extract helpers for file edit operations
- *(backend)* extract helper methods and simplify reindex_workspace
- *(translation)* split long function into smaller focused units
- *(types)* add From trait and helper for tree-sitter conversion
- consolidate position_in_range into SourceRange::contains
- *(ide)* use SourceRange type conversions
- *(handlers)* use collect_sorted_languages for language resolution
- *(config)* remove unused load_from_package_json function
- *(translation)* remove unnecessary dead_code allow
- *(backend)* convert create_diagnostic_options to associated function
- *(completion)* add comprehensive completion tests
- *(indexer)* add comprehensive workspace indexer tests
- add boundary condition and config tests
- resolve TODO comments with proper documentation
- *(indexer)* add query cache and reduce default thread count
- *(playground)* add custom settings project documentation
- *(playground)* restructure with separate projects per library
- *(logging)* replace rolling file appender with configurable logging
- *(trace)* replace verbose trace logs with tracing::instrument
- *(deps)* update major dependencies
- *(deps)* update dependencies
- *(ide)* reduce clone() calls and shorten lock hold times
- *(syntax, ide, config)* add CaptureName enum, batch locks, and key_separator support
- *(playground)* add code action keymap to nvim config
- add jsonc-parser for CST-based JSON manipulation
- add explanatory comments to clippy allow attributes
- *(backend)* extract LSP handlers into dedicated modules
- *(mise)* add cargo-llvm-cov for test coverage
- *(hover)* add unit tests for generate_hover_content
- *(ide)* extract ServerState from Backend
- *(backend)* extract common helper methods to reduce code duplication
- *(backend)* use config-based pattern for file watching
- replace let chains with nested if-let expressions
- *(config)* replace let chain with nested if-let
- update tool versions (hk 1.28.0, pkl 0.30.2)
- *(query)* split capture names for get and call contexts
- *(indexer)* simplify logging by removing timing instrumentation
- *(indexer)* implement true parallel execution with blocking thread pool
- add num_cpus dependency for CPU core detection
- *(indexer)* prioritize translations and limit concurrency to prevent cpu starvation
- *(playground)* fix nvim config typo and update test data
- add tree-sitter-json dependency
- *(playground)* add LSP keybindings to nvim config
- *(ide)* extract common diagnostic logic into helper method
- *(input)* add comprehensive tests for detect_language_from_path
- add manual Debug implementations
- fix clippy warnings
- *(ir)* remove unused Translation struct
- migrate tests to googletest matchers
- *(playground)* enhance nvim config for diagnostic display
- update development environment settings
- improve test assertions with googletest matchers
- replace assertor with googletest
- *(indexer)* improve workspace indexing implementation
- move analyzer to syntax module with salsa integration
- add salsa dependency and update configuration
- *(analyzer)* consolidate error types and use LSP types
- *(playground)* enhance nvim LSP configuration
- *(gitignore)* add .repro_minimal to ignore list
- update development dependencies
- add projects for debug
- init
