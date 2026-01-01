//! Code Action ハンドラー
//!
//! `textDocument/codeAction` リクエストを処理し、
//! 翻訳キーに対する編集アクションを提供します。

use std::collections::HashSet;

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::{
    CodeActionParams,
    CodeActionResponse,
};

use super::super::backend::Backend;

/// `textDocument/codeAction` リクエストを処理
///
/// 翻訳キー上でのみ Code Action を返します。
/// すべての言語に対して「Edit translation for {lang}」アクションを提供します。
pub async fn handle_code_action(
    backend: &Backend,
    params: CodeActionParams,
) -> Result<Option<CodeActionResponse>> {
    let uri = &params.text_document.uri;
    let position = params.range.start;
    let diagnostics = &params.context.diagnostics;

    tracing::debug!(uri = %uri, line = position.line, character = position.character, "Code Action request");

    // 翻訳データが必要なため、インデックス完了を待つ
    if !backend.wait_for_translations().await {
        tracing::debug!("Code Action request - translations not indexed yet");
        return Ok(Some(vec![]));
    }

    // ファイルパスを取得
    let Some(file_path) = Backend::uri_to_path(uri) else {
        return Ok(Some(vec![]));
    };

    let source_position = crate::types::SourcePosition::from(position);

    // カーソル位置の翻訳キーを取得
    let Some(key_text) = backend.get_key_at_position(&file_path, source_position).await else {
        tracing::debug!("No translation key found at position");
        return Ok(Some(vec![]));
    };

    tracing::debug!(key = %key_text, "Found translation key for code action");

    // すべての利用可能な言語を取得
    let all_languages: Vec<String> = {
        let db = backend.state.db.lock().await;
        let translations = backend.state.translations.lock().await;
        translations.iter().map(|t| t.language(&*db)).collect::<HashSet<_>>().into_iter().collect()
    };

    if all_languages.is_empty() {
        tracing::debug!("No translations available");
        return Ok(Some(vec![]));
    }

    // 診断から missing_languages を抽出
    let missing_languages = crate::ide::code_actions::extract_missing_languages(diagnostics);

    // 有効な言語を決定（currentLanguage → primaryLanguages → 最初の言語）
    let current_language = backend.state.current_language.lock().await.clone();
    let primary_languages =
        backend.config_manager.lock().await.get_settings().primary_languages.clone();
    let effective_language = crate::ide::backend::resolve_effective_language(
        current_language.as_deref(),
        primary_languages.as_deref(),
        &all_languages,
    );

    // Code Action を生成（全言語対象）
    let actions = crate::ide::code_actions::generate_code_actions(
        &key_text,
        &all_languages,
        &missing_languages,
        effective_language.as_deref(),
    );

    tracing::debug!("Generated {} code actions for key: {}", actions.len(), key_text);

    Ok(Some(actions))
}
