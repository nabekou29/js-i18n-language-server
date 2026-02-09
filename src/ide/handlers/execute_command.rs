//! Execute command handler for `workspace/executeCommand` requests

use std::collections::HashMap;

use serde::Deserialize;
use serde_json::Value;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::{
    ExecuteCommandParams,
    Position,
    TextEdit,
    Url,
    WorkspaceEdit,
};

use super::super::backend::Backend;

/// Parse the first argument from a command's argument list.
fn parse_command_args<T: serde::de::DeserializeOwned>(
    arguments: Option<Vec<Value>>,
    command_name: &str,
) -> Option<T> {
    let first_arg = arguments.unwrap_or_default().into_iter().next().or_else(|| {
        tracing::warn!("Missing arguments for {command_name}");
        None
    })?;
    serde_json::from_value(first_arg)
        .map_err(|e| {
            tracing::warn!("Invalid arguments for {command_name}: {e}");
        })
        .ok()
}

use crate::ide::code_actions::create_full_file_text_edit;

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
        "i18n.deleteUnusedKeys" => handle_delete_unused_keys(backend, Some(params.arguments)).await,
        "i18n.getKeyAtPosition" => {
            handle_get_key_at_position(backend, Some(params.arguments)).await
        }
        "i18n.getTranslationValue" => {
            handle_get_translation_value(backend, Some(params.arguments)).await
        }
        // No-op: handled by the client (code action trigger for edit translation)
        "i18n.executeClientEditTranslation" => Ok(None),
        "i18n.getDecorations" => handle_get_decorations(backend, Some(params.arguments)).await,
        "i18n.getCurrentLanguage" => handle_get_current_language(backend).await,
        "i18n.setCurrentLanguage" => {
            handle_set_current_language(backend, Some(params.arguments)).await
        }
        "i18n.getAvailableLanguages" => handle_get_available_languages(backend).await,
        _ => {
            tracing::warn!("Unknown command: {}", params.command);
            Ok(None)
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EditTranslationArgs {
    lang: String,
    key: String,
    value: String,
}

/// Edit a translation value directly. Inserts the key if it doesn't exist.
async fn handle_edit_translation(
    backend: &Backend,
    arguments: Option<Vec<Value>>,
) -> Result<Option<Value>> {
    let Some(parsed_args) =
        parse_command_args::<EditTranslationArgs>(arguments, "i18n.editTranslation")
    else {
        return Ok(None);
    };

    tracing::debug!(
        lang = %parsed_args.lang,
        key = %parsed_args.key,
        "Executing i18n.editTranslation"
    );

    let key_separator = backend.get_key_separator().await;

    let db = backend.state.db.lock().await;
    let translations = backend.state.translations.lock().await;

    let Some(translation) = translations.iter().find(|t| t.language(&*db) == parsed_args.lang)
    else {
        tracing::warn!(lang = %parsed_args.lang, "translation file not found");
        return Ok(None);
    };

    let file_path = translation.file_path(&*db).clone();
    let key_exists = translation.keys(&*db).contains_key(parsed_args.key.as_str());
    let original_text = translation.json_text(&*db).clone();

    let result = if key_exists {
        crate::ide::code_actions::update_key_in_json_text(
            &original_text,
            &parsed_args.key,
            &parsed_args.value,
            &key_separator,
        )
    } else {
        crate::ide::code_actions::insert_key_to_json(
            &*db,
            translation,
            &parsed_args.key,
            &parsed_args.value,
            &key_separator,
        )
    };

    drop(translations);
    drop(db);

    let Some(result) = result else {
        tracing::error!("Failed to edit translation key: {}", parsed_args.key);
        return Ok(None);
    };

    let Ok(uri) = Url::from_file_path(&file_path) else {
        tracing::error!("Failed to convert file path to URI: {}", file_path);
        return Ok(None);
    };

    let text_edit = create_full_file_text_edit(&original_text, result.new_text);
    apply_single_file_edit(backend, uri, text_edit).await;

    // State sync (reload, diagnostics, decorations) is handled by the
    // didChange notification that the client sends after applying the edit.

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
    let Some(parsed_args) =
        parse_command_args::<DeleteUnusedKeysArgs>(arguments, "i18n.deleteUnusedKeys")
    else {
        return Ok(None);
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

    let key_separator = backend.get_key_separator().await;
    let used_keys = backend.collect_used_keys(&key_separator).await;

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
        tracing::info!("no unused translation keys found");
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

    // State sync (reload, diagnostics, decorations) is handled by the
    // didChange notification that the client sends after applying the edit.

    Ok(Some(serde_json::json!({
        "deletedCount": result.deleted_count,
        "deletedKeys": result.deleted_keys
    })))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetKeyAtPositionArgs {
    uri: String,
    position: Position,
}

/// Returns the translation key at the given cursor position.
async fn handle_get_key_at_position(
    backend: &Backend,
    arguments: Option<Vec<Value>>,
) -> Result<Option<Value>> {
    let Some(parsed_args) =
        parse_command_args::<GetKeyAtPositionArgs>(arguments, "i18n.getKeyAtPosition")
    else {
        return Ok(None);
    };

    tracing::debug!(
        uri = %parsed_args.uri,
        line = parsed_args.position.line,
        character = parsed_args.position.character,
        "Executing i18n.getKeyAtPosition"
    );

    let Ok(uri) = Url::parse(&parsed_args.uri) else {
        tracing::warn!("Invalid URI: {}", parsed_args.uri);
        return Ok(None);
    };

    let Some(file_path) = Backend::uri_to_path(&uri) else {
        return Ok(None);
    };

    let source_position = crate::types::SourcePosition::from(parsed_args.position);

    let key = backend.get_key_at_position(&file_path, source_position).await;

    Ok(key.map(|k| serde_json::json!({ "key": k })))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetTranslationValueArgs {
    lang: String,
    key: String,
}

/// Returns the value of a translation key for a given language.
async fn handle_get_translation_value(
    backend: &Backend,
    arguments: Option<Vec<Value>>,
) -> Result<Option<Value>> {
    let Some(parsed_args) =
        parse_command_args::<GetTranslationValueArgs>(arguments, "i18n.getTranslationValue")
    else {
        return Ok(None);
    };

    tracing::debug!(
        lang = %parsed_args.lang,
        key = %parsed_args.key,
        "Executing i18n.getTranslationValue"
    );

    let value = {
        let (db, translations) = backend.state.lock_db_and_translations().await;

        translations
            .iter()
            .find(|t| t.language(&*db) == parsed_args.lang)
            .and_then(|t| t.keys(&*db).get(parsed_args.key.as_str()).cloned())
    };

    Ok(value.map(|v| serde_json::json!({ "value": v })))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetDecorationsArgs {
    uri: String,
    language: Option<String>,
}

/// Returns translation decorations for editor extensions to display inline translations.
async fn handle_get_decorations(
    backend: &Backend,
    arguments: Option<Vec<Value>>,
) -> Result<Option<Value>> {
    let Some(parsed_args) =
        parse_command_args::<GetDecorationsArgs>(arguments, "i18n.getDecorations")
    else {
        return Ok(Some(serde_json::json!([])));
    };

    tracing::debug!(
        uri = %parsed_args.uri,
        language = ?parsed_args.language,
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
    let current_language = backend.state.current_language.lock().await.clone();
    let language = parsed_args.language.clone().or_else(|| {
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
        &key_separator,
    );

    drop(translations);
    drop(source_files);
    drop(db);

    match serde_json::to_value(&decorations) {
        Ok(value) => Ok(Some(value)),
        Err(e) => {
            tracing::error!("Failed to serialize decorations: {e}");
            Ok(Some(serde_json::json!([])))
        }
    }
}

/// Returns the current display language with fallback resolution.
/// Priority: currentLanguage > primaryLanguages > first available
async fn handle_get_current_language(backend: &Backend) -> Result<Option<Value>> {
    let primary_languages = {
        let config = backend.config_manager.lock().await;
        config.get_settings().primary_languages.clone()
    };

    let current_language = backend.state.current_language.lock().await.clone();

    let language = if let Some(lang) = current_language {
        Some(lang)
    } else {
        let (db, translations) = backend.state.lock_db_and_translations().await;
        crate::ide::backend::collect_sorted_languages(
            &*db,
            &translations,
            None,
            primary_languages.as_deref(),
        )
        .first()
        .cloned()
    };

    tracing::debug!(language = ?language, "Executing i18n.getCurrentLanguage");

    Ok(Some(serde_json::json!({ "language": language })))
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
    let parsed_args: SetCurrentLanguageArgs =
        parse_command_args(arguments, "i18n.setCurrentLanguage")
            .unwrap_or(SetCurrentLanguageArgs { language: None });

    tracing::debug!(
        language = ?parsed_args.language,
        "Executing i18n.setCurrentLanguage"
    );

    let mut current_language = backend.state.current_language.lock().await;
    current_language.clone_from(&parsed_args.language);
    drop(current_language);

    tracing::info!(language = ?parsed_args.language, "current language updated");

    backend.send_decorations_changed().await;

    Ok(None)
}

/// Get all available languages from translation files.
async fn handle_get_available_languages(backend: &Backend) -> Result<Option<Value>> {
    let config = backend.config_manager.lock().await;
    let settings = config.get_settings();
    let primary_languages = settings.primary_languages.clone();
    drop(config);

    let current_language = backend.state.current_language.lock().await.clone();
    let (db, translations) = backend.state.lock_db_and_translations().await;

    let languages = crate::ide::backend::collect_sorted_languages(
        &*db,
        &translations,
        current_language.as_deref(),
        primary_languages.as_deref(),
    );

    tracing::debug!(languages = ?languages, "Executing i18n.getAvailableLanguages");

    Ok(Some(serde_json::json!({ "languages": languages })))
}
