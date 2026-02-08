//! LSP Backend implementation

use std::path::{
    Path,
    PathBuf,
};
use std::sync::Arc;
use std::time::Duration;

/// Timeout for waiting translation index completion.
pub(crate) const TRANSLATIONS_INDEX_TIMEOUT: Duration = Duration::from_millis(500);

use tokio::sync::Mutex;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::{
    DidChangeWatchedFilesRegistrationOptions,
    FileChangeType,
    FileSystemWatcher,
    GlobPattern,
    MessageType,
    NumberOrString,
    ProgressParams,
    ProgressParamsValue,
    Registration,
    WatchKind,
    WorkDoneProgress,
    WorkDoneProgressBegin,
    WorkDoneProgressEnd,
    WorkDoneProgressReport,
    WorkspaceFolder,
    notification::{
        Notification,
        Progress,
    },
};

/// Custom notification sent when decorations need refreshing.
pub(crate) struct DecorationsChanged;

impl Notification for DecorationsChanged {
    type Params = ();
    const METHOD: &'static str = "i18n/decorationsChanged";
}
use tower_lsp::{
    Client,
    LanguageServer,
};

use super::handlers;
use super::state::ServerState;
use crate::config::ConfigManager;
use crate::db::I18nDatabaseImpl;
use crate::indexer::workspace::WorkspaceIndexer;

/// LSP Backend
#[derive(Clone)]
pub struct Backend {
    pub client: Client,
    pub config_manager: Arc<Mutex<ConfigManager>>,
    pub workspace_indexer: Arc<WorkspaceIndexer>,
    /// Shared state: `db`, `source_files`, `translations`, `opened_files`
    pub state: ServerState,
}

impl std::fmt::Debug for Backend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Backend")
            .field("config_manager", &"<ConfigManager>")
            .field("workspace_indexer", &"<WorkspaceIndexer>")
            .field("state", &self.state)
            .finish_non_exhaustive()
    }
}

impl Backend {
    /// Converts URI to file path. Returns `None` with warning log on failure.
    pub(crate) fn uri_to_path(uri: &tower_lsp::lsp_types::Url) -> Option<PathBuf> {
        uri.to_file_path().ok().or_else(|| {
            tracing::warn!("Failed to convert URI to file path: {}", uri);
            None
        })
    }

    /// Waits for translation index completion with timeout.
    pub(crate) async fn wait_for_translations(&self) -> bool {
        self.workspace_indexer.wait_for_translations_indexed(TRANSLATIONS_INDEX_TIMEOUT).await
    }

    fn create_diagnostic_options(config: &ConfigManager) -> super::diagnostics::DiagnosticOptions {
        let settings = config.get_settings();
        let mt = &settings.diagnostics.missing_translation;
        super::diagnostics::DiagnosticOptions {
            enabled: mt.enabled,
            severity: mt.severity,
            required_languages: mt.required_languages.as_ref().map(|v| v.iter().cloned().collect()),
            optional_languages: mt.optional_languages.as_ref().map(|v| v.iter().cloned().collect()),
        }
    }

    async fn get_diagnostic_config(&self) -> (super::diagnostics::DiagnosticOptions, String) {
        let config = self.config_manager.lock().await;
        (Self::create_diagnostic_options(&config), config.get_settings().key_separator.clone())
    }

    /// Resets state and initializes index. Creates new Salsa database to clear old cache.
    async fn reset_state(&self) {
        *self.state.db.lock().await = I18nDatabaseImpl::default();
        self.state.source_files.lock().await.clear();
        self.state.translations.lock().await.clear();
        self.workspace_indexer.reset_indexing_state();
    }

    async fn send_progress_begin(&self, token: &NumberOrString, title: &str, message: &str) {
        self.client
            .send_notification::<Progress>(ProgressParams {
                token: token.clone(),
                value: ProgressParamsValue::WorkDone(WorkDoneProgress::Begin(
                    WorkDoneProgressBegin {
                        title: title.to_string(),
                        cancellable: Some(false),
                        message: Some(message.to_string()),
                        percentage: Some(0),
                    },
                )),
            })
            .await;
    }

