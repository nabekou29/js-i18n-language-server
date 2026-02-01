//! Document synchronization handlers.

use tower_lsp::lsp_types::{
    DidChangeTextDocumentParams,
    DidCloseTextDocumentParams,
    DidOpenTextDocumentParams,
    DidSaveTextDocumentParams,
    MessageType,
};

use super::super::backend::Backend;

pub async fn handle_did_open(backend: &Backend, params: DidOpenTextDocumentParams) {
    backend.client.log_message(MessageType::INFO, "file opened!").await;

    let uri = params.text_document.uri.clone();
    let text = params.text_document.text;

    {
        let mut opened_files = backend.state.opened_files.lock().await;
        opened_files.insert(uri.clone());
    }

    backend.update_and_diagnose(uri, text, true).await;
    backend.send_decorations_changed().await;
}

pub async fn handle_did_change(backend: &Backend, params: DidChangeTextDocumentParams) {
    let uri = params.text_document.uri;

    let Some(change) = params.content_changes.into_iter().next_back() else {
        return;
    };
    let new_content = change.text;

    // Check if this is a translation file
    if let Some(file_path) = Backend::uri_to_path(&uri)
        && backend.is_translation_file(&file_path).await
    {
        backend.update_translation_from_content(&file_path, &new_content).await;
        backend.send_diagnostics_to_opened_files().await;
        backend.send_decorations_changed().await;
        return;
    }

    backend.update_and_diagnose(uri, new_content, false).await;
    backend.send_decorations_changed().await;
}

pub async fn handle_did_save(backend: &Backend, _: DidSaveTextDocumentParams) {
    backend.client.log_message(MessageType::INFO, "file saved!").await;
}

pub async fn handle_did_close(backend: &Backend, params: DidCloseTextDocumentParams) {
    backend.client.log_message(MessageType::INFO, "file closed!").await;

    let uri = params.text_document.uri;

    {
        let mut opened_files = backend.state.opened_files.lock().await;
        opened_files.remove(&uri);
    }
}
