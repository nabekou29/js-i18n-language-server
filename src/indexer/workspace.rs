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

#[derive(Clone, Debug)]
pub struct WorkspaceIndexer {
    indexing_completed: Arc<AtomicBool>,
    translations_indexed: Arc<AtomicBool>,
    translations_notify: Arc<Notify>,
}

impl Default for WorkspaceIndexer {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkspaceIndexer {
    #[must_use]
    pub fn new() -> Self {
        Self {
            indexing_completed: Arc::new(AtomicBool::new(false)),
            translations_indexed: Arc::new(AtomicBool::new(false)),
            translations_notify: Arc::new(Notify::new()),
        }
    }

    #[must_use]
    pub fn is_indexing_completed(&self) -> bool {
        self.indexing_completed.load(Ordering::Acquire)
    }

    #[must_use]
    pub fn is_translations_indexed(&self) -> bool {
        self.translations_indexed.load(Ordering::Acquire)
    }

    pub fn reset_indexing_state(&self) {
        tracing::info!("Resetting indexing state");
        self.indexing_completed.store(false, Ordering::Release);
        self.translations_indexed.store(false, Ordering::Release);
    }

    /// Returns 40% of CPU cores (minimum 1 thread).
    ///
    /// Based on ccls approach: limits parallelism for LSP servers to
    /// coexist with other processes in modern dev environments.
    #[must_use]
    fn default_num_threads() -> usize {
        // Use integer arithmetic to avoid floating-point imprecision
        let cpu_count = num_cpus::get();
        let num_threads = (cpu_count * 2) / 5;
        num_threads.max(1)
    }