    async fn send_progress_end(&self, token: &NumberOrString, message: &str) {
        self.client
            .send_notification::<Progress>(ProgressParams {
                token: token.clone(),
                value: ProgressParamsValue::WorkDone(WorkDoneProgress::End(WorkDoneProgressEnd {
                    message: Some(message.to_string()),
                })),
            })
            .await;
    }

    /// Returns the configured key separator (e.g. `"."`).
    pub(crate) async fn get_key_separator(&self) -> String {
        self.config_manager.lock().await.get_settings().key_separator.clone()
    }

    /// Collects all translation keys referenced in source files.
    pub(crate) async fn collect_used_keys(
        &self,
        key_separator: &str,
    ) -> std::collections::HashSet<String> {
        let db = self.state.db.lock().await;
        let source_files = self.state.source_files.lock().await;
        let source_file_vec: Vec<_> = source_files.values().copied().collect();
        drop(source_files);

        let mut keys = std::collections::HashSet::new();
        for source_file in source_file_vec {
            let key_usages =
                crate::syntax::analyze_source(&*db, source_file, key_separator.to_owned());
            for usage in key_usages {
                keys.insert(usage.key(&*db).text(&*db).clone());
            }
        }
        keys
    }

    /// Notifies the client that decorations should be refreshed.
    pub(crate) async fn send_decorations_changed(&self) {
        self.client.send_notification::<DecorationsChanged>(()).await;
    }

    /// Gets translation key text at cursor position from `SourceFile` or `Translation`.
    pub(crate) async fn get_key_at_position(
        &self,
        file_path: &Path,
        position: crate::types::SourcePosition,
    ) -> Option<String> {
        let source_file = {
            let source_files = self.state.source_files.lock().await;
            source_files.get(file_path).copied()
        };

        let key_separator = self.get_key_separator().await;
        let db = self.state.db.lock().await;

        if let Some(source_file) = source_file {
            crate::syntax::key_at_position(&*db, source_file, position, key_separator)
                .map(|key| key.text(&*db).clone())
        } else {
            tracing::debug!("Source file not found, trying Translation: {}", file_path.display());

            let translations = self.state.translations.lock().await;
            let file_path_str = file_path.to_string_lossy();

            let result = translations
                .iter()
                .find(|t| t.file_path(&*db) == file_path_str.as_ref())
                .and_then(|t| t.key_at_position(&*db, position).map(|key| key.text(&*db).clone()));
            drop(translations);
            result
        }
    }

    /// Sends diagnostics to all opened files.
    #[tracing::instrument(skip(self))]
    pub(crate) async fn send_diagnostics_to_opened_files(&self) {
        use crate::input::source::ProgrammingLanguage;

        let opened_files = self.state.opened_files.lock().await;
        let file_count = opened_files.len();

        tracing::info!(file_count, "Sending diagnostics to opened files");

        for uri in opened_files.iter() {
            let Some(file_path) = Self::uri_to_path(uri) else {
                continue;
            };

            if ProgrammingLanguage::from_uri(uri.as_str()).is_none() {
                tracing::debug!("Skipping diagnostics for unsupported file type: {}", uri);
                continue;
            }

            let source_file = {
                let source_files = self.state.source_files.lock().await;
                source_files.get(&file_path).copied()
            };

            let Some(source_file) = source_file else {
                tracing::debug!("Source file not found: {}", file_path.display());
                continue;
            };

            let diagnostics = {
                let (options, key_separator) = self.get_diagnostic_config().await;
                let db = self.state.db.lock().await;
                let translations = self.state.translations.lock().await;
                crate::ide::diagnostics::generate_diagnostics(
                    &*db,
                    source_file,
                    &translations,
                    &options,
                    &key_separator,
                )
            };

            self.client.publish_diagnostics(uri.clone(), diagnostics, None).await;
            tracing::debug!(uri = %uri, "Diagnostics sent");
        }
    }

