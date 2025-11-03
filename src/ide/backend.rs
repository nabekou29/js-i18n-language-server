//! LSP Backend 実装

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::Mutex;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::{
    CompletionOptions,
    DidChangeConfigurationParams,
    DidChangeTextDocumentParams,
    DidChangeWatchedFilesParams,
    DidChangeWorkspaceFoldersParams,
    DidCloseTextDocumentParams,
    DidOpenTextDocumentParams,
    DidSaveTextDocumentParams,
    ExecuteCommandOptions,
    Hover,
    HoverContents,
    HoverParams,
    HoverProviderCapability,
    InitializeParams,
    InitializeResult,
    InitializedParams,
    MarkupContent,
    MarkupKind,
    MessageType,
    OneOf,
    ServerCapabilities,
    TextDocumentSyncCapability,
    TextDocumentSyncKind,
    WorkDoneProgressOptions,
    WorkspaceFolder,
    WorkspaceFoldersServerCapabilities,
    WorkspaceServerCapabilities,
};
use tower_lsp::{
    Client,
    LanguageServer,
};

use crate::config::ConfigManager;
use crate::db::I18nDatabaseImpl;
use crate::indexer::workspace::WorkspaceIndexer;
use crate::input::source::SourceFile;
use crate::input::translation::Translation;

/// LSP Backend
#[derive(Clone)]
pub struct Backend {
    /// LSP クライアント
    pub client: Client,
    /// 設定管理
    pub config_manager: Arc<Mutex<ConfigManager>>,
    /// ワークスペースインデクサー
    pub workspace_indexer: Arc<WorkspaceIndexer>,
    /// Salsa データベース
    pub db: Arc<Mutex<I18nDatabaseImpl>>,
    /// `SourceFile` 管理（ファイルパス → `SourceFile`）
    pub source_files: Arc<Mutex<HashMap<PathBuf, SourceFile>>>,
    /// 翻訳データ
    pub translations: Arc<Mutex<Vec<Translation>>>,
}

impl std::fmt::Debug for Backend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Backend")
            .field("config_manager", &"<ConfigManager>")
            .field("workspace_indexer", &"<WorkspaceIndexer>")
            .field("db", &"<I18nDatabaseImpl>")
            .field("source_files", &"<HashMap<PathBuf, SourceFile>>")
            .field("translations", &"<Vec<Translation>>")
            .finish_non_exhaustive()
    }
}

impl Backend {
    /// ソースファイルを更新または作成し、診断メッセージを生成・送信
    ///
    /// # Arguments
    /// * `uri` - ファイルのURI
    /// * `text` - ファイルの内容
    /// * `force_create` - 既存の `SourceFile` を無視して新規作成するかどうか
    async fn update_and_diagnose(
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

        // ファイルパスを取得
        let Ok(file_path) = uri.to_file_path() else {
            tracing::warn!("Failed to convert URI to file path: {}", uri);
            return;
        };

        // ファイル内容を更新して解析
        let diagnostics = {
            let mut db = self.db.lock().await;
            let mut source_files = self.source_files.lock().await;

            // SourceFile を取得または作成
            let source_file = if force_create {
                // 強制的に新規作成
                let language = ProgrammingLanguage::from_uri(uri.as_str());
                let source_file = SourceFile::new(&*db, uri.to_string(), text, language);
                source_files.insert(file_path.clone(), source_file);
                source_file
            } else if let Some(&existing) = source_files.get(&file_path) {
                // 既存の SourceFile を更新
                existing.set_text(&mut *db).to(text);
                existing
            } else {
                // SourceFile が存在しない場合は新規作成
                let language = ProgrammingLanguage::from_uri(uri.as_str());
                let source_file = SourceFile::new(&*db, uri.to_string(), text, language);
                source_files.insert(file_path.clone(), source_file);
                source_file
            };
            drop(source_files);

            // 診断メッセージを生成
            let translations = self.translations.lock().await;
            let diagnostics =
                crate::ide::diagnostics::generate_diagnostics(&*db, source_file, &translations);
            drop(db);
            drop(translations);

            diagnostics
        };

        // 診断メッセージを送信
        self.client.publish_diagnostics(uri.clone(), diagnostics, None).await;

        tracing::debug!(
            uri = %uri,
            "Diagnostics generated and sent"
        );
    }

    /// ワークスペースフォルダを取得
    ///
    /// クライアントからワークスペースフォルダのリストを取得します。
    /// フォルダが設定されていない場合は空のVecを返します。
    ///
    /// # Errors
    /// クライアントとの通信に失敗した場合
    async fn get_workspace_folders(&self) -> Result<Vec<WorkspaceFolder>> {
        self.client.workspace_folders().await.map(Option::unwrap_or_default)
    }

