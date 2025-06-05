use serde_json::Value;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

#[derive(Debug)]
struct Backend {
    client: Client,
}

impl Backend {
    async fn publish_diagnostics(&self, uri: Url, text: String) {
        let mut diagnostics = Vec::new();

        // "TODO" を検出
        for (line_num, line) in text.lines().enumerate() {
            if let Some(pos) = line.find("TODO") {
                diagnostics.push(Diagnostic {
                    range: Range {
                        start: Position { line: line_num as u32, character: pos as u32 },
                        end: Position { line: line_num as u32, character: (pos + 4) as u32 },
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

            // "FIXME" を検出
            if let Some(pos) = line.find("FIXME") {
                diagnostics.push(Diagnostic {
                    range: Range {
                        start: Position { line: line_num as u32, character: pos as u32 },
                        end: Position { line: line_num as u32, character: (pos + 5) as u32 },
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
                    work_done_progress_options: Default::default(),
                    all_commit_characters: None,
                    ..Default::default()
                }),
                execute_command_provider: Some(ExecuteCommandOptions {
                    commands: vec!["dummy.do_something".to_string()],
                    work_done_progress_options: Default::default(),
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

        // ファイルを開いた時にも診断を実行
        self.publish_diagnostics(params.text_document.uri, params.text_document.text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        self.client.log_message(MessageType::INFO, "file changed!").await;

        // ファイル変更時に診断を実行
        let text = &params.content_changes[0].text;
        self.publish_diagnostics(params.text_document.uri, text.clone()).await;
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

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();

    let (stdin, stdout) = (tokio::io::stdin(), tokio::io::stdout());
    let (service, socket) = LspService::new(|client| Backend { client });
    Server::new(stdin, stdout, socket).serve(service).await;
}
