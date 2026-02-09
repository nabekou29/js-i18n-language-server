//! Document synchronization handlers.

use tower_lsp::lsp_types::{
    DidChangeTextDocumentParams,
    DidCloseTextDocumentParams,
    DidOpenTextDocumentParams,
    DidSaveTextDocumentParams,
};

use super::super::backend::Backend;

pub async fn handle_did_open(backend: &Backend, params: DidOpenTextDocumentParams) {
    let uri = params.text_document.uri.clone();
    if uri.scheme() != "file" {
        return;
    }
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
    if uri.scheme() != "file" {
        return;
    }

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

#[allow(clippy::unused_async)]
pub async fn handle_did_save(_: &Backend, _: DidSaveTextDocumentParams) {}

pub async fn handle_did_close(backend: &Backend, params: DidCloseTextDocumentParams) {
    let uri = params.text_document.uri;
    if uri.scheme() != "file" {
        return;
    }

    {
        let mut opened_files = backend.state.opened_files.lock().await;
        opened_files.remove(&uri);
    }
}
