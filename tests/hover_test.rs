use pretty_assertions::assert_eq;
use rust_lsp_tutorial::Backend;
use tower_lsp::lsp_types::*;
use tower_lsp::{
    LanguageServer,
    LspService,
};

async fn create_test_backend() -> Backend {
    let (service, _socket) = LspService::new(|client| Backend { client });
    service.inner().clone()
}

#[tokio::test]
async fn test_hover_returns_markdown_content() {
    let backend = create_test_backend().await;

    let hover_params = HoverParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: Url::parse("file:///test.txt").unwrap() },
            position: Position { line: 0, character: 0 },
        },
        work_done_progress_params: WorkDoneProgressParams { work_done_token: None },
    };

    let result = backend.hover(hover_params).await;

    assert!(result.is_ok());
    let hover = result.unwrap();
    assert!(hover.is_some());

    let hover_content = hover.unwrap();
    match hover_content.contents {
        HoverContents::Markup(markup) => {
            assert_eq!(markup.kind, MarkupKind::Markdown);
            assert_eq!(markup.value, "**Hello from LSP!**\n\nThis is a hover message.");
        }
        _ => panic!("Expected Markup content"),
    }

    assert!(hover_content.range.is_none());
}

#[tokio::test]
async fn test_hover_capability_is_enabled() {
    let backend = create_test_backend().await;

    let init_params = InitializeParams {
        process_id: None,
        root_path: None,
        root_uri: None,
        initialization_options: None,
        capabilities: ClientCapabilities::default(),
        trace: None,
        workspace_folders: None,
        client_info: None,
        locale: None,
    };

    let result = backend.initialize(init_params).await;

    assert!(result.is_ok());
    let init_result = result.unwrap();

    assert!(init_result.capabilities.hover_provider.is_some());
    match init_result.capabilities.hover_provider.unwrap() {
        HoverProviderCapability::Simple(enabled) => assert!(enabled),
        _ => panic!("Expected Simple hover provider capability"),
    }
}

#[tokio::test]
async fn test_hover_with_different_positions() {
    let backend = create_test_backend().await;

    let positions = vec![
        Position { line: 0, character: 0 },
        Position { line: 10, character: 20 },
        Position { line: 100, character: 0 },
    ];

    for position in positions {
        let hover_params = HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: Url::parse("file:///test.txt").unwrap(),
                },
                position,
            },
            work_done_progress_params: WorkDoneProgressParams { work_done_token: None },
        };

        let result = backend.hover(hover_params).await;
        assert!(result.is_ok());

        let hover = result.unwrap();
        assert!(hover.is_some());

        let hover_content = hover.unwrap();
        match hover_content.contents {
            HoverContents::Markup(markup) => {
                assert_eq!(markup.kind, MarkupKind::Markdown);
                assert!(markup.value.contains("Hello from LSP!"));
            }
            _ => panic!("Expected Markup content"),
        }
    }
}
