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

    let position = params.range.start;
    let source_position = crate::types::SourcePosition::from(position);

    let Some(key_text) = backend.get_key_at_position(&file_path, source_position).await else {
        return Ok(Some(vec![]));
    };

    let mut actions: Vec<CodeActionOrCommand> = Vec::new();

    // Delete key action (always available, no client opt-in needed)
    {
        let settings = backend.config_manager.lock().await.get_settings().clone();
        let db = backend.state.db.lock().await;
        let translations = backend.state.translations.lock().await;
        if let Some(action) = crate::ide::code_actions::generate_delete_key_code_action(
            &*db,
            &key_text,
            &translations,
            &settings.key_separator,
            settings.namespace_separator.as_deref(),
        ) {
            actions.push(action);
        }
    }

    // Edit/Add translation actions (requires client opt-in)
    let code_actions_enabled = *backend.state.code_actions_enabled.lock().await;
    if code_actions_enabled {
        let missing_languages = crate::ide::code_actions::extract_missing_languages(diagnostics);

        let (effective_language, sorted_languages) = {
            let config = backend.config_manager.lock().await;
            let primary_languages = config.get_settings().primary_languages.clone();
            drop(config);

            let current_language = backend.state.current_language.lock().await.clone();
            let db = backend.state.db.lock().await;
            let translations = backend.state.translations.lock().await;

            let sorted = crate::ide::backend::collect_sorted_languages(
                &*db,
                &translations,
                current_language.as_deref(),
                primary_languages.as_deref(),
            );
            drop(translations);
            drop(db);
            let effective = sorted.first().cloned();
            (effective, sorted)
        };

        let edit_actions = crate::ide::code_actions::generate_code_actions(
            &key_text,
            &sorted_languages,
            &missing_languages,
            effective_language.as_deref(),
        );
        actions.extend(edit_actions);
    }

    Ok(Some(actions))
}

async fn generate_translation_file_code_actions(
    backend: &Backend,
    uri: &tower_lsp::lsp_types::Url,
    file_path: &str,
    diagnostics: &[tower_lsp::lsp_types::Diagnostic],
) -> Result<Option<CodeActionResponse>> {
    let key_separator = backend.get_key_separator().await;
    let used_keys = backend.collect_used_keys(&key_separator).await;

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
