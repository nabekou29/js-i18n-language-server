//! Workspace indexer implementation
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{
    AtomicBool,
    AtomicU32,
    Ordering,
};
use std::time::Duration;

use futures::stream::{
    self,
    StreamExt,
};
use globset::{
    Glob,
    GlobSetBuilder,
};
use ignore::WalkBuilder;
use tokio::sync::{
    Mutex,
    Notify,
};
use tower_lsp::lsp_types::Url;

use crate::config::ConfigManager;
use crate::indexer::types::IndexerError;
use crate::input::source::SourceFile;
use crate::input::translation::{
    Translation,
    load_translation_file,
};

/// Workspace indexer
#[derive(Clone, Debug)]
pub struct WorkspaceIndexer {
    /// グローバルインデックスが完了したかどうか
    indexing_completed: Arc<AtomicBool>,
    /// 翻訳ファイルのインデックスが完了したかどうか
    translations_indexed: Arc<AtomicBool>,
    /// 翻訳インデックス完了通知
    translations_notify: Arc<Notify>,
}

impl Default for WorkspaceIndexer {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkspaceIndexer {
    /// 新しいインデクサーを作成
    #[must_use]
    pub fn new() -> Self {
        Self {
            indexing_completed: Arc::new(AtomicBool::new(false)),
            translations_indexed: Arc::new(AtomicBool::new(false)),
            translations_notify: Arc::new(Notify::new()),
        }
    }

    /// インデックスが完了しているかチェック
    #[must_use]
    pub fn is_indexing_completed(&self) -> bool {
        self.indexing_completed.load(Ordering::Acquire)
    }

    /// 翻訳ファイルのインデックスが完了しているかチェック
    #[must_use]
    pub fn is_translations_indexed(&self) -> bool {
        self.translations_indexed.load(Ordering::Acquire)
    }

    /// インデックス状態をリセット
    ///
    /// ワークスペースの再インデックス時に呼び出され、
    /// 全ての状態フラグを初期化する。
    pub fn reset_indexing_state(&self) {
        tracing::info!("Resetting indexing state");
        self.indexing_completed.store(false, Ordering::Release);
        self.translations_indexed.store(false, Ordering::Release);
    }

    /// デフォルトのスレッド数を計算
    ///
    /// CPUコア数の80%を返す（最低1スレッド）。
    /// これはcclsなど他のLSP実装の例に従い、複数のLSPサーバーが
    /// 同時に起動する環境を考慮した設定。
    #[must_use]
    fn default_num_threads() -> usize {
        // CPUコア数の80% = (コア数 * 4) / 5
        // 浮動小数点演算を避けるため整数演算を使用
        let cpu_count = num_cpus::get();
        let num_threads = (cpu_count * 4) / 5;
        num_threads.max(1)
    }

    /// 翻訳ファイルのインデックス完了を待つ（タイムアウト付き）
    ///
    /// タイムアウト内にインデックスが完了すれば `true`、
    /// タイムアウトした場合は `false` を返す。
    ///
    /// # Arguments
    /// * `timeout` - 待機時間の上限
    pub async fn wait_for_translations_indexed(&self, timeout: Duration) -> bool {
        // すでに完了している場合は即座に返す
        if self.translations_indexed.load(Ordering::Acquire) {
            tracing::debug!("Translations already indexed");
            return true;
        }

        // タイムアウト付きで通知を待機
        tokio::select! {
            () = self.translations_notify.notified() => {
                // 通知を受け取ったので状態を確認
                let indexed = self.translations_indexed.load(Ordering::Acquire);
                tracing::debug!(indexed, "Translation index notification received");
                indexed
            }
            () = tokio::time::sleep(timeout) => {
                tracing::debug!("Timeout waiting for translations indexed");
                false
            }
        }
    }

