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

        // package.json の設定をマージ（オプション、将来実装）
        // if let Some(root) = &workspace_root {
        //     if let Some(package_settings) = loader::load_from_package_json(root)? {
        //         // マージロジック
        //     }
        // }

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

    // /// TODO: Doc
    // pub async fn update_global_settings(&self, settings: I18nSettings) {
    //     *self.global_settings.write().await = settings;
    //
    //     // グローバル設定変更時はワークスペース設定もクリア
    //     let mut workspace_settings = self.workspace_settings.write().await;
    //     workspace_settings.clear();
    // }
    //
    // /// ドキュメントの設定を取得
    // pub async fn get_document_settings(&self, uri: &Url) -> I18nSettings {
    //     let workspace_path = self.get_workspace_for_uri(uri).await;
    //
    //     // ワークスペース設定がキャッシュされているか確認
    //     {
    //         let workspace_settings = self.workspace_settings.read().await;
    //         if let Some(settings) = workspace_settings.get(&workspace_path) {
    //             return settings.clone();
    //         }
    //     }
    //
    //     // キャッシュがない場合は読み込み
    //     let settings = self.load_workspace_settings(&workspace_path).await;
    //     {
    //         let mut workspace_settings = self.workspace_settings.write().await;
    //         workspace_settings.insert(workspace_path, settings.clone());
    //     }
    //
    //     settings
    // }
    //
    // /// TODO: doc
    // async fn get_workspace_for_uri(&self, uri: &Url) -> PathBuf {
    //     // キャッシュからマッピングを確認
    //     {
    //         let file_mapping = self.file_to_workspace.read().await;
    //         if let Some(workspace) = file_mapping.get(uri) {
    //             return workspace.clone();
    //         }
    //     }
    //
    //     // ファイルパスからワークスペースルートを探索
    //     let file_path = PathBuf::from(uri.path());
    //     let workspace_root = self.find_workspace_root(&file_path);
    //
    //     // マッピングをキャッシュ
    //     {
    //         let mut file_mapping = self.file_to_workspace.write().await;
    //         file_mapping.insert(uri.clone(), workspace_root.clone());
    //     }
    //
    //     workspace_root
    // }
    //
    // /// TODO: doc
    // #[must_use]
    // pub fn find_workspace_root(&self, file_path: &Path) -> PathBuf {
    //     let mut current = file_path.parent().unwrap_or(file_path);
    //
    //     loop {
    //         if current.join("package.json").exists() {
    //             return current.to_path_buf();
    //         }
    //
    //         // if current.join(".js-i18n.json").exists() {
    //         //     return current.to_path_buf();
    //         // }
    //
    //         if current.join(".git").exists() {
    //             return current.to_path_buf();
    //         }
    //
    //         match current.parent() {
    //             Some(parent) => current = parent,
    //             None => return file_path.parent().unwrap_or(file_path).to_path_buf(),
    //         }
    //     }
    // }
    //
    // /// TODO: doc
    // async fn load_workspace_settings(&self, workspace_path: &Path) -> I18nSettings {
    //     use super::loader::ConfigLoader;
    //
    //     // if let Some(settings) = ConfigLoader::load_project_config(workspace_path).await {
    //     //     return settings;
    //     // }
    //
    //     if let Some(settings) = ConfigLoader::infer_from_package_json(workspace_path).await {
    //         return settings;
    //     }
    //
    //     let global_settings = self.global_settings.read().await;
    //     global_settings.clone()
    // }
}
