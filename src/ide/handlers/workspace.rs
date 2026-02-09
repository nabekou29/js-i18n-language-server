//! Workspace-related handlers.

use tower_lsp::lsp_types::{
    DidChangeConfigurationParams,
    DidChangeWatchedFilesParams,
    FileChangeType,
};

use super::super::backend::Backend;

pub async fn handle_did_change_configuration(
    backend: &Backend,
    params: DidChangeConfigurationParams,
) {
    tracing::info!(settings = %params.settings, "didChangeConfiguration received");

    let new_settings = serde_json::from_value::<crate::config::I18nSettings>(
        params.settings.clone(),
    )
    .or_else(|_| {
        serde_json::from_value::<crate::config::ServerSettings>(params.settings)
            .map(|wrapped| wrapped.js_i18n)
    });

    if let Ok(new_settings) = new_settings {
        let mut config_manager = backend.config_manager.lock().await;
        match config_manager.update_settings(new_settings) {
            Ok(()) => {
                drop(config_manager);
                tracing::info!("configuration updated successfully");

                backend.reindex_workspace().await;
            }
            Err(error) => {
                tracing::error!(%error, "configuration validation error");
            }
        }
    }
}

pub async fn handle_did_change_watched_files(
    backend: &Backend,
    params: DidChangeWatchedFilesParams,
) {
    let mut translations_changed = false;

    for change in params.changes {
        let Some(file_path) = Backend::uri_to_path(&change.uri) else {
            continue;
        };

        if Backend::is_config_file(&file_path) {
            backend.handle_config_file_change(&file_path, change.typ).await;
            continue;
        }

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

    if translations_changed {
        backend.send_diagnostics_to_opened_files().await;
        backend.send_unused_key_diagnostics().await;
        backend.send_decorations_changed().await;
    }
}