    /// Sends unused key diagnostics to translation files.
    #[tracing::instrument(skip(self))]
    pub(crate) async fn send_unused_key_diagnostics(&self) {
        let settings = self.config_manager.lock().await.get_settings().clone();

        if !settings.diagnostics.unused_translation.enabled {
            tracing::debug!("Unused translation diagnostics disabled, skipping");
            return;
        }

        let key_separator = &settings.key_separator;

        // Collect data before releasing lock
        let source_file_vec: Vec<crate::input::source::SourceFile> =
            self.state.source_files.lock().await.values().copied().collect();

        let diagnostics_to_send: Vec<(String, Vec<tower_lsp::lsp_types::Diagnostic>)> = {
            let translations = self.state.translations.lock().await;
            let db = self.state.db.lock().await;

            tracing::info!(
                translation_count = translations.len(),
                source_file_count = source_file_vec.len(),
                "Sending unused key diagnostics"
            );

            translations
                .iter()
                .map(|translation| {
                    let diagnostics = crate::ide::diagnostics::generate_unused_key_diagnostics(
                        &*db,
                        *translation,
                        &source_file_vec,
                        key_separator,
                        &settings.diagnostics.unused_translation.ignore_patterns,
                        settings.diagnostics.unused_translation.severity,
                    );
                    let file_path = translation.file_path(&*db).clone();
                    (file_path, diagnostics)
                })
                .collect()
        };

        for (file_path, diagnostics) in diagnostics_to_send {
            if let Ok(uri) = tower_lsp::lsp_types::Url::from_file_path(&file_path) {
                self.client.publish_diagnostics(uri, diagnostics, None).await;
                tracing::debug!(file_path = %file_path, "Unused key diagnostics sent");
            } else {
                tracing::warn!(file_path = %file_path, "Failed to convert file path to URI");
            }
        }
    }

    /// Updates or creates source file and generates diagnostics.
    ///
    /// # Arguments
    /// * `force_create` - If true, ignores existing `SourceFile` and creates new one
    #[tracing::instrument(skip(self, text), fields(uri = %uri))]
    pub(crate) async fn update_and_diagnose(
        &self,
        uri: tower_lsp::lsp_types::Url,
        text: String,
        force_create: bool,
    ) {
        use salsa::Setter;

        use crate::input::source::{
            ProgrammingLanguage,
            SourceFile,
        };

        tracing::info!(uri = %uri, force_create, "Updating source file and diagnosing");

        let Some(file_path) = Self::uri_to_path(&uri) else {
            return;
        };

        let Some(language) = ProgrammingLanguage::from_uri(uri.as_str()) else {
            tracing::debug!("Skipping SourceFile creation for unsupported file type: {}", uri);
            return;
        };

        if !self.is_source_file(&file_path).await {
            tracing::debug!(
                "Skipping SourceFile creation for file not matching includePatterns: {}",
                file_path.display()
            );
            return;
        }

        // Queue update during indexing to avoid Salsa deadlock.
        // Salsa setters (set_text) conflict with queries in spawn_blocking.
        // Processed by process_pending_updates after indexing completes.
        if !self.workspace_indexer.is_indexing_completed() {
            tracing::debug!(
                uri = %uri,
                "Queueing SourceFile update during indexing to avoid Salsa lock contention"
            );
            self.state.pending_updates.lock().await.push((uri, text, force_create));
            return;
        }

        // Update SourceFile without holding source_files lock during Salsa operations
        let source_file = {
            let existing = if force_create {
                None
            } else {
                let source_files = self.state.source_files.lock().await;
                source_files.get(&file_path).copied()
            };

            if let Some(existing) = existing {
                let mut db = self.state.db.lock().await;
                existing.set_text(&mut *db).to(text);
                existing
            } else {
                let db = self.state.db.lock().await;
                let source_file = SourceFile::new(&*db, uri.to_string(), text, language);
                drop(db);

                let mut source_files = self.state.source_files.lock().await;
                source_files.insert(file_path.clone(), source_file);
                source_file
            }
        };

        tracing::debug!(uri = %uri, "Source file updated");

        if !self.wait_for_translations().await {
            tracing::debug!(uri = %uri, "Skipping diagnostics - translations not indexed yet");
            return;
        }

        tracing::debug!(uri = %uri, "Generating diagnostics");

        let diagnostics = {
            let (options, key_separator) = self.get_diagnostic_config().await;
            let db = self.state.db.lock().await;
            let translations = self.state.translations.lock().await;
            crate::ide::diagnostics::generate_diagnostics(
                &*db,
                source_file,
                &translations,
                &options,
                &key_separator,
            )
        };

        self.client.publish_diagnostics(uri.clone(), diagnostics, None).await;
        tracing::debug!(uri = %uri, "Diagnostics generated and sent");

        self.send_unused_key_diagnostics().await;
    }

