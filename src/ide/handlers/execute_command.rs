//! Execute Command ハンドラー
//!
//! `workspace/executeCommand` リクエストを処理し、
//! カスタムコマンドを実行します。

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

/// `workspace/executeCommand` リクエストを処理
#[allow(clippy::single_match_else)] // 将来的にコマンドが増える可能性を考慮
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
        let result =
            crate::ide::code_actions::insert_key_to_json(&*db, translation, key, &key_separator);
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

/// `i18n.getDecorations` コマンドの引数
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetDecorationsArgs {
    /// ファイル URI
    uri: String,
    /// 表示する言語（省略時は最初に見つかった翻訳を使用）
    language: Option<String>,
    /// 最大表示文字数（省略時は設定のデフォルト値）
    max_length: Option<usize>,
}

/// `i18n.getDecorations` コマンドを実行
///
/// ドキュメント内の翻訳キーと翻訳値のリストを返す。
/// エディタ拡張がこの情報を使用して、キー文字列を翻訳値で置換表示する。
///
/// # Arguments
/// * `arguments[0]` - `GetDecorationsArgs` オブジェクト
///
/// # Returns
/// `TranslationDecoration` の配列（JSON）
async fn handle_get_decorations(
    backend: &Backend,
    arguments: Option<Vec<Value>>,
) -> Result<Option<Value>> {
    let args = arguments.unwrap_or_default();

    // 引数をパース
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

    // URI をパース
    let Ok(uri) = Url::parse(&parsed_args.uri) else {
        tracing::warn!("Invalid URI: {}", parsed_args.uri);
        return Ok(Some(serde_json::json!([])));
    };

    // ファイルパスを取得
    let Some(file_path) = Backend::uri_to_path(&uri) else {
        tracing::warn!("Failed to convert URI to path: {}", parsed_args.uri);
        return Ok(Some(serde_json::json!([])));
    };

    // 設定からデフォルト値を取得
    let config = backend.config_manager.lock().await;
    let settings = config.get_settings();
    let max_length = parsed_args.max_length.unwrap_or(settings.virtual_text.max_length);
    let primary_languages = settings.primary_languages.clone();
    let key_separator = settings.key_separator.clone();
    drop(config);

    // SourceFile を取得
    let db = backend.state.db.lock().await;
    let source_files = backend.state.source_files.lock().await;

    let Some(source_file) = source_files.get(&file_path).copied() else {
        tracing::debug!("Source file not found: {:?}", file_path);
        return Ok(Some(serde_json::json!([])));
    };

    // 翻訳データを取得
    let translations = backend.state.translations.lock().await;

    // 有効な言語を決定（リクエスト指定 → currentLanguage → primaryLanguages → 最初の言語）
    let language = parsed_args.language.clone().or_else(|| {
        let available_languages: Vec<String> = translations
            .iter()
            .map(|t| t.language(&*db))
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        let current_language = backend.state.current_language.blocking_lock().clone();
        crate::ide::backend::resolve_effective_language(
            current_language.as_deref(),
            primary_languages.as_deref(),
            &available_languages,
        )
    });

    // 翻訳装飾情報を生成
    let decorations = crate::ide::virtual_text::get_translation_decorations(
        &*db,
        source_file,
        &translations,
        language.as_deref(),
        max_length,
        &key_separator,
    );

    // ロックを解放
    drop(translations);
    drop(source_files);
    drop(db);

    // JSON に変換して返す
    match serde_json::to_value(&decorations) {
        Ok(value) => Ok(Some(value)),
        Err(e) => {
            tracing::error!("Failed to serialize decorations: {}", e);
            Ok(Some(serde_json::json!([])))
        }
    }
}

/// `i18n.setCurrentLanguage` コマンドの引数
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetCurrentLanguageArgs {
    /// 設定する言語コード（null でリセット）
    language: Option<String>,
}

/// `i18n.setCurrentLanguage` コマンドを実行
///
/// 現在の表示言語を変更する。Virtual Text、補完、Code Actions で使用される。
///
/// # Arguments
/// * `arguments[0]` - `SetCurrentLanguageArgs` オブジェクト
///
/// # Returns
/// 成功時は `null`
async fn handle_set_current_language(
    backend: &Backend,
    arguments: Option<Vec<Value>>,
) -> Result<Option<Value>> {
    let args = arguments.unwrap_or_default();

    // 引数をパース
    let parsed_args: SetCurrentLanguageArgs = if let Some(first_arg) = args.first().cloned() {
        match serde_json::from_value(first_arg) {
            Ok(args) => args,
            Err(e) => {
                tracing::warn!("Invalid arguments for i18n.setCurrentLanguage: {}", e);
                return Ok(None);
            }
        }
    } else {
        // 引数なしの場合はリセット
        SetCurrentLanguageArgs { language: None }
    };

    tracing::debug!(
        language = ?parsed_args.language,
        "Executing i18n.setCurrentLanguage"
    );

    // current_language を更新
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
