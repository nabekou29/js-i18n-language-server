//! Doc
mod loader;
mod manager;
mod types;

pub use manager::ConfigManager;
pub use types::{
    ConfigError,
    I18nSettings,
    ServerSettings,
    TranslationFilesConfig,
    ValidationError,
};
