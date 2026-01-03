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
    /// CPUコア数の40%を返す（最低1スレッド）。
    /// これはcclsなど他のLSP実装の例に従い、複数のLSPサーバーが
    /// 同時に起動する環境を考慮した設定。
    #[must_use]
    fn default_num_threads() -> usize {
        // CPUコア数の40% = (コア数 * 2) / 5
        // 浮動小数点演算を避けるため整数演算を使用
        let cpu_count = num_cpus::get();
        let num_threads = (cpu_count * 2) / 5;
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

        let key_separator = settings.key_separator.clone();
        let futures: Vec<_> = files
            .iter()
            .map(|file| {
                let db_clone = db.clone();
                let processed = Arc::clone(&processed_files);
                let report = Arc::clone(&report_progress);
                let sep = key_separator.clone();
                async move {
                    let result = self.index_file(db_clone, file, sep).await;
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
    #[tracing::instrument(skip(self, db, key_separator), fields(file_path = %file_path.display()))]
    async fn index_file(
        &self,
        db: crate::db::I18nDatabaseImpl,
        file_path: &PathBuf,
        key_separator: String,
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

            // SourceFile を作成
            let source_file = SourceFile::new(&db, uri.to_string(), content, language);

            // analyze_source クエリを実行
            let key_usages = crate::syntax::analyze_source(&db, source_file, key_separator);

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
        key_separator: &str,
    ) -> Option<SourceFile> {
        use crate::input::source::ProgrammingLanguage;

        // ファイルの言語を推論
        let language = ProgrammingLanguage::from_uri(uri.as_str())?;

        // 新しい SourceFile を作成
        let source_file = SourceFile::new(db, uri.to_string(), content.to_string(), language);

        // analyze_source クエリを実行（Salsa が自動的にキャッシュ）
        let key_usages = crate::syntax::analyze_source(db, source_file, key_separator.to_string());

        tracing::debug!(
            uri = %uri,
            usages_count = key_usages.len(),
            "Analyzed file"
        );

        Some(source_file)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use std::fs;
    use std::time::Duration;

    use rstest::rstest;
    use tempfile::TempDir;
    use tower_lsp::lsp_types::Url;

    use super::*;
    use crate::db::I18nDatabaseImpl;

    // ========================================
    // 状態管理テスト
    // ========================================

    /// `new`: 初期状態は未完了
    #[rstest]
    fn test_new_initial_state() {
        let indexer = WorkspaceIndexer::new();

        assert!(!indexer.is_indexing_completed());
        assert!(!indexer.is_translations_indexed());
    }

    /// `Default` trait: `new()` と同じ初期状態
    #[rstest]
    fn test_default_trait() {
        let indexer = WorkspaceIndexer::default();

        assert!(!indexer.is_indexing_completed());
        assert!(!indexer.is_translations_indexed());
    }

    /// `reset_indexing_state`: フラグがリセットされる
    #[rstest]
    fn test_reset_indexing_state() {
        let indexer = WorkspaceIndexer::new();

        // 手動でフラグを true に設定
        indexer.indexing_completed.store(true, Ordering::Release);
        indexer.translations_indexed.store(true, Ordering::Release);

        // リセット
        indexer.reset_indexing_state();

        // 両方とも false に戻る
        assert!(!indexer.is_indexing_completed());
        assert!(!indexer.is_translations_indexed());
    }

    /// `Clone`: 状態を共有する
    #[rstest]
    fn test_clone_shares_state() {
        let indexer1 = WorkspaceIndexer::new();
        let indexer2 = indexer1.clone();

        // indexer1 でフラグを変更
        indexer1.indexing_completed.store(true, Ordering::Release);

        // indexer2 でも反映される（Arc で共有）
        assert!(indexer2.is_indexing_completed());
    }

    // ========================================
    // default_num_threads テスト
    // ========================================

    /// `default_num_threads`: 最低1スレッドを保証
    #[rstest]
    fn test_default_num_threads_minimum() {
        let threads = WorkspaceIndexer::default_num_threads();

        // 最低 1 スレッド
        assert!(threads >= 1, "Expected at least 1 thread, got {threads}");
    }

    /// `default_num_threads`: CPU コア数の 40% 程度
    #[rstest]
    fn test_default_num_threads_calculation() {
        let cpu_count = num_cpus::get();
        let threads = WorkspaceIndexer::default_num_threads();

        // 計算式: (cpu_count * 2) / 5 = cpu_count * 0.4
        let expected = (cpu_count * 2) / 5;
        let expected = expected.max(1);

        assert_eq!(threads, expected, "CPU count: {cpu_count}");
    }

    /// `default_num_threads`: CPU コア数以下
    #[rstest]
    fn test_default_num_threads_upper_bound() {
        let cpu_count = num_cpus::get();
        let threads = WorkspaceIndexer::default_num_threads();

        assert!(
            threads <= cpu_count,
            "Threads ({threads}) should not exceed CPU count ({cpu_count})"
        );
    }

    // ========================================
    // find_source_files テスト
    // ========================================

    /// `find_source_files`: include パターンでファイルを選択
    #[rstest]
    fn test_find_source_files_include_pattern() {
        let temp_dir = TempDir::new().unwrap();

        // テストファイルを作成
        fs::write(temp_dir.path().join("app.tsx"), "").unwrap();
        fs::write(temp_dir.path().join("index.ts"), "").unwrap();
        fs::write(temp_dir.path().join("readme.md"), "").unwrap();

        let files =
            WorkspaceIndexer::find_source_files(temp_dir.path(), &["**/*.tsx".to_string()], &[])
                .unwrap();

        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("app.tsx"));
    }

    /// `find_source_files`: 複数の include パターン
    #[rstest]
    fn test_find_source_files_multiple_include_patterns() {
        let temp_dir = TempDir::new().unwrap();

        fs::write(temp_dir.path().join("app.tsx"), "").unwrap();
        fs::write(temp_dir.path().join("index.ts"), "").unwrap();
        fs::write(temp_dir.path().join("readme.md"), "").unwrap();

        let files = WorkspaceIndexer::find_source_files(
            temp_dir.path(),
            &["**/*.tsx".to_string(), "**/*.ts".to_string()],
            &[],
        )
        .unwrap();

        assert_eq!(files.len(), 2);
    }

    /// `find_source_files`: exclude パターンでファイルを除外
    #[rstest]
    fn test_find_source_files_exclude_pattern() {
        let temp_dir = TempDir::new().unwrap();

        // node_modules ディレクトリを作成
        fs::create_dir(temp_dir.path().join("node_modules")).unwrap();
        fs::write(temp_dir.path().join("node_modules/lib.tsx"), "").unwrap();
        fs::write(temp_dir.path().join("app.tsx"), "").unwrap();

        let files = WorkspaceIndexer::find_source_files(
            temp_dir.path(),
            &["**/*.tsx".to_string()],
            &["**/node_modules/**".to_string()],
        )
        .unwrap();

        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("app.tsx"));
    }

    /// `find_source_files`: ネストしたディレクトリ
    #[rstest]
    fn test_find_source_files_nested_directories() {
        let temp_dir = TempDir::new().unwrap();

        // ネストしたディレクトリ構造を作成
        fs::create_dir_all(temp_dir.path().join("src/components")).unwrap();
        fs::write(temp_dir.path().join("src/index.tsx"), "").unwrap();
        fs::write(temp_dir.path().join("src/components/Button.tsx"), "").unwrap();

        let files =
            WorkspaceIndexer::find_source_files(temp_dir.path(), &["**/*.tsx".to_string()], &[])
                .unwrap();

        assert_eq!(files.len(), 2);
    }

    /// `find_source_files`: 空のディレクトリ
    #[rstest]
    fn test_find_source_files_empty_directory() {
        let temp_dir = TempDir::new().unwrap();

        let files =
            WorkspaceIndexer::find_source_files(temp_dir.path(), &["**/*.tsx".to_string()], &[])
                .unwrap();

        assert!(files.is_empty());
    }

    /// `find_source_files`: 無効な include パターンでエラー
    #[rstest]
    fn test_find_source_files_invalid_include_pattern() {
        let temp_dir = TempDir::new().unwrap();

        let result = WorkspaceIndexer::find_source_files(
            temp_dir.path(),
            &["[invalid".to_string()], // 不正な glob パターン
            &[],
        );

        assert!(result.is_err());
    }

    /// `find_source_files`: 無効な exclude パターンでエラー
    #[rstest]
    fn test_find_source_files_invalid_exclude_pattern() {
        let temp_dir = TempDir::new().unwrap();

        let result = WorkspaceIndexer::find_source_files(
            temp_dir.path(),
            &["**/*.tsx".to_string()],
            &["[invalid".to_string()], // 不正な glob パターン
        );

        assert!(result.is_err());
    }

    /// `find_source_files`: include と exclude の両方が適用される
    #[rstest]
    fn test_find_source_files_include_and_exclude() {
        let temp_dir = TempDir::new().unwrap();

        fs::create_dir(temp_dir.path().join("src")).unwrap();
        fs::create_dir(temp_dir.path().join("test")).unwrap();
        fs::write(temp_dir.path().join("src/app.tsx"), "").unwrap();
        fs::write(temp_dir.path().join("test/app.test.tsx"), "").unwrap();

        let files = WorkspaceIndexer::find_source_files(
            temp_dir.path(),
            &["**/*.tsx".to_string()],
            &["**/test/**".to_string()],
        )
        .unwrap();

        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("src/app.tsx"));
    }

    // ========================================
    // wait_for_translations_indexed テスト
    // ========================================

    /// `wait_for_translations_indexed`: すでに完了している場合は即座に true
    #[rstest]
    #[tokio::test]
    async fn test_wait_for_translations_indexed_already_done() {
        let indexer = WorkspaceIndexer::new();

        // すでに完了状態に設定
        indexer.translations_indexed.store(true, Ordering::Release);

        let result = indexer.wait_for_translations_indexed(Duration::from_millis(100)).await;

        assert!(result);
    }

    /// `wait_for_translations_indexed`: タイムアウト時は false
    #[rstest]
    #[tokio::test]
    async fn test_wait_for_translations_indexed_timeout() {
        let indexer = WorkspaceIndexer::new();

        // 短いタイムアウトで待機（完了しないので false）
        let result = indexer.wait_for_translations_indexed(Duration::from_millis(10)).await;

        assert!(!result);
    }

    /// `wait_for_translations_indexed`: 通知を受け取ると状態を返す
    #[rstest]
    #[tokio::test]
    async fn test_wait_for_translations_indexed_notified() {
        let indexer = WorkspaceIndexer::new();
        let indexer_clone = indexer.clone();

        // 別タスクで通知を送信
        let handle = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            indexer_clone.translations_indexed.store(true, Ordering::Release);
            indexer_clone.translations_notify.notify_waiters();
        });

        let result = indexer.wait_for_translations_indexed(Duration::from_millis(1000)).await;

        handle.await.unwrap();
        assert!(result);
    }

    // ========================================
    // update_file テスト
    // ========================================

    /// `update_file`: 有効な TypeScript ファイル
    #[rstest]
    fn test_update_file_valid_typescript() {
        let indexer = WorkspaceIndexer::new();
        let db = I18nDatabaseImpl::default();
        let uri = Url::parse("file:///test/app.tsx").unwrap();
        let content = r#"const { t } = useTranslation(); t("hello");"#;

        let result = indexer.update_file(&db, &uri, content, ".");

        assert!(result.is_some());
    }

    /// `update_file`: 無効な拡張子は None
    #[rstest]
    fn test_update_file_invalid_extension() {
        let indexer = WorkspaceIndexer::new();
        let db = I18nDatabaseImpl::default();
        let uri = Url::parse("file:///test/readme.md").unwrap();
        let content = "# README";

        let result = indexer.update_file(&db, &uri, content, ".");

        assert!(result.is_none());
    }

    /// `update_file`: JavaScript ファイルも処理可能
    #[rstest]
    fn test_update_file_javascript() {
        let indexer = WorkspaceIndexer::new();
        let db = I18nDatabaseImpl::default();
        let uri = Url::parse("file:///test/app.js").unwrap();
        let content = r#"const { t } = useTranslation(); t("world");"#;

        let result = indexer.update_file(&db, &uri, content, ".");

        assert!(result.is_some());
    }

    /// `update_file`: 空のファイルも処理可能
    #[rstest]
    fn test_update_file_empty_content() {
        let indexer = WorkspaceIndexer::new();
        let db = I18nDatabaseImpl::default();
        let uri = Url::parse("file:///test/empty.tsx").unwrap();
        let content = "";

        let result = indexer.update_file(&db, &uri, content, ".");

        assert!(result.is_some());
    }
}
