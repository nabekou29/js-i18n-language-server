//! LSP feature handlers: completion, hover, `goto_definition`, references, rename.

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::{
    CompletionParams,
    CompletionResponse,
    GotoDefinitionParams,
    GotoDefinitionResponse,
    Hover,
    HoverContents,
    HoverParams,
    Location,
    MarkupContent,
    MarkupKind,
    PrepareRenameResponse,
    ReferenceParams,
    RenameParams,
    TextDocumentPositionParams,
    WorkspaceEdit,
};

use super::super::backend::Backend;

pub async fn handle_completion(
    backend: &Backend,
    params: CompletionParams,
) -> Result<Option<CompletionResponse>> {
    let uri = params.text_document_position.text_document.uri;
    let position = params.text_document_position.position;

    tracing::debug!(uri = %uri, line = position.line, character = position.character, "Completion request");

    if !backend.wait_for_translations().await {
        tracing::debug!("Completion request - translations not indexed yet");
        return Ok(None);
    }

    let Some(file_path) = Backend::uri_to_path(&uri) else {
        return Ok(None);
    };

    let source_file = {
        let source_files = backend.state.source_files.lock().await;
        source_files.get(&file_path).copied()
    };

    let Some(source_file) = source_file else {
        tracing::debug!("Source file not found: {}", file_path.display());
        return Ok(None);
    };

    // Acquire config before db to respect lock ordering (config_manager → db → translations)
    let (key_separator, primary_languages) = {
        let settings = backend.config_manager.lock().await.get_settings().clone();
        (settings.key_separator, settings.primary_languages)
    };

    let db = backend.state.db.lock().await;
    let text = source_file.text(&*db);
    let language = source_file.language(&*db);

    // Use tree-sitter based extraction (supports renamed functions, ignores comments)
    let completion_context = crate::ide::completion::extract_completion_context_tree_sitter(
        text,
        language,
        position.line,
        position.character,
        &key_separator,
    );

    let Some(context) = completion_context else {
        tracing::debug!("Not in translation function context");
        return Ok(None);
    };

    tracing::debug!(
        partial_key = ?context.partial_key,
        quote_context = ?context.quote_context,
        "Extracted completion context"
    );

    let translations = backend.state.translations.lock().await;
    let partial_key_opt =
        if context.partial_key.is_empty() { None } else { Some(context.partial_key.as_str()) };

    let current_language = backend.state.current_language.lock().await.clone();
    let sorted_languages = crate::ide::backend::collect_sorted_languages(
        &*db,
        &translations,
        current_language.as_deref(),
        primary_languages.as_deref(),
    );
    let effective_language = sorted_languages.first().cloned();

    let items = crate::ide::completion::generate_completions(
        &*db,
        &translations,
        partial_key_opt,
        &context.quote_context,
        context.key_prefix.as_deref(),
        effective_language.as_deref(),
        &key_separator,
    );
    drop(db);
    drop(translations);

    tracing::debug!("Generated {} completion items", items.len());

    if items.is_empty() { Ok(None) } else { Ok(Some(CompletionResponse::Array(items))) }
}

pub async fn handle_hover(backend: &Backend, params: HoverParams) -> Result<Option<Hover>> {
    let uri = params.text_document_position_params.text_document.uri;
    let position = params.text_document_position_params.position;

    tracing::debug!(uri = %uri, line = position.line, character = position.character, "Hover request");

    if !backend.wait_for_translations().await {
        tracing::debug!("Hover request timeout - translations not indexed yet");
        return Ok(None);
    }

    let Some(file_path) = Backend::uri_to_path(&uri) else {
        return Ok(None);
    };

    let source_position = crate::types::SourcePosition::from(position);

    let Some(key_text) = backend.get_key_at_position(&file_path, source_position).await else {
        tracing::debug!("No translation key found at position");
        return Ok(None);
    };

    let hover_text = {
        let config = backend.config_manager.lock().await;
        let settings = config.get_settings();
        let key_separator = settings.key_separator.clone();
        let primary_languages = settings.primary_languages.clone();
        drop(config);

        let current_language = backend.state.current_language.lock().await.clone();
        let db = backend.state.db.lock().await;
        let key = crate::interned::TransKey::new(&*db, key_text.clone());
        let translations = backend.state.translations.lock().await;
        crate::ide::hover::generate_hover_content(
            &*db,
            key,
            &translations,
            &key_separator,
            current_language.as_deref(),
            primary_languages.as_deref(),
        )
    };

    let Some(hover_text) = hover_text else {
        tracing::debug!("No translations found for key: {}", key_text);
        return Ok(None);
    };

    tracing::debug!("Generated hover content for key: {}", key_text);

    Ok(Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: hover_text,
        }),
        range: None,
    }))
}