    /// Returns workspace folders stored during `initialize`.
    pub(crate) async fn get_workspace_folders(&self) -> Vec<WorkspaceFolder> {
        self.state.workspace_folders.lock().await.clone()
    }

    /// Reindexes workspace with new Salsa database and clears old cache.
    #[tracing::instrument(skip(self))]
    pub(crate) async fn reindex_workspace(&self) {
        tracing::info!("Starting workspace reindex");

        self.reset_state().await;

        for folder in self.get_workspace_folders().await {
            if let Ok(workspace_path) = folder.uri.to_file_path() {
                let token = NumberOrString::String("workspace-reindexing".to_string());

                self.send_progress_begin(
                    &token,
                    "Reindexing Workspace",
                    "Configuration changed, reindexing...",
                )
                .await;

                let config_manager = self.config_manager.lock().await;
                let db = self.state.db.lock().await.clone();
                let source_files = self.state.source_files.clone();

                let (progress_tx, mut progress_rx) = tokio::sync::mpsc::channel::<(u32, u32)>(100);

                let progress_task = {
                    let client = self.client.clone();
                    let token = token.clone();
                    tokio::spawn(async move {
                        while let Some((current, total)) = progress_rx.recv().await {
                            let percentage = (current * 100).checked_div(total).unwrap_or(0);
                            client
                                .send_notification::<Progress>(ProgressParams {
                                    token: token.clone(),
                                    value: ProgressParamsValue::WorkDone(WorkDoneProgress::Report(
                                        WorkDoneProgressReport {
                                            cancellable: Some(false),
                                            message: Some(format!(
                                                "Processing files: {current}/{total}"
                                            )),
                                            percentage: Some(percentage),
                                        },
                                    )),
                                })
                                .await;
                        }
                    })
                };

                let progress_callback = move |current: u32, total: u32| {
                    let _ = progress_tx.try_send((current, total));
                };

                let index_result = self
                    .workspace_indexer
                    .index_workspace(
                        db,
                        &workspace_path,
                        &config_manager,
                        source_files,
                        self.state.translations.clone(),
                        Some(progress_callback),
                    )
                    .await;

                drop(config_manager);

                let _ = progress_task.await;

                match index_result {
                    Ok(()) => {
                        self.send_progress_end(&token, "Reindexing complete").await;
                        self.send_decorations_changed().await;
                        tracing::info!("Workspace reindex complete");
                    }
                    Err(error) => {
                        self.send_progress_end(&token, &format!("Reindexing failed: {error}"))
                            .await;
                        tracing::error!("Workspace reindex failed: {}", error);
                    }
                }
            }
        }
    }