    /// ワークスペースを再インデックス
    ///
    /// 新しい Salsa データベースを作成して、全ファイルを再インデックスします。
    /// これにより、設定変更が反映され、古いキャッシュがクリアされます。
    async fn reindex_workspace(&self) {
        self.client.log_message(MessageType::INFO, "Reindexing workspace...").await;

        // 新しい Salsa データベースを作成（古いキャッシュをクリア）
        let new_db = I18nDatabaseImpl::default();
        *self.db.lock().await = new_db;

        // source_files と translations をクリア
        self.source_files.lock().await.clear();
        self.translations.lock().await.clear();

        // ワークスペースを再インデックス
        if let Ok(workspace_folders) = self.get_workspace_folders().await {
            for folder in workspace_folders {
                if let Ok(workspace_path) = folder.uri.to_file_path() {
                    let config_manager = self.config_manager.lock().await;
                    let db = self.db.lock().await.clone();
                    let source_files = self.source_files.clone();

                    match self
                        .workspace_indexer
                        .index_workspace(db, &workspace_path, &config_manager, source_files)
                        .await
                    {
                        Ok(translations) => {
                            // 翻訳データを保存
                            *self.translations.lock().await = translations;
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
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        // ワークスペースルートを取得
        let workspace_root = params
            .workspace_folders
            .as_ref()
            .and_then(|folders| folders.first())
            .and_then(|folder| folder.uri.to_file_path().ok());

        // ConfigManager に設定を読み込ませる
        let mut config_manager = self.config_manager.lock().await;
        if let Err(error) = config_manager.load_settings(workspace_root) {
            self.client
                .log_message(MessageType::ERROR, format!("Configuration error: {error}"))
                .await;
            tracing::error!("Configuration error during initialize: {}", error);
        }
        drop(config_manager); // ロックを早めに解放

        Ok(InitializeResult {
            server_info: None,
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec![".".to_string()]),
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                    all_commit_characters: None,
                    completion_item: None,
                }),
                execute_command_provider: Some(ExecuteCommandOptions {
                    commands: vec!["dummy.do_something".to_string()],
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                }),
                workspace: Some(WorkspaceServerCapabilities {
                    workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                        supported: Some(true),
                        change_notifications: Some(OneOf::Left(true)),
                    }),
                    file_operations: None,
                }),
                ..ServerCapabilities::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client.log_message(MessageType::INFO, "initialized!").await;

        if let Ok(workspace_folders) = self.get_workspace_folders().await {
            self.client
                .log_message(MessageType::INFO, format!("Workspace folders: {workspace_folders:?}"))
                .await;

            for folder in workspace_folders {
                if let Ok(workspace_path) = folder.uri.to_file_path() {
                    // ConfigManager をロックして参照を取得
                    let config_manager = self.config_manager.lock().await;

                    // Database をクローン（Salsa のクローンは安価）
                    let db = self.db.lock().await.clone();

                    // source_files をクローン（Arc のクローンは安価）
                    let source_files = self.source_files.clone();

                    match self
                        .workspace_indexer
                        .index_workspace(db, &workspace_path, &config_manager, source_files)
                        .await
                    {
                        Ok(translations) => {
                            // 翻訳データを保存
                            *self.translations.lock().await = translations;
                            self.client
                                .log_message(MessageType::INFO, "Workspace indexing complete")
                                .await;
                        }
                        Err(error) => {
                            self.client
                                .log_message(
                                    MessageType::ERROR,
                                    format!("error indexing workspace: {error}"),
                                )
                                .await;
                        }
                    }
                }
            }
        }
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_change_workspace_folders(&self, _: DidChangeWorkspaceFoldersParams) {
        self.client.log_message(MessageType::INFO, "workspace folders changed!").await;
    }

    async fn did_change_configuration(&self, params: DidChangeConfigurationParams) {
        self.client.log_message(MessageType::INFO, "configuration changed!").await;

        // 設定を更新
        if let Ok(new_settings) =
            serde_json::from_value::<crate::config::I18nSettings>(params.settings)
        {
            let mut config_manager = self.config_manager.lock().await;
            match config_manager.update_settings(new_settings) {
                Ok(()) => {
                    drop(config_manager); // ロックを解放
                    self.client
                        .log_message(MessageType::INFO, "Configuration updated successfully")
                        .await;

                    // 設定変更後、ワークスペースを再インデックス
                    self.reindex_workspace().await;
                }
                Err(error) => {
                    self.client
                        .log_message(
                            MessageType::ERROR,
                            format!("Configuration validation error: {error}"),
                        )
                        .await;
                }
            }
        }
    }

    async fn did_change_watched_files(&self, _: DidChangeWatchedFilesParams) {
        self.client.log_message(MessageType::INFO, "watched files have changed!").await;
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.client.log_message(MessageType::INFO, "file opened!").await;

        let uri = params.text_document.uri;
        let text = params.text_document.text;

        self.update_and_diagnose(uri, text, true).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;

        // 変更内容を取得（FULL sync なので全体のテキストが送られてくる）
        let Some(change) = params.content_changes.into_iter().last() else {
            return;
        };
        let new_content = change.text;

        self.update_and_diagnose(uri, new_content, false).await;
    }

    async fn did_save(&self, _: DidSaveTextDocumentParams) {
        self.client.log_message(MessageType::INFO, "file saved!").await;
    }

    async fn did_close(&self, _: DidCloseTextDocumentParams) {
        self.client.log_message(MessageType::INFO, "file closed!").await;
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        tracing::debug!(uri = %uri, line = position.line, character = position.character, "Hover request");

        // ファイルパスを取得
        let Ok(file_path) = uri.to_file_path() else {
            tracing::warn!("Failed to convert URI to file path: {}", uri);
            return Ok(None);
        };

        // SourceFile を取得
        let source_file = {
            let source_files = self.source_files.lock().await;
            source_files.get(&file_path).copied()
        };

        let Some(source_file) = source_file else {
            tracing::debug!("Source file not found in cache: {}", file_path.display());
            return Ok(None);
        };

        // カーソル位置の翻訳キーを取得
        let db = self.db.lock().await;
        let source_position = crate::types::SourcePosition::from(position);
        let Some(key) = crate::syntax::key_at_position(&*db, source_file, source_position) else {
            tracing::debug!("No translation key found at position");
            return Ok(None);
        };

        // 翻訳内容を取得
        let translations = self.translations.lock().await;
        let Some(hover_text) = crate::ide::hover::generate_hover_content(&*db, key, &translations)
        else {
            tracing::debug!("No translations found for key: {}", key.text(&*db));
            return Ok(None);
        };

        tracing::debug!("Generated hover content for key: {}", key.text(&*db));
        drop(db);

        Ok(Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: hover_text,
            }),
            range: None,
        }))
    }
}
