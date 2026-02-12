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
        return generate_translation_file_code_actions(
            backend,
            uri,
            &file_path_str,
            diagnostics,
            params.range.start,
        )
        .await;
    }

    let position = params.range.start;
    let source_position = crate::types::SourcePosition::from(position);

    let Some(key_context) = backend.get_key_at_position(&file_path, source_position).await else {
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
            &key_context.key_text,
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
            &key_context.key_text,
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
    position: tower_lsp::lsp_types::Position,
) -> Result<Option<CodeActionResponse>> {
    let key_separator = backend.get_key_separator().await;
    let used_keys = backend.collect_used_keys(&key_separator).await;

    let mut actions: Vec<CodeActionOrCommand> = Vec::new();

    {
        let settings = backend.config_manager.lock().await.get_settings().clone();
        let db = backend.state.db.lock().await;
        let translations = backend.state.translations.lock().await;

        let Some(translation) = translations.iter().find(|t| t.file_path(&*db) == file_path) else {
            tracing::debug!("Translation file not found: {}", file_path);
            return Ok(Some(vec![]));
        };

        // Delete key at cursor position
        let source_position = crate::types::SourcePosition::from(position);
        if let Some(key) = translation.key_at_position(&*db, source_position) {
            let key_text = key.text(&*db).clone();
            if let Some(action) = crate::ide::code_actions::generate_delete_key_code_action(
                &*db,
                &key_text,
                &translations,
                &settings.key_separator,
                settings.namespace_separator.as_deref(),
            ) {
                let is_unused =
                    !crate::ide::diagnostics::is_key_used(&key_text, &used_keys, &key_separator);
                let action =
                    promote_to_quickfix_if_unused(action, is_unused, diagnostics, position);
                actions.push(action);
            }
        }

        // Delete unused keys
        let all_keys = translation.keys(&*db).clone();
        drop(translations);
        drop(db);

        let unused_key_count = all_keys
            .keys()
            .filter(|key| !crate::ide::diagnostics::is_key_used(key, &used_keys, &key_separator))
            .count();

        if unused_key_count > 0 {
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
                diagnostics: (!unused_key_diagnostics.is_empty()).then_some(unused_key_diagnostics),
                is_preferred: Some(true),
                command: Some(Command {
                    title: format!("Delete {unused_key_count} unused translation key(s)"),
                    command: "i18n.deleteUnusedKeys".to_string(),
                    arguments: Some(vec![serde_json::json!({
                        "uri": uri.to_string()
                    })]),
                }),
                ..Default::default()
            };
            actions.push(CodeActionOrCommand::CodeAction(action));
        }
    }

    Ok(Some(actions))
}

/// Promote a delete-key action to QUICKFIX when the key is unused,
/// so it appears alongside the "Delete N unused key(s)" action in the Quick Fix menu.
fn promote_to_quickfix_if_unused(
    action: CodeActionOrCommand,
    is_unused: bool,
    diagnostics: &[tower_lsp::lsp_types::Diagnostic],
    position: tower_lsp::lsp_types::Position,
) -> CodeActionOrCommand {
    let CodeActionOrCommand::CodeAction(mut ca) = action else {
        return action;
    };

    if !is_unused {
        return CodeActionOrCommand::CodeAction(ca);
    }

    ca.kind = Some(CodeActionKind::QUICKFIX);

    let matching_diag: Vec<_> = diagnostics
        .iter()
        .filter(|d| {
            d.code.as_ref().is_some_and(
                |c| matches!(c, NumberOrString::String(s) if s == "unused-translation-key"),
            ) && d.range.start <= position
                && position <= d.range.end
        })
        .cloned()
        .collect();

    if !matching_diag.is_empty() {
        ca.diagnostics = Some(matching_diag);
    }

    CodeActionOrCommand::CodeAction(ca)
}
