//! LSP ライフサイクルハンドラー
//!
//! `initialize`, `initialized`, `shutdown` の処理を担当します。

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::{
    CompletionOptions,
    ExecuteCommandOptions,
    HoverProviderCapability,
    InitializeParams,
    InitializeResult,
    InitializedParams,
    MessageType,
    NumberOrString,
    OneOf,
    ProgressParams,
    ProgressParamsValue,
    ServerCapabilities,
    TextDocumentSyncCapability,
    TextDocumentSyncKind,
    WorkDoneProgress,
    WorkDoneProgressBegin,
    WorkDoneProgressEnd,
    WorkDoneProgressOptions,
    WorkDoneProgressReport,
    WorkspaceFoldersServerCapabilities,
    WorkspaceServerCapabilities,
    notification::Progress,
};

use super::super::backend::Backend;

/// `initialize` リクエストを処理
pub async fn handle_initialize(
    backend: &Backend,
    params: InitializeParams,
) -> Result<InitializeResult> {
    // ワークスペースルートを取得
    let workspace_root = params
        .workspace_folders
        .as_ref()
        .and_then(|folders| folders.first())
        .and_then(|folder| folder.uri.to_file_path().ok());

    // ConfigManager に設定を読み込ませる
    let mut config_manager = backend.config_manager.lock().await;
    if let Err(error) = config_manager.load_settings(workspace_root) {
        backend
            .client
            .log_message(MessageType::ERROR, format!("Configuration error: {error}"))
            .await;
        tracing::error!("Configuration error during initialize: {}", error);
    }
    drop(config_manager); // ロックを早めに解放

    Ok(InitializeResult {
        server_info: None,
        capabilities: ServerCapabilities {
            text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
            hover_provider: Some(HoverProviderCapability::Simple(true)),
            completion_provider: Some(CompletionOptions {
                resolve_provider: Some(false),
                trigger_characters: Some(vec![".".to_string(), "\"".to_string()]),
                work_done_progress_options: WorkDoneProgressOptions::default(),
                all_commit_characters: None,
                completion_item: None,
            }),
            definition_provider: Some(OneOf::Left(true)),
            references_provider: Some(OneOf::Left(true)),
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

/// `initialized` 通知を処理
#[allow(clippy::too_many_lines)]
pub async fn handle_initialized(backend: &Backend, _: InitializedParams) {
    backend.client.log_message(MessageType::INFO, "initialized!").await;

    if let Ok(workspace_folders) = backend.get_workspace_folders().await {
        backend
            .client
            .log_message(MessageType::INFO, format!("Workspace folders: {workspace_folders:?}"))
            .await;

        for folder in workspace_folders {
            if let Ok(workspace_path) = folder.uri.to_file_path() {
                // 進捗トークン
                let token = NumberOrString::String("workspace-indexing".to_string());

                // 進捗開始通知
                backend
                    .client
                    .send_notification::<Progress>(ProgressParams {
                        token: token.clone(),
                        value: ProgressParamsValue::WorkDone(WorkDoneProgress::Begin(
                            WorkDoneProgressBegin {
                                title: "Indexing Workspace".to_string(),
                                cancellable: Some(false),
                                message: Some("Starting...".to_string()),
                                percentage: Some(0),
                            },
                        )),
                    })
                    .await;

                // ConfigManager をロックして参照を取得
                let config_manager = backend.config_manager.lock().await;

                // Database をクローン（Salsa のクローンは安価）
                let db = backend.state.db.lock().await.clone();

                // source_files をクローン（Arc のクローンは安価）
                let source_files = backend.state.source_files.clone();

                // 進捗報告コールバック
                let client = backend.client.clone();
                let progress_callback = move |current: u32, total: u32| {
                    let token = token.clone();
                    let client = client.clone();
                    tokio::spawn(async move {
                        let percentage = if total > 0 { (current * 100) / total } else { 0 };
                        client
                            .send_notification::<Progress>(ProgressParams {
                                token,
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
                    });
                };

                // インデックス実行
                match backend
                    .workspace_indexer
                    .index_workspace(
                        db,
                        &workspace_path,
                        &config_manager,
                        source_files,
                        backend.state.translations.clone(),
                        Some(progress_callback),
                    )
                    .await
                {
                    Ok(()) => {
                        // 進捗完了通知
                        backend
                            .client
                            .send_notification::<Progress>(ProgressParams {
                                token: NumberOrString::String("workspace-indexing".to_string()),
                                value: ProgressParamsValue::WorkDone(WorkDoneProgress::End(
                                    WorkDoneProgressEnd {
                                        message: Some("Workspace indexing complete".to_string()),
                                    },
                                )),
                            })
                            .await;
                    }
                    Err(error) => {
                        // エラー時も進捗を終了
                        backend
                            .client
                            .send_notification::<Progress>(ProgressParams {
                                token: NumberOrString::String("workspace-indexing".to_string()),
                                value: ProgressParamsValue::WorkDone(WorkDoneProgress::End(
                                    WorkDoneProgressEnd {
                                        message: Some(format!("Indexing failed: {error}")),
                                    },
                                )),
                            })
                            .await;

                        backend
                            .client
                            .log_message(
                                MessageType::ERROR,
                                format!("error indexing workspace: {error}"),
                            )
                            .await;
                    }
                }
            }
        }

        // すべてのワークスペースフォルダーのインデックス完了後、診断を送信
        backend.send_diagnostics_to_opened_files().await;
    }

    // ファイルウォッチを登録
    backend.register_file_watchers().await;
}

/// `shutdown` リクエストを処理
#[allow(clippy::unused_async)] // trait requires async
pub async fn handle_shutdown() -> Result<()> {
    Ok(())
}
