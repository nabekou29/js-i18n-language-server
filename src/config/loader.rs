//! 設定ファイルの読み込み関数

use std::path::Path;

use super::{
    ConfigError,
    I18nSettings,
};

/// ワークスペースから設定を読み込む
///
/// `.js-i18n.json` ファイルを探して読み込む
///
/// # Arguments
/// * `workspace_root` - ワークスペースのルートパス
///
/// # Returns
/// - `Ok(Some(settings))`: 設定ファイルが見つかり、読み込みに成功
/// - `Ok(None)`: 設定ファイルが見つからない
/// - `Err(ConfigError)`: ファイル読み込みまたはパースエラー
///
/// # Errors
/// - ファイル読み込みエラー
/// - JSON パースエラー
pub(super) fn load_from_workspace(
    workspace_root: &Path,
) -> Result<Option<I18nSettings>, ConfigError> {
    let config_path = workspace_root.join(".js-i18n.json");

    if !config_path.exists() {
        tracing::debug!("Configuration file not found: {:?}", config_path);
        return Ok(None);
    }

    tracing::debug!("Loading configuration from: {:?}", config_path);

    let content = std::fs::read_to_string(&config_path)?;
    let settings: I18nSettings = serde_json::from_str(&content)?;

    Ok(Some(settings))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use std::fs;

    use rstest::rstest;
    use tempfile::TempDir;

    use super::*;

    /// `load_from_workspace`: 設定ファイルが存在する場合
    #[rstest]
    fn test_load_from_workspace_with_valid_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_content = r#"{"keySeparator": "-"}"#;
        fs::write(temp_dir.path().join(".js-i18n.json"), config_content).unwrap();

        let result = load_from_workspace(temp_dir.path());

        assert!(result.is_ok());
        let settings = result.unwrap();
        assert!(settings.is_some());
        assert_eq!(settings.unwrap().key_separator, "-");
    }

    /// `load_from_workspace`: 設定ファイルが存在しない場合
    #[rstest]
    fn test_load_from_workspace_no_config_file() {
        let temp_dir = TempDir::new().unwrap();

        let result = load_from_workspace(temp_dir.path());

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    /// `load_from_workspace`: JSON パースエラー
    #[rstest]
    fn test_load_from_workspace_invalid_json() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join(".js-i18n.json"), "invalid json").unwrap();

        let result = load_from_workspace(temp_dir.path());

        assert!(result.is_err());
    }
}
