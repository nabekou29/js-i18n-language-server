//! ドキュメント同期ハンドラー
//!
//! `did_open`, `did_change`, `did_save`, `did_close` の処理を担当します。

use tower_lsp::lsp_types::{
    DidChangeTextDocumentParams,
    DidCloseTextDocumentParams,
    DidOpenTextDocumentParams,
    DidSaveTextDocumentParams,
    MessageType,
};

use super::super::backend::Backend;

/// `textDocument/didOpen` 通知を処理
pub async fn handle_did_open(backend: &Backend, params: DidOpenTextDocumentParams) {
    backend.client.log_message(MessageType::INFO, "file opened!").await;

    let uri = params.text_document.uri.clone();
    let text = params.text_document.text;

    // 開いているファイルリストに追加
    {
        let mut opened_files = backend.state.opened_files.lock().await;
        opened_files.insert(uri.clone());
    }

    backend.update_and_diagnose(uri, text, true).await;
}

/// `textDocument/didChange` 通知を処理
pub async fn handle_did_change(backend: &Backend, params: DidChangeTextDocumentParams) {
    let uri = params.text_document.uri;

    // 変更内容を取得（FULL sync なので全体のテキストが送られてくる）
    let Some(change) = params.content_changes.into_iter().next_back() else {
        return;
    };
    let new_content = change.text;

    backend.update_and_diagnose(uri, new_content, false).await;
}

/// `textDocument/didSave` 通知を処理
pub async fn handle_did_save(backend: &Backend, _: DidSaveTextDocumentParams) {
    backend.client.log_message(MessageType::INFO, "file saved!").await;
}

/// `textDocument/didClose` 通知を処理
pub async fn handle_did_close(backend: &Backend, params: DidCloseTextDocumentParams) {
    backend.client.log_message(MessageType::INFO, "file closed!").await;

    let uri = params.text_document.uri;

    // 開いているファイルリストから削除
    {
        let mut opened_files = backend.state.opened_files.lock().await;
        opened_files.remove(&uri);
    }
}
