//! Code Action handler for `textDocument/codeAction` requests.

use std::path::Path;

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::{
    CodeAction,
    CodeActionKind,
    CodeActionOrCommand,
    CodeActionParams,
    CodeActionResponse,
    Command,
    NumberOrString,
};

use super::super::backend::Backend;

pub async fn handle_code_action(
    backend: &Backend,
    params: CodeActionParams,
) -> Result<Option<CodeActionResponse>> {
    let uri = &params.text_document.uri;
    let diagnostics = &params.context.diagnostics;

    tracing::debug!(uri = %uri, "Code Action request");

    if !backend.wait_for_translations().await {
        tracing::debug!("Code Action request - translations not indexed yet");
        return Ok(Some(vec![]));
    }

    let Some(file_path) = Backend::uri_to_path(uri) else {
        return Ok(Some(vec![]));
    };

    let is_translation_file = {
        let config_manager = backend.config_manager.lock().await;
        config_manager
            .file_matcher()
            .is_some_and(|matcher| matcher.is_translation_file(Path::new(&file_path)))
    };

    if is_translation_file {
        let file_path_str = file_path.to_string_lossy();
        return generate_translation_file_code_actions(backend, uri, &file_path_str, diagnostics)
            .await;
    }

    // Source files: no code actions for now
    Ok(Some(vec![]))
}

async fn generate_translation_file_code_actions(
    backend: &Backend,
    uri: &tower_lsp::lsp_types::Url,
    file_path: &str,
    diagnostics: &[tower_lsp::lsp_types::Diagnostic],
) -> Result<Option<CodeActionResponse>> {
    use std::collections::HashSet;

    let key_separator = {
        let config = backend.config_manager.lock().await;
        config.get_settings().key_separator.clone()
    };

    let used_keys: HashSet<String> = {
        let db = backend.state.db.lock().await;
        let source_files = backend.state.source_files.lock().await;
        let source_file_vec: Vec<_> = source_files.values().copied().collect();
        drop(source_files);

        let mut keys = HashSet::new();
        for source_file in source_file_vec {
            let key_usages =
                crate::syntax::analyze_source(&*db, source_file, key_separator.clone());
            for usage in key_usages {
                keys.insert(usage.key(&*db).text(&*db).clone());
            }
        }
        keys
    };

    let unused_key_count = {
        let db = backend.state.db.lock().await;
        let translations = backend.state.translations.lock().await;

        let Some(translation) = translations.iter().find(|t| t.file_path(&*db) == file_path) else {
            tracing::debug!("Translation file not found: {}", file_path);
            return Ok(Some(vec![]));
        };

        let all_keys = translation.keys(&*db).clone();
        drop(translations);
        drop(db);

        all_keys
            .keys()
            .filter(|key| !crate::ide::diagnostics::is_key_used(key, &used_keys, &key_separator))
            .count()
    };

    if unused_key_count == 0 {
        tracing::debug!("No unused translation keys");
        return Ok(Some(vec![]));
    }

    tracing::debug!(unused_key_count, "Found unused translation keys");

    let unused_key_diagnostics: Vec<_> = diagnostics
        .iter()
        .filter(|d| {
            d.code.as_ref().is_some_and(
                |c| matches!(c, NumberOrString::String(s) if s == "unused-translation-key"),
            )
        })
        .cloned()
        .collect();

    let action = CodeAction {
        title: format!("Delete {unused_key_count} unused translation key(s)"),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: if unused_key_diagnostics.is_empty() {
            None
        } else {
            Some(unused_key_diagnostics)
        },
        is_preferred: Some(true),
        disabled: None,
        edit: None,
        command: Some(Command {
            title: format!("Delete {unused_key_count} unused translation key(s)"),
            command: "i18n.deleteUnusedKeys".to_string(),
            arguments: Some(vec![serde_json::json!({
                "uri": uri.to_string()
            })]),
        }),
        data: None,
    };

    Ok(Some(vec![CodeActionOrCommand::CodeAction(action)]))
}
