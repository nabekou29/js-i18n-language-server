//! Execute command handler for `workspace/executeCommand` requests

use std::collections::HashMap;

use serde::Deserialize;
use serde_json::Value;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::{
    ExecuteCommandParams,
    MessageType,
    Position,
    Range,
    ShowDocumentParams,
    TextEdit,
    Url,
    WorkspaceEdit,
};

use super::super::backend::Backend;

/// Create a `TextEdit` that replaces the entire file content
#[allow(clippy::cast_possible_truncation)] // File content won't exceed 4 billion lines
fn create_full_file_text_edit(original_text: &str, new_text: String) -> TextEdit {
    let line_count = original_text.lines().count();
    let last_line = original_text.lines().last().unwrap_or("");

    TextEdit {
        range: Range {
            start: Position { line: 0, character: 0 },
            end: Position {
                line: line_count.saturating_sub(1) as u32,
                character: last_line.len() as u32,
            },
        },
        new_text,
    }
}

/// Apply a workspace edit with a single file change
async fn apply_single_file_edit(backend: &Backend, uri: Url, text_edit: TextEdit) {
    let mut changes = HashMap::new();
    changes.insert(uri, vec![text_edit]);

    let edit_result = backend
        .client
        .apply_edit(WorkspaceEdit { changes: Some(changes), ..Default::default() })
        .await;

    if let Err(e) = edit_result {
        tracing::error!("Failed to apply workspace edit: {e}");
    }
}

#[allow(clippy::single_match_else)] // More commands may be added in the future
pub async fn handle_execute_command(
    backend: &Backend,
    params: ExecuteCommandParams,
) -> Result<Option<Value>> {
    tracing::debug!(command = %params.command, "Execute Command request");

    match params.command.as_str() {
        "i18n.editTranslation" => handle_edit_translation(backend, Some(params.arguments)).await,
        "i18n.getDecorations" => handle_get_decorations(backend, Some(params.arguments)).await,
        "i18n.setCurrentLanguage" => {
            handle_set_current_language(backend, Some(params.arguments)).await
        }
        "i18n.deleteUnusedKeys" => handle_delete_unused_keys(backend, Some(params.arguments)).await,
        _ => {
            tracing::warn!("Unknown command: {}", params.command);
            Ok(None)
        }
    }
}