    /// Reloads translation file and updates translations.
    #[tracing::instrument(skip(self), fields(file_path = %file_path.display()))]
    pub(crate) async fn reload_translation_file(&self, file_path: &Path) {
        let config_manager = self.config_manager.lock().await;
        let key_separator = config_manager.get_settings().key_separator.clone();
        drop(config_manager);

        let db = self.state.db.lock().await;

        match crate::input::translation::load_translation_file(&*db, file_path, &key_separator) {
            Ok(new_translation) => {
                let mut translations = self.state.translations.lock().await;

                let file_path_str = file_path.to_string_lossy().to_string();
                translations.retain(|t| t.file_path(&*db) != &file_path_str);

                translations.push(new_translation);
                drop(translations);

                tracing::info!("Reloaded translation file: {:?}", file_path);
            }
            Err(e) => {
                tracing::warn!("Failed to reload translation file {:?}: {}", file_path, e);
            }
        }
    }

    /// Updates translation from buffer content (for unsaved changes).
    #[tracing::instrument(skip(self, content), fields(file_path = %file_path.display()))]
    pub(crate) async fn update_translation_from_content(&self, file_path: &Path, content: &str) {
        let config_manager = self.config_manager.lock().await;
        let key_separator = config_manager.get_settings().key_separator.clone();
        drop(config_manager);

        let db = self.state.db.lock().await;

        match crate::input::translation::load_translation_from_content(
            &*db,
            file_path,
            content,
            &key_separator,
        ) {
            Ok(new_translation) => {
                let mut translations = self.state.translations.lock().await;

                let file_path_str = file_path.to_string_lossy().to_string();
                translations.retain(|t| t.file_path(&*db) != &file_path_str);

                translations.push(new_translation);
                drop(translations);

                tracing::info!("Updated translation from buffer: {:?}", file_path);
            }
            Err(e) => {
                tracing::warn!("Failed to update translation {:?}: {}", file_path, e);
            }
        }
    }

    /// Removes translation entry for the specified file.
    #[tracing::instrument(skip(self), fields(file_path = %file_path.display()))]
    pub(crate) async fn remove_translation_file(&self, file_path: &Path) {
        let db = self.state.db.lock().await;
        let mut translations = self.state.translations.lock().await;

        let file_path_str = file_path.to_string_lossy().to_string();
        let before_len = translations.len();
        translations.retain(|t| t.file_path(&*db) != &file_path_str);

        if translations.len() < before_len {
            tracing::info!("Removed translation file: {:?}", file_path);
        }
    }

    /// Registers file watchers for config and translation files.
    pub(crate) async fn register_file_watchers(&self) {
        let translation_patterns = {
            let config_manager = self.config_manager.lock().await;
            config_manager.get_settings().translation_files.include_patterns.clone()
        };

        let mut watchers = vec![FileSystemWatcher {
            glob_pattern: GlobPattern::String("**/.js-i18n.json".to_string()),
            kind: Some(WatchKind::all()),
        }];
        for pattern in &translation_patterns {
            watchers.push(FileSystemWatcher {
                glob_pattern: GlobPattern::String(pattern.clone()),
                kind: Some(WatchKind::all()),
            });
        }

        let Ok(register_options) =
            serde_json::to_value(DidChangeWatchedFilesRegistrationOptions { watchers })
        else {
            tracing::warn!("Failed to serialize file watcher options");
            return;
        };

        let registration = Registration {
            id: "watch-files".to_string(),
            method: "workspace/didChangeWatchedFiles".to_string(),
            register_options: Some(register_options),
        };

        tracing::debug!(
            patterns = ?translation_patterns,
            "Registering file watcher for translation files"
        );
        if let Err(e) = self.client.register_capability(vec![registration]).await {
            tracing::warn!("Failed to register file watcher: {}", e);
        }
    }

    pub(crate) fn is_config_file(file_path: &Path) -> bool {
        file_path.file_name().is_some_and(|name| name == ".js-i18n.json")
    }

