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

    // Code Action を生成（全言語対象）
    // TODO: primary_language の設定対応
    let actions = crate::ide::code_actions::generate_code_actions(
        &key_text,
        &all_languages,
        &missing_languages,
        None, // primary_language - 将来的に設定から取得
    );

    tracing::debug!("Generated {} code actions for key: {}", actions.len(), key_text);

    Ok(Some(actions))
}
