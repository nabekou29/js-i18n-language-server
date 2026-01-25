//! Configuration file loading.

use std::path::{
    Path,
    PathBuf,
};

use super::{
    ConfigError,
    I18nSettings,
};

/// Result of loading configuration from workspace.
pub(super) struct LoadResult {
    pub settings: I18nSettings,
    /// Directory containing the config file (if found).
    pub config_dir: Option<PathBuf>,
}

/// Loads settings from `.js-i18n.json` in the workspace root.
///
/// Returns both the settings and the directory where the config was found.
pub(super) fn load_from_workspace(workspace_root: &Path) -> Result<LoadResult, ConfigError> {
    let config_path = workspace_root.join(".js-i18n.json");

    if !config_path.exists() {
        tracing::debug!("Configuration file not found: {:?}", config_path);
        return Ok(LoadResult { settings: I18nSettings::default(), config_dir: None });
    }

    tracing::debug!("Loading configuration from: {:?}", config_path);

    let content = std::fs::read_to_string(&config_path)?;
    let settings: I18nSettings = serde_json::from_str(&content)?;

    Ok(LoadResult { settings, config_dir: Some(workspace_root.to_path_buf()) })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use std::fs;

    use rstest::rstest;
    use tempfile::TempDir;

    use super::*;

    #[rstest]
    fn test_load_from_workspace_with_valid_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_content = r#"{"keySeparator": "-"}"#;
        fs::write(temp_dir.path().join(".js-i18n.json"), config_content).unwrap();

        let result = load_from_workspace(temp_dir.path());

        assert!(result.is_ok());
        let load_result = result.unwrap();
        assert!(load_result.config_dir.is_some());
        assert_eq!(load_result.settings.key_separator, "-");
        assert_eq!(load_result.config_dir.unwrap(), temp_dir.path());
    }

    #[rstest]
    fn test_load_from_workspace_no_config_file() {
        let temp_dir = TempDir::new().unwrap();

        let result = load_from_workspace(temp_dir.path());

        assert!(result.is_ok());
        let load_result = result.unwrap();
        assert!(load_result.config_dir.is_none());
        // Default settings should be used
        assert_eq!(load_result.settings.key_separator, ".");
    }

    #[rstest]
    fn test_load_from_workspace_invalid_json() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join(".js-i18n.json"), "invalid json").unwrap();

        let result = load_from_workspace(temp_dir.path());

        assert!(result.is_err());
    }
}