/// Handles `textDocument/definition` request.
pub async fn handle_goto_definition(
    backend: &Backend,
    params: GotoDefinitionParams,
) -> Result<Option<GotoDefinitionResponse>> {
    let uri = params.text_document_position_params.text_document.uri;
    let position = params.text_document_position_params.position;

    tracing::debug!(uri = %uri, line = position.line, character = position.character, "Goto Definition request");

    if !backend.wait_for_translations().await {
        tracing::debug!("Goto Definition request - translations not indexed yet");
        return Ok(None);
    }

    let Some(file_path) = Backend::uri_to_path(&uri) else {
        return Ok(None);
    };

    let source_position = crate::types::SourcePosition::from(position);

    let Some(key_text) = backend.get_key_at_position(&file_path, source_position).await else {
        tracing::debug!("No translation key found at position");
        return Ok(None);
    };

    let locations = {
        let key_separator = backend.get_key_separator().await;
        let db = backend.state.db.lock().await;
        let key = crate::interned::TransKey::new(&*db, key_text);
        let translations = backend.state.translations.lock().await;
        crate::ide::goto_definition::find_definitions(&*db, key, &translations, &key_separator)
    };

    tracing::debug!("Found {} definitions for key", locations.len());

    if locations.is_empty() { Ok(None) } else { Ok(Some(GotoDefinitionResponse::Array(locations))) }
}

pub async fn handle_references(
    backend: &Backend,
    params: ReferenceParams,
) -> Result<Option<Vec<Location>>> {
    let uri = params.text_document_position.text_document.uri;
    let position = params.text_document_position.position;

    tracing::debug!(uri = %uri, line = position.line, character = position.character, "References request");

    if !backend.workspace_indexer.is_indexing_completed() {
        tracing::debug!("References request - indexing not completed, returning empty results");
        return Ok(Some(vec![]));
    }

    let Some(file_path) = Backend::uri_to_path(&uri) else {
        return Ok(None);
    };

    let source_position = crate::types::SourcePosition::from(position);

    let Some(key_text) = backend.get_key_at_position(&file_path, source_position).await else {
        tracing::debug!("No translation key found at position");
        return Ok(None);
    };

    let locations = {
        let key_separator = backend.get_key_separator().await;
        let db = backend.state.db.lock().await;
        let key = crate::interned::TransKey::new(&*db, key_text.clone());
        let source_files = backend.state.source_files.lock().await;
        crate::ide::references::find_references(&*db, key, &source_files, &key_separator)
    };

    tracing::debug!("Found {} references for key: {}", locations.len(), key_text);

    if locations.is_empty() { Ok(None) } else { Ok(Some(locations)) }
}

pub async fn handle_prepare_rename(
    backend: &Backend,
    params: TextDocumentPositionParams,
) -> Result<Option<PrepareRenameResponse>> {
    let uri = params.text_document.uri;
    let position = params.position;

    tracing::debug!(uri = %uri, line = position.line, character = position.character, "Prepare rename request");

    if !backend.workspace_indexer.is_indexing_completed() {
        return Ok(None);
    }

    let Some(file_path) = Backend::uri_to_path(&uri) else {
        return Ok(None);
    };

    let source_file = {
        let source_files = backend.state.source_files.lock().await;
        source_files.get(&file_path).copied()
    };

    let key_separator = backend.get_key_separator().await;
    let db = backend.state.db.lock().await;
    let source_position = crate::types::SourcePosition::from(position);

    if let Some(source_file) = source_file {
        let usages = crate::syntax::analyze_source(&*db, source_file, key_separator);

        for usage in usages {
            let range = usage.range(&*db);
            if range.contains(source_position) {
                let key_text = usage.key(&*db).text(&*db).clone();

                return Ok(Some(PrepareRenameResponse::RangeWithPlaceholder {
                    range: range.to_unquoted_range(),
                    placeholder: key_text,
                }));
            }
        }
    } else {
        // Translation file: find key at cursor position
        let translations = backend.state.translations.lock().await;
        let file_path_str = file_path.to_string_lossy();

        if let Some(translation) =
            translations.iter().find(|t| t.file_path(&*db) == file_path_str.as_ref())
        {
            if let Some(key) = translation.key_at_position(&*db, source_position) {
                let key_text = key.text(&*db).clone();

                // Look up key range in key_ranges
                if let Some(range) = translation.key_ranges(&*db).get(&key_text) {
                    return Ok(Some(PrepareRenameResponse::RangeWithPlaceholder {
                        range: range.to_unquoted_range(),
                        placeholder: key_text,
                    }));
                }
            }
        }
    }

    Ok(None)
}

pub async fn handle_rename(
    backend: &Backend,
    params: RenameParams,
) -> Result<Option<WorkspaceEdit>> {
    let uri = params.text_document_position.text_document.uri;
    let position = params.text_document_position.position;
    let new_name = params.new_name;

    tracing::debug!(uri = %uri, new_name = %new_name, "Rename request");

    if !backend.workspace_indexer.is_indexing_completed() {
        return Ok(None);
    }

    let Some(file_path) = Backend::uri_to_path(&uri) else {
        return Ok(None);
    };

    let source_position = crate::types::SourcePosition::from(position);

    let Some(old_key) = backend.get_key_at_position(&file_path, source_position).await else {
        return Ok(None);
    };

    let settings = backend.config_manager.lock().await.get_settings().clone();
    let db = backend.state.db.lock().await;
    let translations = backend.state.translations.lock().await;
    let source_files = backend.state.source_files.lock().await;

    let edit = crate::ide::rename::compute_rename_edits(
        &*db,
        &old_key,
        &new_name,
        &translations,
        &source_files,
        &settings.key_separator,
        settings.namespace_separator.as_deref(),
    );

    Ok(Some(edit))
}