/// Open translation file and position cursor at the key's value.
///
/// If the key doesn't exist, inserts a placeholder and opens the file.
/// If the key exists, opens the file and moves cursor to value position.
async fn handle_edit_translation(
    backend: &Backend,
    arguments: Option<Vec<Value>>,
) -> Result<Option<Value>> {
    let args = arguments.unwrap_or_default();

    let lang = args.first().and_then(|v| v.as_str());
    let key = args.get(1).and_then(|v| v.as_str());

    let (Some(lang), Some(key)) = (lang, key) else {
        tracing::warn!("Invalid arguments for i18n.editTranslation");
        return Ok(None);
    };

    tracing::debug!(lang = %lang, key = %key, "Executing i18n.editTranslation");

    let key_separator = {
        let config = backend.config_manager.lock().await;
        config.get_settings().key_separator.clone()
    };

    let db = backend.state.db.lock().await;
    let translations = backend.state.translations.lock().await;

    let Some(translation) = translations.iter().find(|t| t.language(&*db) == lang) else {
        backend
            .client
            .log_message(MessageType::WARNING, format!("Translation file not found for: {lang}"))
            .await;
        return Ok(None);
    };

    let file_path = translation.file_path(&*db).clone();
    let key_exists = translation.keys(&*db).contains_key(key);

    let (insert_result, original_text, cursor_range) = if key_exists {
        // Position cursor before closing quote
        let range = translation.value_ranges(&*db).get(key).map(|r| {
            // r.end points after the closing `"`, so subtract 1 to position before it
            let cursor_char = r.end.character.saturating_sub(1);
            Range {
                start: Position { line: r.end.line, character: cursor_char },
                end: Position { line: r.end.line, character: cursor_char },
            }
        });
        (None, None, range)
    } else {
        // Key doesn't exist - insert via CST manipulation
        let original = translation.json_text(&*db).clone();
        let result =
            crate::ide::code_actions::insert_key_to_json(&*db, translation, key, &key_separator);
        let cursor = result.as_ref().map(|r| r.cursor_range);
        (result, Some(original), cursor)
    };

    drop(translations);
    drop(db);

    let Ok(uri) = Url::from_file_path(&file_path) else {
        tracing::error!("Failed to convert file path to URI: {}", file_path);
        return Ok(None);
    };

    if let (Some(result), Some(original)) = (insert_result, original_text) {
        let text_edit = create_full_file_text_edit(&original, result.new_text);
        apply_single_file_edit(backend, uri.clone(), text_edit).await;
    }

    let show_result = backend
        .client
        .show_document(ShowDocumentParams {
            uri,
            external: Some(false),
            take_focus: Some(true),
            selection: cursor_range,
        })
        .await;

    if let Err(e) = show_result {
        tracing::error!("Failed to show document: {}", e);
    }

    Ok(None)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetDecorationsArgs {
    uri: String,
    language: Option<String>,
    max_length: Option<usize>,
}

/// Returns translation decorations for editor extensions to display inline translations.
async fn handle_get_decorations(
    backend: &Backend,
    arguments: Option<Vec<Value>>,
) -> Result<Option<Value>> {
    let args = arguments.unwrap_or_default();

    let Some(first_arg) = args.first().cloned() else {
        tracing::warn!("Missing arguments for i18n.getDecorations");
        return Ok(Some(serde_json::json!([])));
    };

    let parsed_args: GetDecorationsArgs = match serde_json::from_value(first_arg) {
        Ok(args) => args,
        Err(e) => {
            tracing::warn!("Invalid arguments for i18n.getDecorations: {}", e);
            return Ok(Some(serde_json::json!([])));
        }
    };

    tracing::debug!(
        uri = %parsed_args.uri,
        language = ?parsed_args.language,
        max_length = ?parsed_args.max_length,
        "Executing i18n.getDecorations"
    );

    let Ok(uri) = Url::parse(&parsed_args.uri) else {
        tracing::warn!("Invalid URI: {}", parsed_args.uri);
        return Ok(Some(serde_json::json!([])));
    };

    let Some(file_path) = Backend::uri_to_path(&uri) else {
        tracing::warn!("Failed to convert URI to path: {}", parsed_args.uri);
        return Ok(Some(serde_json::json!([])));
    };

    let config = backend.config_manager.lock().await;
    let settings = config.get_settings();
    let max_length = parsed_args.max_length.unwrap_or(settings.virtual_text.max_length);
    let primary_languages = settings.primary_languages.clone();
    let key_separator = settings.key_separator.clone();
    drop(config);

    let db = backend.state.db.lock().await;
    let source_files = backend.state.source_files.lock().await;

    let Some(source_file) = source_files.get(&file_path).copied() else {
        tracing::debug!("Source file not found: {:?}", file_path);
        return Ok(Some(serde_json::json!([])));
    };

    let translations = backend.state.translations.lock().await;

    // Priority: request arg > currentLanguage > primaryLanguages > first available
    let language = parsed_args.language.clone().or_else(|| {
        let current_language = backend.state.current_language.blocking_lock().clone();
        let sorted_languages = crate::ide::backend::collect_sorted_languages(
            &*db,
            &translations,
            current_language.as_deref(),
            primary_languages.as_deref(),
        );
        sorted_languages.first().cloned()
    });

    let decorations = crate::ide::virtual_text::get_translation_decorations(
        &*db,
        source_file,
        &translations,
        language.as_deref(),
        max_length,
        &key_separator,
    );

    drop(translations);
    drop(source_files);
    drop(db);

    match serde_json::to_value(&decorations) {
        Ok(value) => Ok(Some(value)),
        Err(e) => {
            tracing::error!("Failed to serialize decorations: {}", e);
            Ok(Some(serde_json::json!([])))
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetCurrentLanguageArgs {
    language: Option<String>,
}

/// Set current display language used by virtual text, completion, and code actions.
async fn handle_set_current_language(
    backend: &Backend,
    arguments: Option<Vec<Value>>,
) -> Result<Option<Value>> {
    let args = arguments.unwrap_or_default();

    let parsed_args: SetCurrentLanguageArgs = if let Some(first_arg) = args.first().cloned() {
        match serde_json::from_value(first_arg) {
            Ok(args) => args,
            Err(e) => {
                tracing::warn!("Invalid arguments for i18n.setCurrentLanguage: {}", e);
                return Ok(None);
            }
        }
    } else {
        // No arguments means reset to default
        SetCurrentLanguageArgs { language: None }
    };

    tracing::debug!(
        language = ?parsed_args.language,
        "Executing i18n.setCurrentLanguage"
    );

    let mut current_language = backend.state.current_language.lock().await;
    current_language.clone_from(&parsed_args.language);
    drop(current_language);

    backend
        .client
        .log_message(
            MessageType::INFO,
            format!("Current language set to: {:?}", parsed_args.language),
        )
        .await;

    Ok(None)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeleteUnusedKeysArgs {
    uri: String,
}

/// Delete all unused translation keys from a translation file.
#[allow(clippy::too_many_lines)]
async fn handle_delete_unused_keys(
    backend: &Backend,
    arguments: Option<Vec<Value>>,
) -> Result<Option<Value>> {
    use std::collections::HashSet;
    use std::path::Path;

    let args = arguments.unwrap_or_default();

    let Some(first_arg) = args.first().cloned() else {
        tracing::warn!("Missing arguments for i18n.deleteUnusedKeys");
        return Ok(None);
    };

    let parsed_args: DeleteUnusedKeysArgs = match serde_json::from_value(first_arg) {
        Ok(args) => args,
        Err(e) => {
            tracing::warn!("Invalid arguments for i18n.deleteUnusedKeys: {e}");
            return Ok(None);
        }
    };

    tracing::debug!(uri = %parsed_args.uri, "Executing i18n.deleteUnusedKeys");

    let Ok(uri) = Url::parse(&parsed_args.uri) else {
        tracing::warn!("Invalid URI: {}", parsed_args.uri);
        return Ok(None);
    };

    let Some(file_path) = Backend::uri_to_path(&uri) else {
        tracing::warn!("Failed to convert URI to path: {}", parsed_args.uri);
        return Ok(None);
    };

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

    let (json_text, unused_keys) = {
        let db = backend.state.db.lock().await;
        let translations = backend.state.translations.lock().await;

        let file_path_str = file_path.to_string_lossy().to_string();
        let Some(translation) = translations.iter().find(|t| t.file_path(&*db) == &file_path_str)
        else {
            tracing::warn!("Translation file not found: {:?}", file_path);
            return Ok(None);
        };

        let json_text = translation.json_text(&*db).clone();
        let all_keys = translation.keys(&*db).clone();
        drop(translations);
        drop(db);

        let unused: Vec<String> = all_keys
            .keys()
            .filter(|key| !crate::ide::diagnostics::is_key_used(key, &used_keys, &key_separator))
            .cloned()
            .collect();

        (json_text, unused)
    };

    if unused_keys.is_empty() {
        backend.client.log_message(MessageType::INFO, "No unused translation keys found").await;
        return Ok(Some(serde_json::json!({
            "deletedCount": 0,
            "deletedKeys": []
        })));
    }

    let Some(result) = crate::ide::code_actions::delete_keys_from_json_text(
        &json_text,
        &unused_keys,
        &key_separator,
    ) else {
        tracing::error!("Failed to delete keys from JSON");
        return Ok(None);
    };

    tracing::info!(deleted_count = result.deleted_count, "Deleting unused translation keys");

    let text_edit = create_full_file_text_edit(&json_text, result.new_text.clone());
    apply_single_file_edit(backend, uri, text_edit).await;

    backend.reload_translation_file(Path::new(&file_path)).await;
    backend.send_unused_key_diagnostics().await;

    let deleted_count = result.deleted_count;
    backend
        .client
        .log_message(
            MessageType::INFO,
            format!("Deleted {deleted_count} unused translation key(s)"),
        )
        .await;

    Ok(Some(serde_json::json!({
        "deletedCount": result.deleted_count,
        "deletedKeys": result.deleted_keys
    })))
}
