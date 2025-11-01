//! LSP Backend 実装

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
use crate::indexer::workspace::WorkspaceIndexer;

/// LSP Backend
#[derive(Debug, Clone)]
pub struct Backend {
    /// LSP クライアント
    pub client: Client,
    /// 設定管理
    pub config_manager: Arc<Mutex<ConfigManager>>,
    /// ワークスペースインデクサー
    pub workspace_indexer: Arc<WorkspaceIndexer>,
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
                    if let Err(error) = self
                        .workspace_indexer
                        .index_workspace(&workspace_path, &config_manager)
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

        // 設定を更新（Phase 1: 基本的な更新のみ、Phase 3 で再インデックスを実装予定）
        if let Ok(new_settings) =
            serde_json::from_value::<crate::config::I18nSettings>(params.settings)
        {
            let mut config_manager = self.config_manager.lock().await;
            match config_manager.update_settings(new_settings) {
                Ok(()) => {
                    self.client
                        .log_message(MessageType::INFO, "Configuration updated successfully")
                        .await;
                    // TODO: Phase 3 で再インデックスをトリガー
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

    async fn did_change(&self, _params: DidChangeTextDocumentParams) {
        self.client.log_message(MessageType::INFO, "file changed!").await;
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
