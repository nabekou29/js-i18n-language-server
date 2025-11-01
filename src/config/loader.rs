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
