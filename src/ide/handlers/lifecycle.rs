//! LSP lifecycle handlers: `initialize`, `initialized`, `shutdown`.

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::{
    CodeActionProviderCapability,
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

pub async fn handle_initialize(
    backend: &Backend,
    params: InitializeParams,
) -> Result<InitializeResult> {
    let workspace_root = params
        .workspace_folders
        .as_ref()
        .and_then(|folders| folders.first())
        .and_then(|folder| folder.uri.to_file_path().ok());

    let mut config_manager = backend.config_manager.lock().await;
    if let Err(error) = config_manager.load_settings(workspace_root) {
        backend
            .client
            .log_message(MessageType::ERROR, format!("Configuration error: {error}"))
            .await;
        tracing::error!("Configuration error during initialize: {}", error);
    }
    drop(config_manager);

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
            code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
            execute_command_provider: Some(ExecuteCommandOptions {
                commands: vec![
                    "i18n.editTranslation".to_string(),
                    "i18n.deleteUnusedKeys".to_string(),
                    "i18n.getKeyAtPosition".to_string(),
                    "i18n.getTranslationValue".to_string(),
                    "i18n.getDecorations".to_string(),
                    "i18n.getCurrentLanguage".to_string(),
                    "i18n.setCurrentLanguage".to_string(),
                ],
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
                let token = NumberOrString::String("workspace-indexing".to_string());

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

                let config_manager = backend.config_manager.lock().await;
                let db = backend.state.db.lock().await.clone();
                let source_files = backend.state.source_files.clone();
                let (progress_tx, mut progress_rx) = tokio::sync::mpsc::channel::<(u32, u32)>(100);

                let progress_task = {
                    let client = backend.client.clone();
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

                let index_result = backend
                    .workspace_indexer
                    .index_workspace(
                        db,
                        &workspace_path,
                        &config_manager,
                        source_files,
                        backend.state.translations.clone(),
                        Some(progress_callback),
                    )
                    .await;

                drop(config_manager);
                let _ = progress_task.await;

                match index_result {
                    Ok(()) => {
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

        backend.process_pending_updates().await;
        backend.send_diagnostics_to_opened_files().await;
        backend.send_unused_key_diagnostics().await;
        backend.send_decorations_changed().await;
    }

    backend.register_file_watchers().await;
}

#[allow(clippy::unused_async)]
pub async fn handle_shutdown() -> Result<()> {
    Ok(())
}