    pub(crate) async fn is_translation_file(&self, file_path: &Path) -> bool {
        let config_manager = self.config_manager.lock().await;
        config_manager.file_matcher().is_some_and(|matcher| matcher.is_translation_file(file_path))
    }

    /// Checks if file matches `includePatterns` and not `excludePatterns`.
    pub(crate) async fn is_source_file(&self, file_path: &Path) -> bool {
        let config_manager = self.config_manager.lock().await;
        config_manager.file_matcher().is_some_and(|matcher| matcher.is_source_file(file_path))
    }

    /// Processes pending updates queued during indexing.
    #[tracing::instrument(skip(self))]
    pub(crate) async fn process_pending_updates(&self) {
        let pending_updates = {
            let mut pending = self.state.pending_updates.lock().await;
            std::mem::take(&mut *pending)
        };

        if pending_updates.is_empty() {
            tracing::debug!("No pending updates to process");
            return;
        }

        tracing::info!(count = pending_updates.len(), "Processing pending updates");

        for (uri, text, force_create) in pending_updates {
            tracing::debug!(uri = %uri, "Processing pending update");
            self.update_and_diagnose(uri, text, force_create).await;
        }

        tracing::info!("Pending updates processed");
    }

    /// Handles config file changes (create/modify/delete).
    ///
    /// 1. Reloads config (or resets to default on delete)
    /// 2. Re-registers file watchers if pattern changed
    /// 3. Reindexes workspace
    /// 4. Updates diagnostics
    pub(crate) async fn handle_config_file_change(
        &self,
        file_path: &Path,
        change_type: FileChangeType,
    ) {
        tracing::info!("Config file changed: {:?}, type: {:?}", file_path, change_type);

        let workspace_root = file_path.parent().map(Path::to_path_buf);

        let old_patterns = {
            let config_manager = self.config_manager.lock().await;
            config_manager.get_settings().translation_files.include_patterns.clone()
        };

        match change_type {
            FileChangeType::CREATED | FileChangeType::CHANGED => {
                let mut config_manager = self.config_manager.lock().await;
                match config_manager.load_settings(workspace_root) {
                    Ok(()) => {
                        self.client
                            .log_message(MessageType::INFO, "Configuration reloaded successfully")
                            .await;
                        tracing::info!("Configuration reloaded successfully");
                    }
                    Err(error) => {
                        self.client
                            .log_message(
                                MessageType::ERROR,
                                format!("Failed to reload configuration: {error}"),
                            )
                            .await;
                        tracing::error!("Failed to reload configuration: {}", error);
                        return;
                    }
                }
            }
            FileChangeType::DELETED => {
                let mut config_manager = self.config_manager.lock().await;
                match config_manager.update_settings(crate::config::I18nSettings::default()) {
                    Ok(()) => {
                        self.client
                            .log_message(
                                MessageType::INFO,
                                "Configuration file deleted, using default settings",
                            )
                            .await;
                        tracing::info!("Configuration reset to defaults");
                    }
                    Err(error) => {
                        self.client
                            .log_message(
                                MessageType::ERROR,
                                format!("Failed to reset configuration: {error}"),
                            )
                            .await;
                        tracing::error!("Failed to reset configuration: {}", error);
                        return;
                    }
                }
            }
            _ => {
                tracing::warn!("Unknown file change type: {:?}", change_type);
                return;
            }
        }

        let new_patterns = {
            let config_manager = self.config_manager.lock().await;
            config_manager.get_settings().translation_files.include_patterns.clone()
        };

        if old_patterns != new_patterns {
            tracing::info!(
                "Translation file patterns changed: {:?} -> {:?}, re-registering watchers",
                old_patterns,
                new_patterns
            );
            self.register_file_watchers().await;
        }

        self.reindex_workspace().await;

        self.send_diagnostics_to_opened_files().await;
        self.send_unused_key_diagnostics().await;
    }
}

