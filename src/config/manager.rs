//! 設定管理を行うモジュール

use std::path::PathBuf;

use super::{
    ConfigError,
    I18nSettings,
    loader,
};

/// 設定管理を行う
#[derive(Default, Debug, Clone)]
pub struct ConfigManager {
    /// 現在の設定
    current_settings: I18nSettings,

    /// ワークスペースのルートパス
    workspace_root: Option<PathBuf>,
}

impl ConfigManager {
    /// 新しい設定マネージャーを作成
    #[must_use]
    pub fn new() -> Self {
        Self { current_settings: I18nSettings::default(), workspace_root: None }
    }

    /// 設定を読み込む
    ///
    /// # Arguments
    /// * `workspace_root` - ワークスペースのルートパス
    ///
    /// # Returns
    /// - `Ok(())`: 設定の読み込みとバリデーション成功
    /// - `Err(ConfigError)`: エラー
    ///
    /// # Errors
    /// - ファイル読み込みエラー
    /// - JSON パースエラー
    /// - バリデーションエラー
    pub fn load_settings(&mut self, workspace_root: Option<PathBuf>) -> Result<(), ConfigError> {
        tracing::debug!("Loading settings for workspace: {:?}", workspace_root);

        // ワークスペースの設定を読み込み
        let settings = if let Some(root) = &workspace_root {
            loader::load_from_workspace(root)?.map_or_else(I18nSettings::default, |ws| {
                tracing::debug!("Loaded workspace settings: {:?}", ws);
                ws
            })
        } else {
            I18nSettings::default()
        };

        // バリデーション
        settings.validate().map_err(ConfigError::ValidationErrors)?;

        // 設定を保存
        self.current_settings = settings;
        self.workspace_root = workspace_root;
        tracing::debug!("Settings loaded successfully: {:?}", self.current_settings);

        Ok(())
    }

    /// 設定を更新する（`did_change_configuration` 用、将来実装）
    pub fn update_settings(&mut self, new_settings: I18nSettings) -> Result<(), ConfigError> {
        tracing::debug!("Updating settings...");

        // バリデーション
        new_settings.validate().map_err(ConfigError::ValidationErrors)?;

        // 設定を更新
        self.current_settings = new_settings;
        tracing::debug!("Settings updated successfully");

        Ok(())
    }

    /// 現在の設定を取得
    #[must_use]
    pub const fn get_settings(&self) -> &I18nSettings {
        &self.current_settings
    }

    /// ワークスペースルートを取得
    #[must_use]
    pub const fn workspace_root(&self) -> Option<&PathBuf> {
        self.workspace_root.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use rstest::rstest;
    use tempfile::TempDir;

    use super::*;

    #[allow(clippy::unwrap_used)]

    /// new: デフォルト値で作成される
    #[rstest]
    fn test_new_creates_default_settings() {
        let manager = ConfigManager::new();

        assert_eq!(manager.get_settings().key_separator, ".");
        assert!(manager.workspace_root().is_none());
    }

    /// load_settings: workspace_root が None の場合
    #[rstest]
    fn test_load_settings_without_workspace() {
        let mut manager = ConfigManager::new();

        let result = manager.load_settings(None);

        assert!(result.is_ok());
        assert_eq!(manager.get_settings().key_separator, ".");
        assert!(manager.workspace_root().is_none());
    }

    /// load_settings: 設定ファイルがある場合
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
    }

    /// load_settings: 設定ファイルがない場合はデフォルト値
    #[rstest]
    fn test_load_settings_without_config_file() {
        let temp_dir = TempDir::new().unwrap();

        let mut manager = ConfigManager::new();
        let result = manager.load_settings(Some(temp_dir.path().to_path_buf()));

        assert!(result.is_ok());
        assert_eq!(manager.get_settings().key_separator, ".");
    }

    /// update_settings: 有効な設定で更新成功
    #[rstest]
    fn test_update_settings_valid() {
        let mut manager = ConfigManager::new();
        let mut new_settings = I18nSettings::default();
        new_settings.key_separator = "-".to_string();

        let result = manager.update_settings(new_settings);

        assert!(result.is_ok());
        assert_eq!(manager.get_settings().key_separator, "-");
    }

    /// update_settings: 無効な設定でエラー
    #[rstest]
    fn test_update_settings_invalid() {
        let mut manager = ConfigManager::new();
        let mut new_settings = I18nSettings::default();
        new_settings.key_separator = String::new(); // 空文字は無効

        let result = manager.update_settings(new_settings);

        assert!(result.is_err());
    }
}
