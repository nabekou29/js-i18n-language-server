//! Doc
/// Config file loader
mod loader;
/// Configuration manager
mod manager;
/// Configuration types and settings
mod types;

pub use manager::ConfigManager;
pub use types::{
    ConfigError,
    I18nSettings,
    ServerSettings,
    TranslationFilesConfig,
    ValidationError,
};