// =============================================================================
// LanguageServer Trait Implementation
// =============================================================================
// Each method delegates to corresponding handler function.

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    // -------------------------------------------------------------------------
    // Lifecycle
    // -------------------------------------------------------------------------

    async fn initialize(
        &self,
        params: tower_lsp::lsp_types::InitializeParams,
    ) -> Result<tower_lsp::lsp_types::InitializeResult> {
        handlers::lifecycle::handle_initialize(self, params).await
    }

    async fn initialized(&self, params: tower_lsp::lsp_types::InitializedParams) {
        handlers::lifecycle::handle_initialized(self, params).await;
    }

    async fn shutdown(&self) -> Result<()> {
        handlers::lifecycle::handle_shutdown().await
    }

    // -------------------------------------------------------------------------
    // Document Sync
    // -------------------------------------------------------------------------

    async fn did_open(&self, params: tower_lsp::lsp_types::DidOpenTextDocumentParams) {
        handlers::document_sync::handle_did_open(self, params).await;
    }

    async fn did_change(&self, params: tower_lsp::lsp_types::DidChangeTextDocumentParams) {
        handlers::document_sync::handle_did_change(self, params).await;
    }

    async fn did_save(&self, params: tower_lsp::lsp_types::DidSaveTextDocumentParams) {
        handlers::document_sync::handle_did_save(self, params).await;
    }

    async fn did_close(&self, params: tower_lsp::lsp_types::DidCloseTextDocumentParams) {
        handlers::document_sync::handle_did_close(self, params).await;
    }

    // -------------------------------------------------------------------------
    // Workspace
    // -------------------------------------------------------------------------

    async fn did_change_configuration(
        &self,
        params: tower_lsp::lsp_types::DidChangeConfigurationParams,
    ) {
        handlers::workspace::handle_did_change_configuration(self, params).await;
    }

    async fn did_change_watched_files(
        &self,
        params: tower_lsp::lsp_types::DidChangeWatchedFilesParams,
    ) {
        handlers::workspace::handle_did_change_watched_files(self, params).await;
    }

    // -------------------------------------------------------------------------
    // Features
    // -------------------------------------------------------------------------

    async fn completion(
        &self,
        params: tower_lsp::lsp_types::CompletionParams,
    ) -> Result<Option<tower_lsp::lsp_types::CompletionResponse>> {
        handlers::features::handle_completion(self, params).await
    }

    async fn hover(
        &self,
        params: tower_lsp::lsp_types::HoverParams,
    ) -> Result<Option<tower_lsp::lsp_types::Hover>> {
        handlers::features::handle_hover(self, params).await
    }

    async fn goto_definition(
        &self,
        params: tower_lsp::lsp_types::GotoDefinitionParams,
    ) -> Result<Option<tower_lsp::lsp_types::GotoDefinitionResponse>> {
        handlers::features::handle_goto_definition(self, params).await
    }

    async fn references(
        &self,
        params: tower_lsp::lsp_types::ReferenceParams,
    ) -> Result<Option<Vec<tower_lsp::lsp_types::Location>>> {
        handlers::features::handle_references(self, params).await
    }

    async fn prepare_rename(
        &self,
        params: tower_lsp::lsp_types::TextDocumentPositionParams,
    ) -> Result<Option<tower_lsp::lsp_types::PrepareRenameResponse>> {
        handlers::features::handle_prepare_rename(self, params).await
    }

    async fn rename(
        &self,
        params: tower_lsp::lsp_types::RenameParams,
    ) -> Result<Option<tower_lsp::lsp_types::WorkspaceEdit>> {
        handlers::features::handle_rename(self, params).await
    }

    async fn code_action(
        &self,
        params: tower_lsp::lsp_types::CodeActionParams,
    ) -> Result<Option<tower_lsp::lsp_types::CodeActionResponse>> {
        handlers::code_action::handle_code_action(self, params).await
    }

    async fn execute_command(
        &self,
        params: tower_lsp::lsp_types::ExecuteCommandParams,
    ) -> Result<Option<serde_json::Value>> {
        handlers::execute_command::handle_execute_command(self, params).await
    }
}

