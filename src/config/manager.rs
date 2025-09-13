//! TODO
use std::collections::HashMap;
use std::path::{
    Path,
    PathBuf,
};
use std::sync::Arc;

use tokio::sync::RwLock;
use tower_lsp::lsp_types::Url;

use crate::config::I18nSettings;

/// TODO
#[derive(Default, Debug, Clone)]
pub struct ConfigManager {
    /// TODO
    global_settings: Arc<RwLock<I18nSettings>>,
    /// TODO
    workspace_settings: Arc<RwLock<HashMap<PathBuf, I18nSettings>>>,
    /// TODO
    file_to_workspace: Arc<RwLock<HashMap<Url, PathBuf>>>,
}

impl ConfigManager {
    /// TODO
    #[must_use]
    pub fn new() -> Self {
        Self {
            global_settings: Arc::new(RwLock::new(I18nSettings::default())),
            workspace_settings: Arc::new(RwLock::new(HashMap::new())),
            file_to_workspace: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// TODO: Doc
    pub async fn update_global_settings(&self, settings: I18nSettings) {
        *self.global_settings.write().await = settings;

        // グローバル設定変更時はワークスペース設定もクリア
        let mut workspace_settings = self.workspace_settings.write().await;
        workspace_settings.clear();
    }

    /// ドキュメントの設定を取得
    pub async fn get_document_settings(&self, uri: &Url) -> I18nSettings {
        let workspace_path = self.get_workspace_for_uri(uri).await;

        // ワークスペース設定がキャッシュされているか確認
        {
            let workspace_settings = self.workspace_settings.read().await;
            if let Some(settings) = workspace_settings.get(&workspace_path) {
                return settings.clone();
            }
        }

        // キャッシュがない場合は読み込み
        let settings = self.load_workspace_settings(&workspace_path).await;
        {
            let mut workspace_settings = self.workspace_settings.write().await;
            workspace_settings.insert(workspace_path, settings.clone());
        }

        settings
    }

    /// TODO: doc
    async fn get_workspace_for_uri(&self, uri: &Url) -> PathBuf {
        // キャッシュからマッピングを確認
        {
            let file_mapping = self.file_to_workspace.read().await;
            if let Some(workspace) = file_mapping.get(uri) {
                return workspace.clone();
            }
        }

        // ファイルパスからワークスペースルートを探索
        let file_path = PathBuf::from(uri.path());
        let workspace_root = self.find_workspace_root(&file_path);

        // マッピングをキャッシュ
        {
            let mut file_mapping = self.file_to_workspace.write().await;
            file_mapping.insert(uri.clone(), workspace_root.clone());
        }

        workspace_root
    }

    /// TODO: doc
    #[must_use]
    pub fn find_workspace_root(&self, file_path: &Path) -> PathBuf {
        let mut current = file_path.parent().unwrap_or(file_path);

        loop {
            if current.join("package.json").exists() {
                return current.to_path_buf();
            }

            // if current.join(".js-i18n.json").exists() {
            //     return current.to_path_buf();
            // }

            if current.join(".git").exists() {
                return current.to_path_buf();
            }

            match current.parent() {
                Some(parent) => current = parent,
                None => return file_path.parent().unwrap_or(file_path).to_path_buf(),
            }
        }
    }

    /// TODO: doc
    async fn load_workspace_settings(&self, workspace_path: &Path) -> I18nSettings {
        use super::loader::ConfigLoader;

        // if let Some(settings) = ConfigLoader::load_project_config(workspace_path).await {
        //     return settings;
        // }

        if let Some(settings) = ConfigLoader::infer_from_package_json(workspace_path).await {
            return settings;
        }

        let global_settings = self.global_settings.read().await;
        global_settings.clone()
    }
}
