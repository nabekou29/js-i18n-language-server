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

/// package.json から設定を推測する
///
/// package.json の `jsI18n` フィールドから設定を読み込む
///
/// # Arguments
/// * `workspace_root` - ワークスペースのルートパス
///
/// # Returns
/// - `Ok(Some(settings))`: 設定が見つかり、読み込みに成功
/// - `Ok(None)`: 設定が見つからない
/// - `Err(ConfigError)`: ファイル読み込みまたはパースエラー
///
/// # Errors
/// - ファイル読み込みエラー
/// - JSON パースエラー
#[allow(dead_code)]
pub(super) fn load_from_package_json(
    workspace_root: &Path,
) -> Result<Option<I18nSettings>, ConfigError> {
    let package_json_path = workspace_root.join("package.json");

    if !package_json_path.exists() {
        tracing::debug!("package.json not found: {:?}", package_json_path);
        return Ok(None);
    }

    tracing::debug!("Checking package.json for configuration: {:?}", package_json_path);

    let content = std::fs::read_to_string(&package_json_path)?;
    let package_json: serde_json::Value = serde_json::from_str(&content)?;

    // package.json の "jsI18n" フィールドを探す
    if let Some(js_i18n_config) = package_json.get("jsI18n") {
        let settings: I18nSettings = serde_json::from_value(js_i18n_config.clone())?;
        tracing::debug!("Loaded settings from package.json");
        return Ok(Some(settings));
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use rstest::rstest;
    use tempfile::TempDir;

    use super::*;

    #[allow(clippy::unwrap_used)]

    /// load_from_workspace: 設定ファイルが存在する場合
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

    /// load_from_workspace: 設定ファイルが存在しない場合
    #[rstest]
    fn test_load_from_workspace_no_config_file() {
        let temp_dir = TempDir::new().unwrap();

        let result = load_from_workspace(temp_dir.path());

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    /// load_from_workspace: JSON パースエラー
    #[rstest]
    fn test_load_from_workspace_invalid_json() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join(".js-i18n.json"), "invalid json").unwrap();

        let result = load_from_workspace(temp_dir.path());

        assert!(result.is_err());
    }

    /// load_from_package_json: jsI18n フィールドがある場合
    #[rstest]
    fn test_load_from_package_json_with_js_i18n_field() {
        let temp_dir = TempDir::new().unwrap();
        let package_json = r#"{
            "name": "test",
            "jsI18n": {"keySeparator": ":"}
        }"#;
        fs::write(temp_dir.path().join("package.json"), package_json).unwrap();

        let result = load_from_package_json(temp_dir.path());

        assert!(result.is_ok());
        let settings = result.unwrap();
        assert!(settings.is_some());
        assert_eq!(settings.unwrap().key_separator, ":");
    }

    /// load_from_package_json: jsI18n フィールドがない場合
    #[rstest]
    fn test_load_from_package_json_without_js_i18n_field() {
        let temp_dir = TempDir::new().unwrap();
        let package_json = r#"{"name": "test"}"#;
        fs::write(temp_dir.path().join("package.json"), package_json).unwrap();

        let result = load_from_package_json(temp_dir.path());

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    /// load_from_package_json: package.json が存在しない場合
    #[rstest]
    fn test_load_from_package_json_no_file() {
        let temp_dir = TempDir::new().unwrap();

        let result = load_from_package_json(temp_dir.path());

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    /// load_from_package_json: JSON パースエラー
    #[rstest]
    fn test_load_from_package_json_invalid_json() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("package.json"), "invalid json").unwrap();

        let result = load_from_package_json(temp_dir.path());

        assert!(result.is_err());
    }
}