/// Collects and sorts languages from translations.
///
/// Sort order:
/// 1. `current_language`
/// 2. `primary_languages` (config order)
/// 3. Others (alphabetical)
#[must_use]
pub fn collect_sorted_languages(
    db: &dyn crate::db::I18nDatabase,
    translations: &[crate::input::translation::Translation],
    current_language: Option<&str>,
    primary_languages: Option<&[String]>,
) -> Vec<String> {
    let languages: std::collections::HashSet<String> =
        translations.iter().map(|t| t.language(db)).collect();
    sort_languages(languages, current_language, primary_languages)
}

fn sort_languages(
    languages: std::collections::HashSet<String>,
    current_language: Option<&str>,
    primary_languages: Option<&[String]>,
) -> Vec<String> {
    let mut result = Vec::new();
    let mut remaining: std::collections::HashSet<_> = languages;

    if let Some(current) = current_language
        && remaining.remove(current)
    {
        result.push(current.to_string());
    }

    if let Some(primaries) = primary_languages {
        for primary in primaries {
            if remaining.remove(primary) {
                result.push(primary.clone());
            }
        }
    }

    let mut others: Vec<_> = remaining.into_iter().collect();
    others.sort();
    result.extend(others);

    result
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use rstest::rstest;

    use super::sort_languages;

    fn langs(list: &[&str]) -> Vec<String> {
        list.iter().copied().map(String::from).collect()
    }

    fn lang_set(list: &[&str]) -> HashSet<String> {
        list.iter().copied().map(String::from).collect()
    }

    #[rstest]
    fn sort_languages_no_priority() {
        let languages = lang_set(&["en", "ja", "zh"]);
        let result = sort_languages(languages, None, None);
        assert_eq!(result, langs(&["en", "ja", "zh"]));
    }

    #[rstest]
    fn sort_languages_with_current_language() {
        let languages = lang_set(&["en", "ja", "zh"]);
        let result = sort_languages(languages, Some("ja"), None);
        assert_eq!(result, langs(&["ja", "en", "zh"]));
    }

    #[rstest]
    fn sort_languages_with_primary_languages() {
        let languages = lang_set(&["en", "ja", "zh"]);
        let primaries = langs(&["zh", "ja"]);
        let result = sort_languages(languages, None, Some(&primaries));
        assert_eq!(result, langs(&["zh", "ja", "en"]));
    }

    #[rstest]
    fn sort_languages_current_overrides_primary() {
        let languages = lang_set(&["en", "ja", "zh"]);
        let primaries = langs(&["zh", "ja"]);
        let result = sort_languages(languages, Some("en"), Some(&primaries));
        assert_eq!(result, langs(&["en", "zh", "ja"]));
    }

    #[rstest]
    fn sort_languages_current_in_primary_no_duplicate() {
        let languages = lang_set(&["en", "ja", "zh"]);
        let primaries = langs(&["ja", "zh"]);
        let result = sort_languages(languages, Some("ja"), Some(&primaries));
        assert_eq!(result, langs(&["ja", "zh", "en"]));
    }

    #[rstest]
    fn sort_languages_nonexistent_current_ignored() {
        let languages = lang_set(&["en", "ja", "zh"]);
        let result = sort_languages(languages, Some("fr"), None);
        assert_eq!(result, langs(&["en", "ja", "zh"]));
    }

    #[rstest]
    fn sort_languages_nonexistent_primary_ignored() {
        let languages = lang_set(&["en", "ja", "zh"]);
        let primaries = langs(&["fr", "de"]);
        let result = sort_languages(languages, None, Some(&primaries));
        assert_eq!(result, langs(&["en", "ja", "zh"]));
    }
}