    /// Wait for translation file indexing with timeout.
    ///
    /// Returns `true` if indexing completes within timeout, `false` otherwise.
    pub async fn wait_for_translations_indexed(&self, timeout: Duration) -> bool {
        if self.translations_indexed.load(Ordering::Acquire) {
            tracing::debug!("Translations already indexed");
            return true;
        }

        tokio::select! {
            () = self.translations_notify.notified() => {
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

    /// Index the workspace.
    ///
    /// Two-phase indexing: translations first (enables LSP features early),
    /// then source files with parallelism limit.
    ///
    /// # Errors
    /// Returns `IndexerError` if file discovery or pattern matching fails.
    #[allow(clippy::too_many_lines)]
    #[allow(clippy::significant_drop_tightening)] // Intentional: set flag while holding lock
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

        let max_concurrent_files =
            settings.indexing.num_threads.unwrap_or_else(Self::default_num_threads);

        tracing::info!(
            workspace_path = %workspace_path.display(),
            max_concurrent = max_concurrent_files,
            "Indexing workspace"
        );

        let Some(file_matcher) = config_manager.file_matcher() else {
            return Err(IndexerError::Error(
                "FileMatcher not available (workspace root not set?)".to_string(),
            ));
        };

        // Use workspace-aware methods to handle config_dir != workspace_path case
        let files = Self::find_files(workspace_path, |path| {
            file_matcher.is_source_file_from_workspace(workspace_path, path)
        });

        let translation_files = Self::find_files(workspace_path, |path| {
            file_matcher.is_translation_file_from_workspace(workspace_path, path)
        });

        #[allow(clippy::cast_possible_truncation)] // File count won't exceed u32::MAX
        let total_files = (files.len() + translation_files.len()) as u32;
        let processed_files = Arc::new(AtomicU32::new(0));
        let last_reported_percent = Arc::new(AtomicU32::new(0));

        // Report progress every 5% or 10 files
        let report_progress = Arc::new(move |current: u32| {
            if let Some(ref callback) = progress_callback {
                let current_percent =
                    if total_files > 0 { (current * 100) / total_files } else { 0 };
                let last_percent = last_reported_percent.load(Ordering::Relaxed);

                if current_percent >= last_percent + 5
                    || current.is_multiple_of(10)
                    || current == total_files
                {
                    callback(current, total_files);
                    last_reported_percent.store(current_percent, Ordering::Relaxed);
                }
            }
        });

        // Step 1: Index translation files first to enable LSP features early
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

        // Set flag while holding lock to guarantee data exists when flag is true
        {
            let mut guard = translations.lock().await;
            guard.extend(loaded_translations);
            self.translations_indexed.store(true, Ordering::Release);
        }
        self.translations_notify.notify_waiters();

        // Step 2: Index source files with parallelism limit
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

    #[tracing::instrument(skip(self, db, key_separator), fields(file_path = %file_path.display()))]
    async fn index_file(
        &self,
        db: crate::db::I18nDatabaseImpl,
        file_path: &PathBuf,
        key_separator: String,
    ) -> Option<(PathBuf, SourceFile)> {
        let content = match tokio::fs::read_to_string(file_path).await {
            Ok(content) => content,
            Err(e) => {
                tracing::warn!("Failed to read file {:?}: {}", file_path, e);
                return None;
            }
        };

        let Ok(uri) = Url::from_file_path(file_path) else {
            tracing::warn!("Failed to create URI for file {:?}", file_path);
            return None;
        };

        let path_clone = file_path.clone();

        // Run CPU-intensive parsing in blocking thread pool
        let result = tokio::task::spawn_blocking(move || {
            use crate::input::source::ProgrammingLanguage;

            let language = ProgrammingLanguage::from_uri(uri.as_str())?;
            let source_file = SourceFile::new(&db, uri.to_string(), content, language);
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

    /// Walk workspace and return files matching the filter.
    ///
    /// The filter receives paths relative to workspace root.
    fn find_files<F>(workspace_path: &Path, filter: F) -> Vec<PathBuf>
    where
        F: Fn(&Path) -> bool,
    {
        let mut found_files = Vec::new();

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

            if !entry.file_type().is_some_and(|ft| ft.is_file()) {
                continue;
            }

            let path = entry.path();

            let Ok(relative_path) = path.strip_prefix(workspace_path) else {
                continue;
            };

            if filter(relative_path) {
                found_files.push(path.to_path_buf());
            }
        }

        found_files
    }

    /// Parse file content and extract key usages.
    ///
    /// Returns a new `SourceFile` with analyzed key usages (Salsa caches automatically).
    pub fn update_file(
        &self,
        db: &crate::db::I18nDatabaseImpl,
        uri: &Url,
        content: &str,
        key_separator: &str,
    ) -> Option<SourceFile> {
        use crate::input::source::ProgrammingLanguage;

        let language = ProgrammingLanguage::from_uri(uri.as_str())?;
        let source_file = SourceFile::new(db, uri.to_string(), content.to_string(), language);
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
    use crate::config::FileMatcher;
    use crate::db::I18nDatabaseImpl;

    #[rstest]
    fn test_new_initial_state() {
        let indexer = WorkspaceIndexer::new();

        assert!(!indexer.is_indexing_completed());
        assert!(!indexer.is_translations_indexed());
    }

    #[rstest]
    fn test_default_trait() {
        let indexer = WorkspaceIndexer::default();

        assert!(!indexer.is_indexing_completed());
        assert!(!indexer.is_translations_indexed());
    }

    #[rstest]
    fn test_reset_indexing_state() {
        let indexer = WorkspaceIndexer::new();

        indexer.indexing_completed.store(true, Ordering::Release);
        indexer.translations_indexed.store(true, Ordering::Release);

        indexer.reset_indexing_state();

        assert!(!indexer.is_indexing_completed());
        assert!(!indexer.is_translations_indexed());
    }

    #[rstest]
    fn test_clone_shares_state() {
        let indexer1 = WorkspaceIndexer::new();
        let indexer2 = indexer1.clone();

        indexer1.indexing_completed.store(true, Ordering::Release);

        // Arc shares state between clones
        assert!(indexer2.is_indexing_completed());
    }

    #[rstest]
    fn test_default_num_threads_minimum() {
        let threads = WorkspaceIndexer::default_num_threads();

        assert!(threads >= 1, "Expected at least 1 thread, got {threads}");
    }

    #[rstest]
    fn test_default_num_threads_calculation() {
        let cpu_count = num_cpus::get();
        let threads = WorkspaceIndexer::default_num_threads();

        let expected = (cpu_count * 2) / 5;
        let expected = expected.max(1);

        assert_eq!(threads, expected, "CPU count: {cpu_count}");
    }

    #[rstest]
    fn test_default_num_threads_upper_bound() {
        let cpu_count = num_cpus::get();
        let threads = WorkspaceIndexer::default_num_threads();

        assert!(
            threads <= cpu_count,
            "Threads ({threads}) should not exceed CPU count ({cpu_count})"
        );
    }

    fn create_test_settings(
        include: &[&str],
        exclude: &[&str],
        translation: &str,
    ) -> crate::config::I18nSettings {
        crate::config::I18nSettings {
            include_patterns: include.iter().copied().map(String::from).collect(),
            exclude_patterns: exclude.iter().copied().map(String::from).collect(),
            translation_files: crate::config::TranslationFilesConfig {
                file_pattern: translation.to_string(),
            },
            ..crate::config::I18nSettings::default()
        }
    }

    #[rstest]
    fn test_find_files_include_pattern() {
        let temp_dir = TempDir::new().unwrap();

        fs::write(temp_dir.path().join("app.tsx"), "").unwrap();
        fs::write(temp_dir.path().join("index.ts"), "").unwrap();
        fs::write(temp_dir.path().join("readme.md"), "").unwrap();

        let settings = create_test_settings(&["**/*.tsx"], &[], "**/locales/**/*.json");
        let matcher = FileMatcher::new(temp_dir.path().to_path_buf(), &settings).unwrap();

        let files = WorkspaceIndexer::find_files(temp_dir.path(), |path| {
            matcher.is_source_file_relative(path)
        });

        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("app.tsx"));
    }

    #[rstest]
    fn test_find_files_multiple_include_patterns() {
        let temp_dir = TempDir::new().unwrap();

        fs::write(temp_dir.path().join("app.tsx"), "").unwrap();
        fs::write(temp_dir.path().join("index.ts"), "").unwrap();
        fs::write(temp_dir.path().join("readme.md"), "").unwrap();

        let settings = create_test_settings(&["**/*.tsx", "**/*.ts"], &[], "**/locales/**/*.json");
        let matcher = FileMatcher::new(temp_dir.path().to_path_buf(), &settings).unwrap();

        let files = WorkspaceIndexer::find_files(temp_dir.path(), |path| {
            matcher.is_source_file_relative(path)
        });

        assert_eq!(files.len(), 2);
    }

    #[rstest]
    fn test_find_files_exclude_pattern() {
        let temp_dir = TempDir::new().unwrap();

        fs::create_dir(temp_dir.path().join("node_modules")).unwrap();
        fs::write(temp_dir.path().join("node_modules/lib.tsx"), "").unwrap();
        fs::write(temp_dir.path().join("app.tsx"), "").unwrap();

        let settings =
            create_test_settings(&["**/*.tsx"], &["**/node_modules/**"], "**/locales/**/*.json");
        let matcher = FileMatcher::new(temp_dir.path().to_path_buf(), &settings).unwrap();

        let files = WorkspaceIndexer::find_files(temp_dir.path(), |path| {
            matcher.is_source_file_relative(path)
        });

        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("app.tsx"));
    }

    #[rstest]
    fn test_find_files_nested_directories() {
        let temp_dir = TempDir::new().unwrap();

        fs::create_dir_all(temp_dir.path().join("src/components")).unwrap();
        fs::write(temp_dir.path().join("src/index.tsx"), "").unwrap();
        fs::write(temp_dir.path().join("src/components/Button.tsx"), "").unwrap();

        let settings = create_test_settings(&["**/*.tsx"], &[], "**/locales/**/*.json");
        let matcher = FileMatcher::new(temp_dir.path().to_path_buf(), &settings).unwrap();

        let files = WorkspaceIndexer::find_files(temp_dir.path(), |path| {
            matcher.is_source_file_relative(path)
        });

        assert_eq!(files.len(), 2);
    }

    #[rstest]
    fn test_find_files_empty_directory() {
        let temp_dir = TempDir::new().unwrap();

        let settings = create_test_settings(&["**/*.tsx"], &[], "**/locales/**/*.json");
        let matcher = FileMatcher::new(temp_dir.path().to_path_buf(), &settings).unwrap();

        let files = WorkspaceIndexer::find_files(temp_dir.path(), |path| {
            matcher.is_source_file_relative(path)
        });

        assert!(files.is_empty());
    }

    #[rstest]
    fn test_find_files_include_and_exclude() {
        let temp_dir = TempDir::new().unwrap();

        fs::create_dir(temp_dir.path().join("src")).unwrap();
        fs::create_dir(temp_dir.path().join("test")).unwrap();
        fs::write(temp_dir.path().join("src/app.tsx"), "").unwrap();
        fs::write(temp_dir.path().join("test/app.test.tsx"), "").unwrap();

        let settings = create_test_settings(&["**/*.tsx"], &["**/test/**"], "**/locales/**/*.json");
        let matcher = FileMatcher::new(temp_dir.path().to_path_buf(), &settings).unwrap();

        let files = WorkspaceIndexer::find_files(temp_dir.path(), |path| {
            matcher.is_source_file_relative(path)
        });

        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("src/app.tsx"));
    }

    #[rstest]
    fn test_find_files_translation_files() {
        let temp_dir = TempDir::new().unwrap();

        fs::create_dir_all(temp_dir.path().join("locales")).unwrap();
        fs::write(temp_dir.path().join("locales/en.json"), "{}").unwrap();
        fs::write(temp_dir.path().join("locales/ja.json"), "{}").unwrap();
        fs::write(temp_dir.path().join("package.json"), "{}").unwrap();

        let settings = create_test_settings(&["**/*.tsx"], &[], "**/locales/**/*.json");
        let matcher = FileMatcher::new(temp_dir.path().to_path_buf(), &settings).unwrap();

        let files = WorkspaceIndexer::find_files(temp_dir.path(), |path| {
            matcher.is_translation_file_relative(path)
        });

        assert_eq!(files.len(), 2);
    }

    #[rstest]
    #[tokio::test]
    async fn test_wait_for_translations_indexed_already_done() {
        let indexer = WorkspaceIndexer::new();

        indexer.translations_indexed.store(true, Ordering::Release);

        let result = indexer.wait_for_translations_indexed(Duration::from_millis(100)).await;

        assert!(result);
    }

    #[rstest]
    #[tokio::test]
    async fn test_wait_for_translations_indexed_timeout() {
        let indexer = WorkspaceIndexer::new();

        let result = indexer.wait_for_translations_indexed(Duration::from_millis(10)).await;

        assert!(!result);
    }

    #[rstest]
    #[tokio::test]
    async fn test_wait_for_translations_indexed_notified() {
        let indexer = WorkspaceIndexer::new();
        let indexer_clone = indexer.clone();

        let handle = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            indexer_clone.translations_indexed.store(true, Ordering::Release);
            indexer_clone.translations_notify.notify_waiters();
        });

        let result = indexer.wait_for_translations_indexed(Duration::from_millis(1000)).await;

        handle.await.unwrap();
        assert!(result);
    }

    #[rstest]
    fn test_update_file_valid_typescript() {
        let indexer = WorkspaceIndexer::new();
        let db = I18nDatabaseImpl::default();
        let uri = Url::parse("file:///test/app.tsx").unwrap();
        let content = r#"const { t } = useTranslation(); t("hello");"#;

        let result = indexer.update_file(&db, &uri, content, ".");

        assert!(result.is_some());
    }

    #[rstest]
    fn test_update_file_invalid_extension() {
        let indexer = WorkspaceIndexer::new();
        let db = I18nDatabaseImpl::default();
        let uri = Url::parse("file:///test/readme.md").unwrap();
        let content = "# README";

        let result = indexer.update_file(&db, &uri, content, ".");

        assert!(result.is_none());
    }

    #[rstest]
    fn test_update_file_javascript() {
        let indexer = WorkspaceIndexer::new();
        let db = I18nDatabaseImpl::default();
        let uri = Url::parse("file:///test/app.js").unwrap();
        let content = r#"const { t } = useTranslation(); t("world");"#;

        let result = indexer.update_file(&db, &uri, content, ".");

        assert!(result.is_some());
    }

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
