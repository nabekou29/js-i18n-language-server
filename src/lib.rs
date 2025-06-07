//! Rust LSP チュートリアルのバックエンド実装
//!
//! このクレートは、Language Server Protocol (LSP) を実装したシンプルなサーバーです。
//! TODOとFIXMEコメントの検出、ホバー機能、コード補完などの基本的な機能を提供します。

use serde_json::Value;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::{
    CompletionItem,
    CompletionOptions,
    CompletionParams,
    CompletionResponse,
    Diagnostic,
    DiagnosticSeverity,
    DidChangeConfigurationParams,
    DidChangeTextDocumentParams,
    DidChangeWatchedFilesParams,
    DidChangeWorkspaceFoldersParams,
    DidCloseTextDocumentParams,
    DidOpenTextDocumentParams,
    DidSaveTextDocumentParams,
    Documentation,
    ExecuteCommandOptions,
    ExecuteCommandParams,
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
    Position,
    Range,
    ServerCapabilities,
    TextDocumentSyncCapability,
    TextDocumentSyncKind,
    Url,
    WorkDoneProgressOptions,
    WorkspaceEdit,
    WorkspaceFoldersServerCapabilities,
    WorkspaceServerCapabilities,
};
use tower_lsp::{
    Client,
    LanguageServer,
};

/// LSPサーバーのバックエンド実装
#[derive(Debug, Clone)]
pub struct Backend {
    /// LSPクライアントとの通信を担当
    pub client: Client,
}

impl Backend {
    /// 診断情報をクライアントに送信
    ///
    /// ファイル内のTODOとFIXMEコメントを検出し、診断情報として報告します。
    pub async fn publish_diagnostics(&self, uri: Url, text: String) {
        let mut diagnostics = Vec::new();

        for (line_num, line) in text.lines().enumerate() {
            if let Some(pos) = line.find("TODO") {
                diagnostics.push(Diagnostic {
                    range: Range {
                        start: Position {
                            line: line_num.try_into().unwrap_or(u32::MAX),
                            character: pos.try_into().unwrap_or(u32::MAX),
                        },
                        end: Position {
                            line: line_num.try_into().unwrap_or(u32::MAX),
                            character: (pos + 4).try_into().unwrap_or(u32::MAX),
                        },
                    },
                    severity: Some(DiagnosticSeverity::INFORMATION),
                    code: None,
                    code_description: None,
                    source: Some("lsp-tutorial".to_string()),
                    message: "TODOコメントが見つかりました".to_string(),
                    related_information: None,
                    tags: None,
                    data: None,
                });
            }

            if let Some(pos) = line.find("FIXME") {
                diagnostics.push(Diagnostic {
                    range: Range {
                        start: Position {
                            line: line_num.try_into().unwrap_or(u32::MAX),
                            character: pos.try_into().unwrap_or(u32::MAX),
                        },
                        end: Position {
                            line: line_num.try_into().unwrap_or(u32::MAX),
                            character: (pos + 5).try_into().unwrap_or(u32::MAX),
                        },
                    },
                    severity: Some(DiagnosticSeverity::WARNING),
                    code: None,
                    code_description: None,
                    source: Some("lsp-tutorial".to_string()),
                    message: "FIXMEコメントが見つかりました".to_string(),
                    related_information: None,
                    tags: None,
                    data: None,
                });
            }
        }

        self.client.publish_diagnostics(uri, diagnostics, None).await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
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
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_change_workspace_folders(&self, _: DidChangeWorkspaceFoldersParams) {
        self.client.log_message(MessageType::INFO, "workspace folders changed!").await;
    }

    async fn did_change_configuration(&self, _: DidChangeConfigurationParams) {
        self.client.log_message(MessageType::INFO, "configuration changed!").await;
    }

    async fn did_change_watched_files(&self, _: DidChangeWatchedFilesParams) {
        self.client.log_message(MessageType::INFO, "watched files have changed!").await;
    }

    async fn execute_command(&self, _: ExecuteCommandParams) -> Result<Option<Value>> {
        self.client.log_message(MessageType::INFO, "command executed!").await;

        match self.client.apply_edit(WorkspaceEdit::default()).await {
            Ok(res) if res.applied => self.client.log_message(MessageType::INFO, "applied").await,
            Ok(_) => self.client.log_message(MessageType::INFO, "rejected").await,
            Err(err) => self.client.log_message(MessageType::ERROR, err).await,
        }

        Ok(None)
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.client.log_message(MessageType::INFO, "file opened!").await;

        self.publish_diagnostics(params.text_document.uri, params.text_document.text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        self.client.log_message(MessageType::INFO, "file changed!").await;

        if let Some(change) = params.content_changes.first() {
            self.publish_diagnostics(params.text_document.uri, change.text.clone()).await;
        }
    }

    async fn did_save(&self, _: DidSaveTextDocumentParams) {
        self.client.log_message(MessageType::INFO, "file saved!").await;
    }

    async fn did_close(&self, _: DidCloseTextDocumentParams) {
        self.client.log_message(MessageType::INFO, "file closed!").await;
    }

    async fn hover(&self, _params: HoverParams) -> Result<Option<Hover>> {
        Ok(Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: "**Hello from LSP!**\n\nThis is a hover message.".to_string(),
            }),
            range: None,
        }))
    }

    async fn completion(&self, _: CompletionParams) -> Result<Option<CompletionResponse>> {
        Ok(Some(CompletionResponse::Array(vec![
            CompletionItem::new_simple("Hello".to_string(), "Some detail".to_string()),
            CompletionItem::new_simple("Bye".to_string(), "More detail".to_string()),
            CompletionItem {
                label: "world".to_string(),
                detail: Some("Say world".to_string()),
                documentation: Some(Documentation::String("Insert a world message".to_string())),
                ..Default::default()
            },
        ])))
    }
}
