//! TODO
use serde::{
    Deserialize,
    Serialize,
};

/// TODO: doc
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerSettings {
    /// TODO: doc
    pub js_i18n: I18nSettings,
}

/// TODO: doc
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct I18nSettings {
    /// TODO: doc
    pub translation_files: TranslationFilesConfig,
    /// TODO: doc
    pub include_patterns: Vec<String>,
    /// TODO: doc
    pub exclude_patterns: Vec<String>,
}

/// TODO: doc
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationFilesConfig {
    /// TODO: doc
    pub file_pattern: String,
}

impl Default for I18nSettings {
    fn default() -> Self {
        Self {
            translation_files: TranslationFilesConfig {
                file_pattern: "**/{locales,messages}/*.json".to_string(),
            },
            include_patterns: vec!["**/*.{js,jsx,ts,tsx}".to_string()],
            exclude_patterns: vec!["node_modules/**".to_string()],
        }
    }
}
