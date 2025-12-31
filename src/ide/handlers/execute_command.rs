//! Execute Command ハンドラー
//!
//! `workspace/executeCommand` リクエストを処理し、
//! カスタムコマンドを実行します。

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

/// `workspace/executeCommand` リクエストを処理
#[allow(clippy::single_match_else)] // 将来的にコマンドが増える可能性を考慮
pub async fn handle_execute_command(
    backend: &Backend,
    params: ExecuteCommandParams,
) -> Result<Option<Value>> {
    tracing::debug!(command = %params.command, "Execute Command request");

    match params.command.as_str() {
        "i18n.editTranslation" => handle_edit_translation(backend, Some(params.arguments)).await,
        _ => {
            tracing::warn!("Unknown command: {}", params.command);
            Ok(None)
        }
    }
}

/// `i18n.editTranslation` コマンドを実行
///
/// # Arguments
/// * `arguments[0]` - 言語コード (例: "en", "ja")
/// * `arguments[1]` - 翻訳キー (例: "common.hello")
///
/// # 動作
/// - キーが存在しない場合: プレースホルダーを挿入し、ファイルを開く
/// - キーが存在する場合: ファイルを開き、値の位置にカーソルを移動
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

    // 設定から key_separator を取得
    let key_separator = {
        let config = backend.config_manager.lock().await;
        config.get_settings().key_separator.clone()
    };

    let db = backend.state.db.lock().await;
    let translations = backend.state.translations.lock().await;

    // 指定された言語の翻訳ファイルを検索
    let Some(translation) = translations.iter().find(|t| t.language(&*db) == lang) else {
        backend
            .client
            .log_message(MessageType::WARNING, format!("Translation file not found for: {lang}"))
            .await;
        return Ok(None);
    };

    let file_path = translation.file_path(&*db).clone();
    let key_exists = translation.keys(&*db).contains_key(key);

    // キーの存在有無で動作を分岐
    let (insert_result, original_text, cursor_range) = if key_exists {
        // キーが存在する → 値の末尾（閉じクォートの手前）にカーソル
        let range = translation.value_ranges(&*db).get(key).map(|r| {
            // end は `"` の後の位置なので、1つ前（`"` の手前）にする
            let cursor_char = r.end.character.saturating_sub(1);
            Range {
                start: Position { line: r.end.line, character: cursor_char },
                end: Position { line: r.end.line, character: cursor_char },
            }
        });
        (None, None, range)
    } else {
        // キーが存在しない → CST でキーを挿入
        let original = translation.json_text(&*db).clone();
        let result = crate::ide::code_actions::insert_key_to_json(
            &*db,
            translation,
            key,
            &key_separator,
        );
        let cursor = result.as_ref().map(|r| r.cursor_range);
        (result, Some(original), cursor)
    };

    // ロックを解放
    drop(translations);
    drop(db);

    // ファイル URI を構築
    let Ok(uri) = Url::from_file_path(&file_path) else {
        tracing::error!("Failed to convert file path to URI: {}", file_path);
        return Ok(None);
    };

    // キーが存在しない場合、ファイル全体を置換
    #[allow(clippy::cast_possible_truncation)] // 翻訳JSONが42億行を超えることはない
    if let (Some(result), Some(original)) = (insert_result, original_text) {
        // 元のファイルの終端位置を計算
        let line_count = original.lines().count();
        let last_line = original.lines().last().unwrap_or("");

        let text_edit = TextEdit {
            range: Range {
                start: Position { line: 0, character: 0 },
                end: Position {
                    line: line_count.saturating_sub(1) as u32,
                    character: last_line.len() as u32,
                },
            },
            new_text: result.new_text,
        };

        let mut changes = std::collections::HashMap::new();
        changes.insert(uri.clone(), vec![text_edit]);

        let edit_result = backend
            .client
            .apply_edit(WorkspaceEdit { changes: Some(changes), ..Default::default() })
            .await;

        if let Err(e) = edit_result {
            tracing::error!("Failed to apply workspace edit: {}", e);
        }
    }

    // ファイルを開き、カーソルを移動
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
