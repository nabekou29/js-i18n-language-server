//! LSP Backend 実装

use std::path::{
    Path,
    PathBuf,
};
use std::sync::Arc;
use std::time::Duration;

/// 翻訳インデックス完了を待機する際のタイムアウト
pub(crate) const TRANSLATIONS_INDEX_TIMEOUT: Duration = Duration::from_millis(500);

use tokio::sync::Mutex;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::{
    DidChangeWatchedFilesRegistrationOptions,
    FileChangeType,
    FileSystemWatcher,
    GlobPattern,
    MessageType,
    Registration,
    WatchKind,
    WorkspaceFolder,
};
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
    /// LSP クライアント
    pub client: Client,
    /// 設定管理
    pub config_manager: Arc<Mutex<ConfigManager>>,
    /// ワークスペースインデクサー
    pub workspace_indexer: Arc<WorkspaceIndexer>,
    /// 共有状態（`db`, `source_files`, `translations`, `opened_files`）
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
    /// URI をファイルパスに変換
    ///
    /// 変換に失敗した場合はログを出力して `None` を返します。
    pub(crate) fn uri_to_path(uri: &tower_lsp::lsp_types::Url) -> Option<PathBuf> {
        uri.to_file_path().ok().or_else(|| {
            tracing::warn!("Failed to convert URI to file path: {}", uri);
            None
        })
    }

    /// 翻訳インデックスの完了を待機
    ///
    /// タイムアウト付きで翻訳データのインデックスが完了するまで待機します。
    /// 完了した場合は `true`、タイムアウトした場合は `false` を返します。
    pub(crate) async fn wait_for_translations(&self) -> bool {
        self.workspace_indexer.wait_for_translations_indexed(TRANSLATIONS_INDEX_TIMEOUT).await
    }

    /// ファイルパスとカーソル位置から翻訳キーのテキストを取得
    ///
    /// `SourceFile` または `Translation` のどちらからも取得を試みます。
    /// キーが見つかった場合は `Some(key_text)` を、見つからない場合は `None` を返します。
    pub(crate) async fn get_key_at_position(
        &self,
        file_path: &Path,
        position: crate::types::SourcePosition,
    ) -> Option<String> {
        // まず SourceFile から試す
        let source_file = {
            let source_files = self.state.source_files.lock().await;
            source_files.get(file_path).copied()
        };

        let db = self.state.db.lock().await;

        if let Some(source_file) = source_file {
            // SourceFile からカーソル位置の翻訳キーを取得
            crate::syntax::key_at_position(&*db, source_file, position)
                .map(|key| key.text(&*db).clone())
        } else {
            // SourceFile が見つからない場合、Translation から試す
            tracing::debug!("Source file not found, trying Translation: {}", file_path.display());

            let translations = self.state.translations.lock().await;
            let file_path_str = file_path.to_string_lossy();

            // ファイルパスが一致する Translation を検索
            let result = translations
                .iter()
                .find(|t| t.file_path(&*db) == file_path_str.as_ref())
                .and_then(|t| t.key_at_position(&*db, position).map(|key| key.text(&*db).clone()));
            drop(translations);
            result
        }
    }

    /// 開いているすべてのファイルに diagnostics を送信
    pub(crate) async fn send_diagnostics_to_opened_files(&self) {
        use crate::input::source::ProgrammingLanguage;

        let opened_files = self.state.opened_files.lock().await;
        let file_count = opened_files.len();

        tracing::info!(file_count, "Sending diagnostics to opened files");

        for uri in opened_files.iter() {
            // ファイルパスを取得
            let Some(file_path) = Self::uri_to_path(uri) else {
                continue;
            };

            // 言語を判定（サポート対象外ならスキップ）
            if ProgrammingLanguage::from_uri(uri.as_str()).is_none() {
                tracing::debug!("Skipping diagnostics for unsupported file type: {}", uri);
                continue;
            }

            // SourceFile を取得
            let source_file = {
                let source_files = self.state.source_files.lock().await;
                source_files.get(&file_path).copied()
            };

            let Some(source_file) = source_file else {
                tracing::debug!("Source file not found: {}", file_path.display());
                continue;
            };

            // Diagnostics を生成
            let diagnostics = {
                let db = self.state.db.lock().await;
                let translations = self.state.translations.lock().await;
                crate::ide::diagnostics::generate_diagnostics(&*db, source_file, &translations)
            };

            // Diagnostics を送信
            self.client.publish_diagnostics(uri.clone(), diagnostics, None).await;
            tracing::debug!(uri = %uri, "Diagnostics sent");
        }
    }

    /// ソースファイルを更新または作成し、診断メッセージを生成・送信
    ///
    /// # Arguments
    /// * `uri` - ファイルのURI
    /// * `text` - ファイルの内容
    /// * `force_create` - 既存の `SourceFile` を無視して新規作成するかどうか
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

        // ファイルパスを取得
        let Some(file_path) = Self::uri_to_path(&uri) else {
            return;
        };

        // 言語を判定
        let Some(language) = ProgrammingLanguage::from_uri(uri.as_str()) else {
            // サポート対象外のファイル（JSON など）は SourceFile として扱わない
            tracing::debug!("Skipping SourceFile creation for unsupported file type: {}", uri);
            return;
        };

        // SourceFile を更新
        let source_file = {
            let mut db = self.state.db.lock().await;
            let mut source_files = self.state.source_files.lock().await;

            // SourceFile を取得または作成
            if force_create {
                // 強制的に新規作成
                let source_file = SourceFile::new(&*db, uri.to_string(), text, language);
                source_files.insert(file_path.clone(), source_file);
                drop(db);
                drop(source_files);
                source_file
            } else if let Some(&existing) = source_files.get(&file_path) {
                // 既存の SourceFile を更新
                existing.set_text(&mut *db).to(text);
                drop(db);
                drop(source_files);
                existing
            } else {
                // SourceFile が存在しない場合は新規作成
                let source_file = SourceFile::new(&*db, uri.to_string(), text, language);
                source_files.insert(file_path.clone(), source_file);
                drop(db);
                drop(source_files);
                source_file
            }
        };

        tracing::info!(uri = %uri, "Source file updated");

        // 翻訳データが必要なため、インデックス完了を待つ
        // タイムアウトした場合は diagnostics をスキップ
        if !self.wait_for_translations().await {
            tracing::debug!(uri = %uri, "Skipping diagnostics - translations not indexed yet");
            return;
        }

        tracing::debug!(uri = %uri, "Generating diagnostics");

        // 翻訳インデックス完了後に診断メッセージを生成して送信
        let diagnostics = {
            let db = self.state.db.lock().await;
            let translations = self.state.translations.lock().await;
            crate::ide::diagnostics::generate_diagnostics(&*db, source_file, &translations)
        };

        self.client.publish_diagnostics(uri.clone(), diagnostics, None).await;
        tracing::debug!(uri = %uri, "Diagnostics generated and sent");
    }

    /// ワークスペースフォルダを取得
    ///
    /// クライアントからワークスペースフォルダのリストを取得します。
    /// フォルダが設定されていない場合は空のVecを返します。
    ///
    /// # Errors
    /// クライアントとの通信に失敗した場合
    pub(crate) async fn get_workspace_folders(&self) -> Result<Vec<WorkspaceFolder>> {
        self.client.workspace_folders().await.map(Option::unwrap_or_default)
    }

    /// ワークスペースを再インデックス
    ///
    /// 新しい Salsa データベースを作成して、全ファイルを再インデックスします。
    /// これにより、設定変更が反映され、古いキャッシュがクリアされます。
    pub(crate) async fn reindex_workspace(&self) {
        self.client.log_message(MessageType::INFO, "Reindexing workspace...").await;

        // 新しい Salsa データベースを作成（古いキャッシュをクリア）
        let new_db = I18nDatabaseImpl::default();
        *self.state.db.lock().await = new_db;

        // source_files と translations をクリア
        self.state.source_files.lock().await.clear();
        self.state.translations.lock().await.clear();

        // インデックス状態をリセット
        self.workspace_indexer.reset_indexing_state();

        // ワークスペースを再インデックス
        if let Ok(workspace_folders) = self.get_workspace_folders().await {
            for folder in workspace_folders {
                if let Ok(workspace_path) = folder.uri.to_file_path() {
                    let config_manager = self.config_manager.lock().await;
                    let db = self.state.db.lock().await.clone();
                    let source_files = self.state.source_files.clone();

                    match self
                        .workspace_indexer
                        .index_workspace(
                            db,
                            &workspace_path,
                            &config_manager,
                            source_files,
                            self.state.translations.clone(),
                            None::<fn(u32, u32)>,
                        )
                        .await
                    {
                        Ok(()) => {
                            self.client.log_message(MessageType::INFO, "Reindexing complete").await;
                        }
                        Err(error) => {
                            self.client
                                .log_message(
                                    MessageType::ERROR,
                                    format!("Reindexing failed: {error}"),
                                )
                                .await;
                        }
                    }
                }
            }
        }
    }

    /// 翻訳ファイルを再読み込み
    ///
    /// 指定されたJSONファイルを再読み込みし、translations を更新します。
    pub(crate) async fn reload_translation_file(&self, file_path: &Path) {
        let config_manager = self.config_manager.lock().await;
        let key_separator = config_manager.get_settings().key_separator.clone();
        drop(config_manager);

        let db = self.state.db.lock().await;

        match crate::input::translation::load_translation_file(&*db, file_path, &key_separator) {
            Ok(new_translation) => {
                let mut translations = self.state.translations.lock().await;

                // 既存のエントリを削除
                let file_path_str = file_path.to_string_lossy().to_string();
                translations.retain(|t| t.file_path(&*db) != &file_path_str);

                // 新しいエントリを追加
                translations.push(new_translation);
                drop(translations);

                tracing::info!("Reloaded translation file: {:?}", file_path);
            }
            Err(e) => {
                tracing::warn!("Failed to reload translation file {:?}: {}", file_path, e);
            }
        }
    }

    /// 翻訳ファイルを削除
    ///
    /// 指定されたファイルに対応する翻訳エントリを translations から削除します。
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

    /// ファイルウォッチを登録
    ///
    /// 設定ファイルと翻訳ファイルの変更を監視するためのファイルウォッチを登録します。
    pub(crate) async fn register_file_watchers(&self) {
        // 設定から翻訳ファイルのパターンを取得
        let translation_pattern = {
            let config_manager = self.config_manager.lock().await;
            config_manager.get_settings().translation_files.file_pattern.clone()
        };

        let Ok(register_options) = serde_json::to_value(DidChangeWatchedFilesRegistrationOptions {
            watchers: vec![
                // 設定ファイル (.js-i18n.json)
                FileSystemWatcher {
                    glob_pattern: GlobPattern::String("**/.js-i18n.json".to_string()),
                    kind: Some(WatchKind::all()),
                },
                // 翻訳ファイル
                FileSystemWatcher {
                    glob_pattern: GlobPattern::String(translation_pattern.clone()),
                    kind: Some(WatchKind::all()),
                },
            ],
        }) else {
            tracing::warn!("Failed to serialize file watcher options");
            return;
        };

        let registration = Registration {
            id: "watch-files".to_string(),
            method: "workspace/didChangeWatchedFiles".to_string(),
            register_options: Some(register_options),
        };

        tracing::info!(
            "Registering file watchers for config and translation files (pattern: {})",
            translation_pattern
        );
        if let Err(e) = self.client.register_capability(vec![registration]).await {
            tracing::warn!("Failed to register file watcher: {}", e);
        }
    }

    /// 設定ファイルかどうかを判定
    pub(crate) fn is_config_file(file_path: &Path) -> bool {
        file_path.file_name().is_some_and(|name| name == ".js-i18n.json")
    }

    /// 翻訳ファイルかどうかを判定
    pub(crate) async fn is_translation_file(&self, file_path: &Path) -> bool {
        let file_pattern = {
            let config_manager = self.config_manager.lock().await;
            config_manager.get_settings().translation_files.file_pattern.clone()
        };

        globset::Glob::new(&file_pattern)
            .is_ok_and(|glob| glob.compile_matcher().is_match(file_path))
    }

    /// 設定ファイルの変更を処理
    ///
    /// TODO: 設定ファイルが変更された場合の処理を実装
    /// - 設定の再読み込み
    /// - ファイルウォッチャーの再登録（パターンが変わった場合）
    /// - ワークスペースの再インデックス
    #[allow(clippy::unused_async)] // TODO: 実装時に async 処理が必要になる予定
    pub(crate) async fn handle_config_file_change(
        &self,
        file_path: &Path,
        change_type: FileChangeType,
    ) {
        tracing::info!(
            "Config file changed: {:?}, type: {:?} (handling not yet implemented)",
            file_path,
            change_type
        );
        // TODO: 実装
    }
}

// =============================================================================
// LanguageServer Trait 実装
// =============================================================================
// 各メソッドは handlers モジュールの対応する関数に委譲します。

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

    async fn did_change_workspace_folders(
        &self,
        params: tower_lsp::lsp_types::DidChangeWorkspaceFoldersParams,
    ) {
        handlers::workspace::handle_did_change_workspace_folders(self, params).await;
    }

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
}
