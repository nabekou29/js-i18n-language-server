//! Configuration management.

use std::path::PathBuf;

use super::matcher::FileMatcher;
use super::{
    ConfigError,
    I18nSettings,
    loader,
};

/// Manages configuration settings for the LSP server.
#[derive(Debug, Clone)]
pub struct ConfigManager {
    current_settings: I18nSettings,
    workspace_root: Option<PathBuf>,
    /// Directory containing `.js-i18n.json` (if found).
    /// Patterns are relative to this directory.
    config_dir: Option<PathBuf>,
    file_matcher: Option<FileMatcher>,
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigManager {
    #[must_use]
    pub fn new() -> Self {
        Self {
            current_settings: I18nSettings::default(),
            workspace_root: None,
            config_dir: None,
            file_matcher: None,
        }
    }

    /// Loads settings from the workspace root.
    ///
    /// If a config file is found, patterns are relative to the config file's directory.
    /// Otherwise, patterns are relative to the workspace root.
    pub fn load_settings(&mut self, workspace_root: Option<PathBuf>) -> Result<(), ConfigError> {
        tracing::debug!("Loading settings for workspace: {:?}", workspace_root);

        let (settings, config_dir) = if let Some(root) = &workspace_root {
            let result = loader::load_from_workspace(root)?;
            if result.config_dir.is_some() {
                tracing::debug!("Loaded workspace settings: {:?}", result.settings);
            }
            (result.settings, result.config_dir)
        } else {
            (I18nSettings::default(), None)
        };

        settings.validate().map_err(ConfigError::ValidationErrors)?;

        // Use config_dir for pattern matching if available, otherwise workspace_root
        let pattern_base = config_dir.as_ref().or(workspace_root.as_ref());

        let file_matcher =
            pattern_base.and_then(|base| match FileMatcher::new(base.clone(), &settings) {
                Ok(matcher) => Some(matcher),
                Err(e) => {
                    tracing::warn!("Failed to build file matcher: {}", e);
                    None
                }
            });

        self.current_settings = settings;
        self.workspace_root = workspace_root;
        self.config_dir = config_dir;
        self.file_matcher = file_matcher;
        tracing::debug!(
            "Settings loaded successfully. config_dir: {:?}, workspace_root: {:?}",
            self.config_dir,
            self.workspace_root
        );

        Ok(())
    }

    pub fn update_settings(&mut self, new_settings: I18nSettings) -> Result<(), ConfigError> {
        tracing::debug!("Updating settings...");

        new_settings.validate().map_err(ConfigError::ValidationErrors)?;

        self.current_settings = new_settings;
        tracing::debug!("Settings updated successfully");

        Ok(())
    }

    #[must_use]
    pub const fn get_settings(&self) -> &I18nSettings {
        &self.current_settings
    }

    #[must_use]
    pub const fn workspace_root(&self) -> Option<&PathBuf> {
        self.workspace_root.as_ref()
    }

    /// Returns the directory containing `.js-i18n.json` (if found).
    /// Patterns are relative to this directory.
    #[must_use]
    pub const fn config_dir(&self) -> Option<&PathBuf> {
        self.config_dir.as_ref()
    }

    /// Returns the base directory for pattern matching.
    /// Uses `config_dir` if available, otherwise `workspace_root`.
    #[must_use]
    pub fn pattern_base_dir(&self) -> Option<&PathBuf> {
        self.config_dir.as_ref().or(self.workspace_root.as_ref())
    }

    #[must_use]
    pub const fn file_matcher(&self) -> Option<&FileMatcher> {
        self.file_matcher.as_ref()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::field_reassign_with_default)]
mod tests {
    use std::fs;

    use rstest::rstest;
    use tempfile::TempDir;

    use super::*;

    #[rstest]
    fn test_new_creates_default_settings() {
        let manager = ConfigManager::new();

        assert_eq!(manager.get_settings().key_separator, ".");
        assert!(manager.workspace_root().is_none());
    }

    #[rstest]
    fn test_load_settings_without_workspace() {
        let mut manager = ConfigManager::new();

        let result = manager.load_settings(None);

        assert!(result.is_ok());
        assert_eq!(manager.get_settings().key_separator, ".");
        assert!(manager.workspace_root().is_none());
    }

    #[rstest]
    fn test_load_settings_with_config_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_content = r#"{"keySeparator": "-"}"#;
        fs::write(temp_dir.path().join(".js-i18n.json"), config_content).unwrap();

        let mut manager = ConfigManager::new();
        let result = manager.load_settings(Some(temp_dir.path().to_path_buf()));

        assert!(result.is_ok());
        assert_eq!(manager.get_settings().key_separator, "-");
        assert!(manager.workspace_root().is_some());
        // config_dir should be set when config file exists
        assert!(manager.config_dir().is_some());
        assert_eq!(manager.pattern_base_dir(), manager.config_dir());
    }

    #[rstest]
    fn test_load_settings_without_config_file() {
        let temp_dir = TempDir::new().unwrap();

        let mut manager = ConfigManager::new();
        let result = manager.load_settings(Some(temp_dir.path().to_path_buf()));

        assert!(result.is_ok());
        assert_eq!(manager.get_settings().key_separator, ".");
        // config_dir should be None when no config file
        assert!(manager.config_dir().is_none());
        // pattern_base_dir falls back to workspace_root
        assert_eq!(manager.pattern_base_dir(), manager.workspace_root());
    }

    #[rstest]
    fn test_update_settings_valid() {
        let mut manager = ConfigManager::new();
        let mut new_settings = I18nSettings::default();
        new_settings.key_separator = "-".to_string();

        let result = manager.update_settings(new_settings);

        assert!(result.is_ok());
        assert_eq!(manager.get_settings().key_separator, "-");
    }

    #[rstest]
    fn test_update_settings_invalid() {
        let mut manager = ConfigManager::new();
        let mut new_settings = I18nSettings::default();
        new_settings.key_separator = String::new();

        let result = manager.update_settings(new_settings);

        assert!(result.is_err());
    }
}
