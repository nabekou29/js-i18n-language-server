//! ワークスペース関連ハンドラー
//!
//! `did_change_workspace_folders`, `did_change_configuration`,
//! `did_change_watched_files` の処理を担当します。

use tower_lsp::lsp_types::{
    DidChangeConfigurationParams,
    DidChangeWatchedFilesParams,
    DidChangeWorkspaceFoldersParams,
    FileChangeType,
    MessageType,
};

use super::super::backend::Backend;

/// `workspace/didChangeWorkspaceFolders` 通知を処理
pub async fn handle_did_change_workspace_folders(
    backend: &Backend,
    _: DidChangeWorkspaceFoldersParams,
) {
    backend.client.log_message(MessageType::INFO, "workspace folders changed!").await;
}

/// `workspace/didChangeConfiguration` 通知を処理
pub async fn handle_did_change_configuration(
    backend: &Backend,
    params: DidChangeConfigurationParams,
) {
    backend.client.log_message(MessageType::INFO, "configuration changed!").await;

    // 設定を更新
    if let Ok(new_settings) = serde_json::from_value::<crate::config::I18nSettings>(params.settings)
    {
        let mut config_manager = backend.config_manager.lock().await;
        match config_manager.update_settings(new_settings) {
            Ok(()) => {
                drop(config_manager); // ロックを解放
                backend
                    .client
                    .log_message(MessageType::INFO, "Configuration updated successfully")
                    .await;

                // 設定変更後、ワークスペースを再インデックス
                backend.reindex_workspace().await;
            }
            Err(error) => {
                backend
                    .client
                    .log_message(
                        MessageType::ERROR,
                        format!("Configuration validation error: {error}"),
                    )
                    .await;
            }
        }
    }
}

/// `workspace/didChangeWatchedFiles` 通知を処理
pub async fn handle_did_change_watched_files(
    backend: &Backend,
    params: DidChangeWatchedFilesParams,
) {
    let mut translations_changed = false;

    for change in params.changes {
        let Some(file_path) = Backend::uri_to_path(&change.uri) else {
            continue;
        };

        // 設定ファイルの変更
        if Backend::is_config_file(&file_path) {
            backend.handle_config_file_change(&file_path, change.typ).await;
            continue;
        }

        // 翻訳ファイルの変更
        if backend.is_translation_file(&file_path).await {
            tracing::debug!("Translation file changed: {:?}, type: {:?}", file_path, change.typ);

            match change.typ {
                FileChangeType::CREATED | FileChangeType::CHANGED => {
                    backend.reload_translation_file(&file_path).await;
                    translations_changed = true;
                }
                FileChangeType::DELETED => {
                    backend.remove_translation_file(&file_path).await;
                    translations_changed = true;
                }
                _ => {}
            }
        }
    }

    // 翻訳ファイルが変更された場合、診断を更新
    if translations_changed {
        backend.send_diagnostics_to_opened_files().await;
        // 翻訳ファイルへの未使用キー診断も更新
        backend.send_unused_key_diagnostics().await;
    }
}
