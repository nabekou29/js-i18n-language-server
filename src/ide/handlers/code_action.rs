//! Code Action ハンドラー
//!
//! `textDocument/codeAction` リクエストを処理し、
//! 翻訳キーに対する編集アクションを提供します。

use std::path::Path;

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::{
    CodeAction,
    CodeActionKind,
    CodeActionOrCommand,
    CodeActionParams,
    CodeActionResponse,
    Command,
};

use super::super::backend::Backend;

/// `textDocument/codeAction` リクエストを処理
///
/// ソースファイル上の翻訳キーに対する「Edit translation」アクションと、
/// 翻訳ファイル上の未使用キーに対する「Delete unused keys」アクションを提供します。
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

    // 翻訳ファイルかどうか判定
    let is_translation_file = {
        let config_manager = backend.config_manager.lock().await;
        config_manager
            .file_matcher()
            .is_some_and(|matcher| matcher.is_translation_file(Path::new(&file_path)))
    };

    if is_translation_file {
        // 翻訳ファイル用の Code Action
        let file_path_str = file_path.to_string_lossy();
        return generate_translation_file_code_actions(backend, uri, &file_path_str, diagnostics)
            .await;
    }

    // ソースファイル用の Code Action
    let source_position = crate::types::SourcePosition::from(position);

    // カーソル位置の翻訳キーを取得
    let Some(key_text) = backend.get_key_at_position(&file_path, source_position).await else {
        tracing::debug!("No translation key found at position");
        return Ok(Some(vec![]));
    };

    tracing::debug!(key = %key_text, "Found translation key for code action");

    // すべての利用可能な言語をソートして取得
    let current_language = backend.state.current_language.lock().await.clone();
    let primary_languages =
        backend.config_manager.lock().await.get_settings().primary_languages.clone();
    let sorted_languages = {
        let db = backend.state.db.lock().await;
        let translations = backend.state.translations.lock().await;
        crate::ide::backend::collect_sorted_languages(
            &*db,
            &translations,
            current_language.as_deref(),
            primary_languages.as_deref(),
        )
    };

    if sorted_languages.is_empty() {
        tracing::debug!("No translations available");
        return Ok(Some(vec![]));
    }

    // 診断から missing_languages を抽出
    let missing_languages = crate::ide::code_actions::extract_missing_languages(diagnostics);

    // 有効な言語（ソート済みの先頭）
    let effective_language = sorted_languages.first().cloned();

    // Code Action を生成（全言語対象）
    let actions = crate::ide::code_actions::generate_code_actions(
        &key_text,
        &sorted_languages,
        &missing_languages,
        effective_language.as_deref(),
    );

    tracing::debug!("Generated {} code actions for key: {}", actions.len(), key_text);

    Ok(Some(actions))
}

/// 翻訳ファイル用の Code Action を処理
///
/// Translation データから未使用キーの正確な数をカウントし、
/// 削除アクションを生成します。
async fn generate_translation_file_code_actions(
    backend: &Backend,
    uri: &tower_lsp::lsp_types::Url,
    file_path: &str,
    diagnostics: &[tower_lsp::lsp_types::Diagnostic],
) -> Result<Option<CodeActionResponse>> {
    use std::collections::HashSet;

    // 設定から key_separator を取得
    let key_separator = {
        let config = backend.config_manager.lock().await;
        config.get_settings().key_separator.clone()
    };

    // ソースファイルから使用されているキーを収集
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

    // 未使用キーの数をカウント
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

    // カーソル位置の診断をフィルタ（Code Action の diagnostics フィールド用）
    let unused_key_diagnostics: Vec<_> = diagnostics
        .iter()
        .filter(|d| {
            d.code.as_ref().is_some_and(|c| {
                matches!(c, tower_lsp::lsp_types::NumberOrString::String(s) if s == "unused-translation-key")
            })
        })
        .cloned()
        .collect();

    // 削除アクションを生成
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
