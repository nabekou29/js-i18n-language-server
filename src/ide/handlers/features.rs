//! LSP 機能ハンドラー
//!
//! `completion`, `hover`, `goto_definition`, `references` の処理を担当します。

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
    ReferenceParams,
};

use super::super::backend::Backend;

/// `textDocument/completion` リクエストを処理
pub async fn handle_completion(
    backend: &Backend,
    params: CompletionParams,
) -> Result<Option<CompletionResponse>> {
    let uri = params.text_document_position.text_document.uri;
    let position = params.text_document_position.position;

    tracing::debug!(uri = %uri, line = position.line, character = position.character, "Completion request");

    // 翻訳データが必要なため、インデックス完了を待つ
    if !backend.wait_for_translations().await {
        tracing::debug!("Completion request - translations not indexed yet");
        return Ok(None);
    }

    // ファイルパスを取得
    let Some(file_path) = Backend::uri_to_path(&uri) else {
        return Ok(None);
    };

    // SourceFile を取得
    let source_file = {
        let source_files = backend.state.source_files.lock().await;
        source_files.get(&file_path).copied()
    };

    let Some(source_file) = source_file else {
        tracing::debug!("Source file not found: {}", file_path.display());
        return Ok(None);
    };

    // ファイルの内容を取得してコンテキストを抽出
    let db = backend.state.db.lock().await;
    let text = source_file.text(&*db);
    let language = source_file.language(&*db);
    let key_separator = backend.config_manager.lock().await.get_settings().key_separator.clone();

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

    // 補完候補を生成
    let translations = backend.state.translations.lock().await;
    let partial_key_opt =
        if context.partial_key.is_empty() { None } else { Some(context.partial_key.as_str()) };

    // 有効な言語を決定（currentLanguage → primaryLanguages → 最初の言語）
    let current_language = backend.state.current_language.lock().await.clone();
    let primary_languages =
        backend.config_manager.lock().await.get_settings().primary_languages.clone();
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

/// `textDocument/hover` リクエストを処理
pub async fn handle_hover(backend: &Backend, params: HoverParams) -> Result<Option<Hover>> {
    let uri = params.text_document_position_params.text_document.uri;
    let position = params.text_document_position_params.position;

    tracing::debug!(uri = %uri, line = position.line, character = position.character, "Hover request");

    // 翻訳データが必要なため、インデックス完了を待つ
    // タイムアウトした場合は hover情報なしを返す
    if !backend.wait_for_translations().await {
        tracing::debug!("Hover request timeout - translations not indexed yet");
        return Ok(None);
    }

    // ファイルパスを取得
    let Some(file_path) = Backend::uri_to_path(&uri) else {
        return Ok(None);
    };

    let source_position = crate::types::SourcePosition::from(position);

    // 翻訳キーを取得
    let Some(key_text) = backend.get_key_at_position(&file_path, source_position).await else {
        tracing::debug!("No translation key found at position");
        return Ok(None);
    };

    // 翻訳内容を取得
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

/// `textDocument/definition` リクエストを処理
pub async fn handle_goto_definition(
    backend: &Backend,
    params: GotoDefinitionParams,
) -> Result<Option<GotoDefinitionResponse>> {
    let uri = params.text_document_position_params.text_document.uri;
    let position = params.text_document_position_params.position;

    tracing::debug!(uri = %uri, line = position.line, character = position.character, "Goto Definition request");

    // 翻訳データが必要なため、インデックス完了を待つ
    if !backend.wait_for_translations().await {
        tracing::debug!("Goto Definition request - translations not indexed yet");
        return Ok(None);
    }

    // ファイルパスを取得
    let Some(file_path) = Backend::uri_to_path(&uri) else {
        return Ok(None);
    };

    let source_position = crate::types::SourcePosition::from(position);

    // 翻訳キーを取得
    let Some(key_text) = backend.get_key_at_position(&file_path, source_position).await else {
        tracing::debug!("No translation key found at position");
        return Ok(None);
    };

    // 翻訳ファイル内の定義を検索
    let locations = {
        let key_separator =
            backend.config_manager.lock().await.get_settings().key_separator.clone();
        let db = backend.state.db.lock().await;
        let key = crate::interned::TransKey::new(&*db, key_text);
        let translations = backend.state.translations.lock().await;
        crate::ide::goto_definition::find_definitions(&*db, key, &translations, &key_separator)
    };

    tracing::debug!("Found {} definitions for key", locations.len());

    if locations.is_empty() { Ok(None) } else { Ok(Some(GotoDefinitionResponse::Array(locations))) }
}

/// `textDocument/references` リクエストを処理
pub async fn handle_references(
    backend: &Backend,
    params: ReferenceParams,
) -> Result<Option<Vec<Location>>> {
    let uri = params.text_document_position.text_document.uri;
    let position = params.text_document_position.position;

    tracing::debug!(uri = %uri, line = position.line, character = position.character, "References request");

    // 全インデックス完了をチェック（待機しない）
    if !backend.workspace_indexer.is_indexing_completed() {
        tracing::debug!("References request - indexing not completed, returning empty results");
        return Ok(Some(vec![]));
    }

    // ファイルパスを取得
    let Some(file_path) = Backend::uri_to_path(&uri) else {
        return Ok(None);
    };

    let source_position = crate::types::SourcePosition::from(position);

    // 翻訳キーを取得
    let Some(key_text) = backend.get_key_at_position(&file_path, source_position).await else {
        tracing::debug!("No translation key found at position");
        return Ok(None);
    };

    // 全ソースファイルから参照を検索
    let locations = {
        let db = backend.state.db.lock().await;
        let key = crate::interned::TransKey::new(&*db, key_text.clone());
        let source_files = backend.state.source_files.lock().await;
        let key_separator =
            backend.config_manager.lock().await.get_settings().key_separator.clone();
        crate::ide::references::find_references(&*db, key, &source_files, &key_separator)
    };

    tracing::debug!("Found {} references for key: {}", locations.len(), key_text);

    if locations.is_empty() { Ok(None) } else { Ok(Some(locations)) }
}