    /// ワークスペースをインデックス
    ///
    /// 翻訳ファイルを優先的にインデックスした後、ソースファイルを並列度制限付きで処理する。
    /// これにより、LSP機能（Hover、Diagnostics）が早期に利用可能になる。
    ///
    /// # Errors
    /// ファイル検索やパターンのビルドに失敗した場合、`IndexerError` を返す。
    #[allow(clippy::too_many_lines)]
    #[allow(clippy::significant_drop_tightening)] // フラグ設定をロック保持中に行うため意図的
    #[tracing::instrument(
        skip(self, db, config_manager, source_files, translations, progress_callback),
        fields(workspace_path = %workspace_path.display())
    )]
    pub async fn index_workspace<F>(
        &self,
        db: crate::db::I18nDatabaseImpl,
        workspace_path: &Path,
        config_manager: &ConfigManager,
        source_files: Arc<Mutex<HashMap<PathBuf, SourceFile>>>,
        translations: Arc<Mutex<Vec<Translation>>>,
        progress_callback: Option<F>,
    ) -> Result<(), IndexerError>
    where
        F: Fn(u32, u32) + Send + Sync + 'static,
    {
        let settings = config_manager.get_settings();

        // CPU飢餓を防ぐため、同時に処理するソースファイルの最大数
        //
        // ユーザー設定がある場合はそれを使用し、ない場合はCPUコア数の80%をデフォルトとする。
        // これはcclsなど他のLSP実装の例に従い、複数のLSPサーバーが同時に起動する
        // 現代的な開発環境を考慮した設定。
        let max_concurrent_files =
            settings.indexing.num_threads.unwrap_or_else(Self::default_num_threads);

        tracing::info!(
            workspace_path = %workspace_path.display(),
            max_concurrent = max_concurrent_files,
            "Indexing workspace"
        );

        let include_patterns = &settings.include_patterns;
        let exclude_patterns = &settings.exclude_patterns;

        // ソースファイルをインデックス
        let files = Self::find_source_files(workspace_path, include_patterns, exclude_patterns)?;

        // 翻訳ファイルを検索（総ファイル数計算のため先に実行）
        let translation_pattern = vec![settings.translation_files.file_pattern.clone()];
        let translation_files =
            Self::find_source_files(workspace_path, &translation_pattern, exclude_patterns)?;

        // 総ファイル数と進捗カウンター
        #[allow(clippy::cast_possible_truncation)]
        // ワークスペース内のファイル数が42億を超えることはない
        let total_files = (files.len() + translation_files.len()) as u32;
        let processed_files = Arc::new(AtomicU32::new(0));
        let last_reported_percent = Arc::new(AtomicU32::new(0));

        // 進捗報告ヘルパー（5%刻みまたは10ファイルごと）
        let report_progress = Arc::new(move |current: u32| {
            if let Some(ref callback) = progress_callback {
                let current_percent =
                    if total_files > 0 { (current * 100) / total_files } else { 0 };
                let last_percent = last_reported_percent.load(Ordering::Relaxed);

                // 5%以上変化したか、10ファイルごと、または最後のファイル
                if current_percent >= last_percent + 5
                    || current.is_multiple_of(10)
                    || current == total_files
                {
                    callback(current, total_files);
                    last_reported_percent.store(current_percent, Ordering::Relaxed);
                }
            }
        });

        // 【ステップ1】翻訳ファイルを優先的にインデックス
        // LSP機能（Hover、Diagnostics）を早期に利用可能にするため、
        // 翻訳ファイルを先にインデックスして通知する
        let mut loaded_translations = Vec::new();
        for file_path in &translation_files {
            match load_translation_file(&db, file_path, &settings.key_separator) {
                Ok(translation) => {
                    tracing::debug!(
                        file_path = %file_path.display(),
                        language = translation.language(&db),
                        key_count = translation.keys(&db).len(),
                        "Loaded translation file"
                    );
                    loaded_translations.push(translation);
                }
                Err(e) => {
                    tracing::warn!(
                        file_path = %file_path.display(),
                        error = %e,
                        "Failed to load translation file"
                    );
                }
            }

            let current = processed_files.fetch_add(1, Ordering::Relaxed) + 1;
            report_progress(current);
        }

        // 翻訳データを保存してから通知（順序が重要）
        // フラグ設定をロック内で行うことで、フラグが true の時に
        // データが必ず存在することを保証する
        {
            let mut guard = translations.lock().await;
            guard.extend(loaded_translations);
            self.translations_indexed.store(true, Ordering::Release);
        }
        self.translations_notify.notify_waiters();

        // 【ステップ2】並列度を制限してソースファイルをインデックス
        // CPU飢餓を防ぐため、buffer_unordered で並列度を制限する

        let futures: Vec<_> = files
            .iter()
            .map(|file| {
                let db_clone = db.clone();
                let processed = Arc::clone(&processed_files);
                let report = Arc::clone(&report_progress);
                async move {
                    let result = self.index_file(db_clone, file).await;
                    let current = processed.fetch_add(1, Ordering::Relaxed) + 1;
                    report(current);
                    result
                }
            })
            .collect();

        let results: Vec<_> =
            stream::iter(futures).buffer_unordered(max_concurrent_files).collect().await;

        let mut source_files_guard = source_files.lock().await;
        for result in results.into_iter().flatten() {
            source_files_guard.insert(result.0, result.1);
        }
        drop(source_files_guard);

        self.indexing_completed.store(true, Ordering::Release);

        tracing::info!(
            translation_files = translation_files.len(),
            source_files = files.len(),
            "Indexing complete"
        );

        Ok(())
    }

    /// 単一ファイルをインデックス
    #[tracing::instrument(skip(self, db), fields(file_path = %file_path.display()))]
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

        let path_clone = file_path.clone();

        // CPU集約的な解析処理を専用スレッドプールで実行
        let result = tokio::task::spawn_blocking(move || {
            use crate::input::source::ProgrammingLanguage;

            // ファイルの言語を推論
            let language = ProgrammingLanguage::from_uri(uri.as_str())?;

            // SourceFile を作成して解析
            let source_file = SourceFile::new(&db, uri.to_string(), content, language);
            let key_usages = crate::syntax::analyze_source(&db, source_file);

            tracing::debug!(
                uri = %uri,
                usages_count = key_usages.len(),
                "Analyzed file"
            );

            Some((path_clone, source_file))
        })
        .await;

        result.ok().flatten()
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
