//! TODO
use std::path::PathBuf;
use std::{
    collections::HashMap,
    path::Path,
    sync::Arc,
};

use globset::{
    Glob,
    GlobSetBuilder,
};
use ignore::WalkBuilder;
use tokio::sync::RwLock;
use tower_lsp::lsp_types::Url;
use tracing::debug;

use crate::config::ConfigManager;
use crate::indexer::types::{
    IndexerError,
    KeyUsageLocation,
};

/// TODO
#[derive(Clone, Debug, Default)]
pub struct WorkspaceIndexer {
    /// TODO
    usage_index: Arc<RwLock<HashMap<String, Vec<KeyUsageLocation>>>>,
}

impl WorkspaceIndexer {
    /// 新しいインデクサーを作成
    #[must_use]
    pub fn new() -> Self {
        Self { usage_index: Arc::new(RwLock::new(HashMap::new())) }
    }

    /// ワークスペースをインデックス
    ///
    /// # Errors
    pub async fn index_workspace(
        &self,
        workspace_path: &Path,
        config_manager: &ConfigManager,
    ) -> Result<(), IndexerError> {
        debug!(workspace_path = %workspace_path.display(), "Indexing workspace");
        let workspace_uri = Url::from_file_path(workspace_path)
            .map_err(|()| IndexerError::InvalidPath(workspace_path.display().to_string()))?;
        let settings = config_manager.get_document_settings(&workspace_uri).await;
        let include_patterns = &settings.include_patterns;
        let exclude_patterns = &settings.exclude_patterns;

        let files = Self::find_source_files(workspace_path, include_patterns, exclude_patterns)?;
        for file in files {
            debug!("Indexing file: {}", file.display());
        }

        Ok(())
    }

    /// ソースファイルを検索
    fn find_source_files(
        workspace_path: &Path,
        include_patterns: &[String],
        exclude_patterns: &[String],
    ) -> Result<Vec<PathBuf>, IndexerError> {
        let mut found_files = Vec::new();
        // Include パターンセットをビルド
        let mut include_builder = GlobSetBuilder::new();
        for pattern in include_patterns {
            let glob = Glob::new(pattern).map_err(|e| {
                IndexerError::Error(format!("Invalid include pattern '{pattern}': {e}"))
            })?;
            include_builder.add(glob);
        }
        let include_set = include_builder
            .build()
            .map_err(|e| IndexerError::Error(format!("Failed to build include patterns: {e}")))?;

        // Exclude パターンセットをビルド
        let mut exclude_builder = GlobSetBuilder::new();
        for pattern in exclude_patterns {
            let glob = Glob::new(pattern).map_err(|e| {
                IndexerError::Error(format!("Invalid exclude pattern '{pattern}': {e}"))
            })?;
            exclude_builder.add(glob);
        }
        let exclude_set = exclude_builder
            .build()
            .map_err(|e| IndexerError::Error(format!("Failed to build exclude patterns: {e}")))?;

        // ignore クレートでファイルを走査
        for result in WalkBuilder::new(workspace_path)
            // TODO: 設定で変更可能にするかも
            .hidden(false)
            .git_ignore(true)
            .git_global(true)
            .git_exclude(true)
            .follow_links(false)
            .build()
        {
            let entry = match result {
                Ok(entry) => entry,
                Err(err) => {
                    debug!(?err, "Failed to read directory entry");
                    continue;
                }
            };

            // ファイルのみを対象
            if !entry.file_type().is_some_and(|ft| ft.is_file()) {
                continue;
            }

            let path = entry.path();

            // workspace からの相対パスを取得
            let Ok(relative_path) = path.strip_prefix(workspace_path) else {
                continue;
            };
            if !include_set.is_match(relative_path) || exclude_set.is_match(relative_path) {
                continue;
            }

            found_files.push(path.to_path_buf());
        }

        Ok(found_files)
    }
}
