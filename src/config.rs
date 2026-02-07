//! Doc
/// Config file loader
mod loader;
/// Configuration manager
mod manager;
/// Source file pattern matcher
mod matcher;
/// Configuration types and settings
mod types;

pub use manager::ConfigManager;
pub use matcher::{
    FileMatcher,
    MatcherError,
};
pub use types::{
    ConfigError,
    DiagnosticsConfig,
    I18nSettings,
    MissingTranslationConfig,
    ServerSettings,
    Severity,
    TranslationFilesConfig,
    UnusedTranslationConfig,
    ValidationError,
};
