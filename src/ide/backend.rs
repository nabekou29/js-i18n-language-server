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
}

impl Backend {
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

        // source_files をクリア
        self.source_files.lock().await.clear();

        // ワークスペースを再インデックス
        if let Ok(workspace_folders) = self.get_workspace_folders().await {
            for folder in workspace_folders {
                if let Ok(workspace_path) = folder.uri.to_file_path() {
                    let config_manager = self.config_manager.lock().await;
                    let db = self.db.lock().await.clone();
                    let source_files = self.source_files.clone();

                    if let Err(error) = self
                        .workspace_indexer
                        .index_workspace(db, &workspace_path, &config_manager, source_files)
                        .await
                    {
                        self.client
                            .log_message(MessageType::ERROR, format!("Reindexing failed: {error}"))
                            .await;
                    } else {
                        self.client.log_message(MessageType::INFO, "Reindexing complete").await;
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
                    TextDocumentSyncKind::INCREMENTAL,
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

                    if let Err(error) = self
                        .workspace_indexer
                        .index_workspace(db, &workspace_path, &config_manager, source_files)
                        .await
                    {
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

    async fn did_open(&self, _params: DidOpenTextDocumentParams) {
        self.client.log_message(MessageType::INFO, "file opened!").await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        use salsa::Setter;

        let uri = params.text_document.uri;

        // ファイルパスを取得
        let Ok(file_path) = uri.to_file_path() else {
            tracing::warn!("Failed to convert URI to file path: {}", uri);
            return;
        };

        // 変更内容を取得（最後の変更を使用）
        let Some(change) = params.content_changes.into_iter().last() else {
            return;
        };
        let new_content = change.text;

        // ファイル内容を更新して解析
        let key_usages_count = {
            // データベースのロックを取得（可変参照が必要）
            let mut db = self.db.lock().await;
            let mut source_files = self.source_files.lock().await;

            // 既存の SourceFile を探すか新規作成
            let source_file = if let Some(existing) = source_files.get(&file_path) {
                // 既存の SourceFile がある場合、内容を更新（Salsa が自動的に依存クエリを無効化）
                existing.set_text(&mut *db).to(new_content);
                *existing
            } else {
                // 新しい SourceFile を作成
                use crate::input::source::{
                    ProgrammingLanguage,
                    SourceFile,
                };
                let language = ProgrammingLanguage::from_uri(uri.as_str());
                let source_file = SourceFile::new(&*db, uri.to_string(), new_content, language);
                source_files.insert(file_path.clone(), source_file);
                source_file
            };

            // analyze_source を実行（Salsa が自動的に差分計算）
            let key_usages = crate::syntax::analyze_source(&*db, source_file);
            key_usages.len()
            // ここでロックが自動的に解放される
        };

        self.client
            .log_message(
                MessageType::LOG,
                format!(
                    "File changed: {}, found {} key usages",
                    file_path.display(),
                    key_usages_count
                ),
            )
            .await;

        // TODO: Phase 4 で診断メッセージを送信
    }

    async fn did_save(&self, _: DidSaveTextDocumentParams) {
        self.client.log_message(MessageType::INFO, "file saved!").await;
    }

    async fn did_close(&self, _: DidCloseTextDocumentParams) {
        self.client.log_message(MessageType::INFO, "file closed!").await;
    }

    async fn hover(&self, _params: HoverParams) -> Result<Option<Hover>> {
        tracing::debug!("Hover params: {:?}", _params);

        Ok(Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: "**Hello from LSP!**\n\nThis is a hover message.".to_string(),
            }),
            range: None,
        }))
    }
}
