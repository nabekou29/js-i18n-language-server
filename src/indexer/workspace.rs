//! TODO
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use globset::{
    Glob,
    GlobSetBuilder,
};
use ignore::WalkBuilder;
use tokio::sync::Mutex;
use tower_lsp::lsp_types::Url;

use crate::config::ConfigManager;
use crate::indexer::types::IndexerError;
use crate::input::source::SourceFile;
use crate::input::translation::{
    Translation,
    load_translation_file,
};

/// TODO
#[derive(Clone, Copy, Debug, Default)]
pub struct WorkspaceIndexer {}

impl WorkspaceIndexer {
    /// 新しいインデクサーを作成
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }

    /// ワークスペースをインデックス
    ///
    /// # Errors
    pub async fn index_workspace(
        &self,
        db: crate::db::I18nDatabaseImpl,
        workspace_path: &Path,
        config_manager: &ConfigManager,
        source_files: Arc<Mutex<HashMap<PathBuf, SourceFile>>>,
    ) -> Result<Vec<Translation>, IndexerError> {
        tracing::debug!(workspace_path = %workspace_path.display(), "Indexing workspace");
        let settings = config_manager.get_settings();
        let include_patterns = &settings.include_patterns;
        let exclude_patterns = &settings.exclude_patterns;

        // ソースファイルをインデックス
        let files = Self::find_source_files(workspace_path, include_patterns, exclude_patterns)?;

        tracing::info!(file_count = files.len(), "Found source files");

        // 並列処理でファイルをインデックス
        // 各ファイルに対して database のクローンを作成（Salsa のクローンは安価）
        let futures: Vec<_> = files.iter().map(|file| self.index_file(db.clone(), file)).collect();

        let results = futures::future::join_all(futures).await;

        // 結果を source_files に登録
        let mut source_files_guard = source_files.lock().await;
        for (file_path, source_file) in results.into_iter().flatten() {
            source_files_guard.insert(file_path, source_file);
        }
        drop(source_files_guard);

        // 翻訳ファイルをインデックス
        let translation_pattern = vec![settings.translation_files.file_pattern.clone()];
        let translation_files =
            Self::find_source_files(workspace_path, &translation_pattern, exclude_patterns)?;

        tracing::info!(translation_file_count = translation_files.len(), "Found translation files");

        let mut translations = Vec::new();
        for file_path in &translation_files {
            match load_translation_file(&db, file_path, &settings.key_separator) {
                Ok(translation) => {
                    tracing::debug!(
                        file_path = %file_path.display(),
                        language = translation.language(&db),
                        key_count = translation.keys(&db).len(),
                        "Loaded translation file"
                    );
                    translations.push(translation);
                }
                Err(e) => {
                    tracing::warn!(
                        file_path = %file_path.display(),
                        error = %e,
                        "Failed to load translation file"
                    );
                }
            }
        }

        tracing::info!("Workspace indexing complete");

        Ok(translations)
    }

    /// 単一ファイルをインデックス
    async fn index_file(
        &self,
        db: crate::db::I18nDatabaseImpl,
        file_path: &PathBuf,
    ) -> Option<(PathBuf, SourceFile)> {
        // ファイル内容を読み込み
        let content = match tokio::fs::read_to_string(file_path).await {
            Ok(content) => content,
            Err(e) => {
                tracing::warn!("Failed to read file {:?}: {}", file_path, e);
                return None; // ファイル読み込みエラーは警告として扱い、処理を続行
            }
        };

        // ファイルURIを作成
        let Ok(uri) = Url::from_file_path(file_path) else {
            tracing::warn!("Failed to create URI for file {:?}", file_path);
            return None;
        };

        // ファイル内容を解析してインデックスに追加
        self.update_file(&db, &uri, &content).map(|source_file| (file_path.clone(), source_file))
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
                    tracing::debug!(?err, "Failed to read directory entry");
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

    /// ファイル内容を更新
    ///
    /// Salsa を使ってファイルを解析し、キー使用箇所を抽出します。
    /// 新しい `SourceFile` を作成して返します。
    pub fn update_file(
        &self,
        db: &crate::db::I18nDatabaseImpl,
        uri: &Url,
        content: &str,
    ) -> Option<SourceFile> {
        use crate::input::source::ProgrammingLanguage;

        // ファイルの言語を推論
        let language = ProgrammingLanguage::from_uri(uri.as_str())?;

        // 新しい SourceFile を作成
        let source_file = SourceFile::new(db, uri.to_string(), content.to_string(), language);

        // analyze_source クエリを実行（Salsa が自動的にキャッシュ）
        let key_usages = crate::syntax::analyze_source(db, source_file);

        tracing::debug!(
            uri = %uri,
            usages_count = key_usages.len(),
            "Analyzed file"
        );

        Some(source_file)
    }
}
