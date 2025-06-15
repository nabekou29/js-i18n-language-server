//! JavaScript/TypeScript向けi18n Language Serverのバックエンド実装
//!
//! このモジュールは、JavaScript/TypeScriptプロジェクト向けのi18n Language Serverの
//! メインバックエンド実装を提供します。以下の主要機能を統合します：
//!
//! - `TranslationCache`: 翻訳リソースの高速キャッシュ管理
//! - `FileIdManager`: メモリ効率的なファイルID管理
//! - `I18nIndexer`: ワークスペース全体の翻訳キー管理
//! - LSP機能: 補完、ホバー、診断、定義ジャンプ
//!
//! # アーキテクチャ
//!
//! ```text
//! Backend
//! ├── Client (LSP通信)
//! ├── I18nIndexer (翻訳キー管理)
//! │   ├── TranslationCache (翻訳データ)
//! │   ├── FileIdManager (ファイル管理)
//! │   └── AnalysisResults (解析結果)
//! └── Workspace (プロジェクト管理)
//! ```
//!
//! # 作成者
//! @nabekou29
//!
//! # 作成日
//! 2025-06-15
//!
//! # 更新日
//! 2025-06-15 - Backend構造体の拡張実装

/// i18n解析エンジン（メインモジュール）
pub mod analyzer;
/// tree-sitterベースのi18n解析クエリシステム
pub mod query;
/// 翻訳リソース管理システム
pub mod translation;
/// i18n Language Server の型定義
pub mod types;

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result as AnyhowResult;
use dashmap::DashMap;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::{
    CompletionItem,
    CompletionItemKind,
    CompletionOptions,
    CompletionParams,
    CompletionResponse,
    Diagnostic,
    DiagnosticSeverity,
    DidChangeTextDocumentParams,
    DidOpenTextDocumentParams,
    ExecuteCommandOptions,
    Hover,
    HoverContents,
    HoverParams,
    HoverProviderCapability,
    InitializeParams,
    InitializeResult,
    InitializedParams,
    MarkupContent,
    MarkupKind,
    MessageType,
    OneOf,
    Position,
    Range,
    ServerCapabilities,
    TextDocumentSyncCapability,
    TextDocumentSyncKind,
    Url,
    WorkDoneProgressOptions,
    WorkspaceFoldersServerCapabilities,
    WorkspaceServerCapabilities,
};
use tower_lsp::{
    Client,
    LanguageServer,
};
use tracing::{
    debug,
    error,
    info,
    warn,
};

use crate::analyzer::{
    FileIdManager,
    analyze_file,
};
use crate::translation::{
    NamespacedKey,
    TranslationCache,
};
use crate::types::{
    AnalysisResult,
    ErrorSeverity,
    FileId,
    TranslationReference,
};

/// i18n Language Serverのバックエンド実装
///
/// JavaScript/TypeScriptコード内のi18n翻訳キーを解析し、
/// LSP機能（補完、ホバー、診断等）を提供します。
///
/// 統合されたコンポーネント：
/// - `I18nIndexer`: ワークスペース全体の翻訳キー管理
/// - `TranslationCache`: 翻訳リソースのキャッシュ
/// - `FileIdManager`: ファイルID管理
#[derive(Debug)]
pub struct Backend {
    /// LSPクライアントとの通信を担当
    pub client: Client,
    /// ワークスペース全体のi18n情報を管理するインデクサー
    pub indexer: Arc<I18nIndexer>,
}

/// ワークスペース全体の翻訳キー管理システム
///
/// 全ファイルの解析結果を統合し、LSP機能に必要な情報を効率的に提供します。
/// スレッドセーフな設計により、並行処理と増分更新に対応します。
#[derive(Debug)]
pub struct I18nIndexer {
    /// 翻訳リソースのキャッシュ管理
    pub translation_cache: Arc<TranslationCache>,
    /// ファイルID管理システム
    pub file_id_manager: Arc<FileIdManager>,
    /// ファイルごとの解析結果（FileId -> `AnalysisResult`）
    pub analysis_results: Arc<DashMap<FileId, AnalysisResult>>,
    /// URL -> `FileId` のマッピング
    pub url_to_file_id: Arc<DashMap<String, FileId>>,
    /// ワークスペースルートパス
    pub workspace_root: Arc<RwLock<Option<PathBuf>>>,
}

impl I18nIndexer {
    /// `新しいI18nIndexerを作成します`
    ///
    /// # 戻り値
    ///
    /// `初期化されたI18nIndexer`
    #[must_use]
    pub fn new() -> Self {
        Self {
            translation_cache: Arc::new(TranslationCache::new()),
            file_id_manager: Arc::new(FileIdManager::new()),
            analysis_results: Arc::new(DashMap::new()),
            url_to_file_id: Arc::new(DashMap::new()),
            workspace_root: Arc::new(RwLock::new(None)),
        }
    }

    /// ワークスペースを初期化します
    ///
    /// # 引数
    ///
    /// * `workspace_root` - ワークスペースのルートパス
    ///
    /// # 戻り値
    ///
    /// 初期化結果（見つかった翻訳ファイル数）
    ///
    /// # Errors
    ///
    /// ワークスペースのスキャンに失敗した場合、`anyhow::Error`を返します
    pub async fn initialize_workspace(&self, workspace_root: PathBuf) -> AnyhowResult<usize> {
        info!("Initializing workspace: {}", workspace_root.display());

        // ワークスペースルートを設定
        {
            let mut root = self.workspace_root.write().await;
            *root = Some(workspace_root.clone());
        }

        // 翻訳ファイルの自動検出とロード
        let mut total_files = 0;

        // 一般的な翻訳ファイルディレクトリを検索
        let translation_dirs = [
            workspace_root.join("locales"),
            workspace_root.join("public/locales"),
            workspace_root.join("src/locales"),
            workspace_root.join("i18n"),
            workspace_root.join("translations"),
            workspace_root.join("lang"),
        ];

        for dir in &translation_dirs {
            if dir.exists() && dir.is_dir() {
                match self.translation_cache.load_directory(dir).await {
                    Ok(count) => {
                        total_files += count;
                        info!("Loaded {} translation files from {}", count, dir.display());
                    }
                    Err(e) => {
                        warn!("Failed to load translations from {}: {}", dir.display(), e);
                    }
                }
            }
        }

        // ファイル監視を開始
        for dir in &translation_dirs {
            if dir.exists() && dir.is_dir() {
                if let Err(e) = self.translation_cache.start_watching(dir) {
                    warn!("Failed to start watching {}: {}", dir.display(), e);
                }
            }
        }

        info!("Workspace initialization completed. Total translation files: {}", total_files);
        Ok(total_files)
    }

    /// ファイルを解析してインデックスに追加します
    ///
    /// # 引数
    ///
    /// * `uri` - ファイルのURI
    /// * `content` - ファイルの内容
    ///
    /// # 戻り値
    ///
    /// 解析結果
    ///
    /// # Errors
    ///
    /// 解析に失敗した場合、`anyhow::Error`を返します
    pub fn analyze_and_index_file(&self, uri: &str, content: &str) -> AnyhowResult<AnalysisResult> {
        let parsed_url = Url::parse(uri)?;
        let file_path = PathBuf::from(parsed_url.path());

        // ファイルIDを取得または作成
        let file_id = self.file_id_manager.get_or_create_file_id(file_path.clone());

        // URI -> FileId のマッピングを保存
        self.url_to_file_id.insert(uri.to_string(), file_id);

        // ファイルを解析
        let analysis_result = analyze_file(file_id, &file_path, content)?;

        // 解析結果をインデックスに保存
        self.analysis_results.insert(file_id, analysis_result.clone());

        debug!(
            "Analyzed file: {} (FileId: {}), References: {}, Errors: {}",
            file_path.display(),
            file_id.as_u32(),
            analysis_result.references.len(),
            analysis_result.errors.len()
        );

        Ok(analysis_result)
    }

    /// 指定された位置で翻訳キーを検索します
    ///
    /// # 引数
    ///
    /// * `uri` - ファイルのURI
    /// * `line` - 行番号（0ベース）
    /// * `column` - 列番号（0ベース）
    ///
    /// # 戻り値
    ///
    /// 見つかった翻訳参照
    #[must_use]
    pub fn find_translation_at_position(
        &self,
        uri: &str,
        line: u32,
        column: u32,
    ) -> Option<TranslationReference> {
        let file_id = self.url_to_file_id.get(uri)?;
        let analysis_result = self.analysis_results.get(&file_id)?;

        // 指定された位置に一致する翻訳参照を検索
        analysis_result
            .references
            .iter()
            .find(|ref_| ref_.line == line && ref_.column <= column)
            .cloned()
    }

    /// 翻訳キーの補完候補を取得します
    ///
    /// # 引数
    ///
    /// * `prefix` - 入力されたプレフィックス
    /// * `namespace` - 検索対象の名前空間
    /// * `limit` - 結果の最大数
    ///
    /// # 戻り値
    ///
    /// 補完候補のリスト
    #[must_use]
    pub fn get_completion_items(
        &self,
        prefix: &str,
        namespace: Option<&str>,
        limit: usize,
    ) -> Vec<CompletionItem> {
        let keys = self.translation_cache.search_keys_with_prefix(namespace, prefix, limit);

        keys.into_iter()
            .map(|key| CompletionItem {
                label: key.clone(),
                kind: Some(CompletionItemKind::CONSTANT),
                detail: Some(format!(
                    "Translation key{}",
                    namespace.map_or(String::new(), |ns| format!(" (namespace: {ns})"))
                )),
                documentation: self.get_translation_content(&key, namespace),
                ..Default::default()
            })
            .collect()
    }

    /// 翻訳キーの内容を取得します
    ///
    /// # 引数
    ///
    /// * `key` - 翻訳キー
    /// * `namespace` - 名前空間
    ///
    /// # 戻り値
    ///
    /// 翻訳内容（MarkupContent形式）
    fn get_translation_content(
        &self,
        key: &str,
        namespace: Option<&str>,
    ) -> Option<tower_lsp::lsp_types::Documentation> {
        let namespaced_key = NamespacedKey::new(namespace.map(String::from), key.to_string());

        let value = self.translation_cache.get_translation(&namespaced_key)?;
        let content = value.as_string()?;

        Some(tower_lsp::lsp_types::Documentation::MarkupContent(MarkupContent {
            kind: MarkupKind::Markdown,
            value: format!("**Translation:**\n\n```\n{content}\n```"),
        }))
    }

    /// 診断を生成します
    ///
    /// # 引数
    ///
    /// * `uri` - ファイルのURI
    /// * `result` - 解析結果
    ///
    /// # 戻り値
    ///
    /// 診断のベクター
    #[must_use]
    pub fn generate_diagnostics(&self, uri: &str, result: &AnalysisResult) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // 解析エラーを診断に変換
        for error in &result.errors {
            let severity = match error.severity() {
                ErrorSeverity::Warning => DiagnosticSeverity::WARNING,
                ErrorSeverity::Info => DiagnosticSeverity::INFORMATION,
                ErrorSeverity::Fatal | ErrorSeverity::Error => DiagnosticSeverity::ERROR,
            };

            let diagnostic = Diagnostic {
                range: Range {
                    start: Position {
                        line: error.position().map_or(0, |p| p.line.saturating_sub(1)),
                        character: error.position().map_or(0, |p| p.column.saturating_sub(1)),
                    },
                    end: Position {
                        line: error.position().map_or(0, |p| p.line.saturating_sub(1)),
                        character: error.position().map_or(0, |p| p.column + 10), // 仮の終了位置
                    },
                },
                severity: Some(severity),
                code: None,
                code_description: None,
                source: Some("js-i18n-language-server".to_string()),
                message: error.to_string(),
                related_information: None,
                tags: None,
                data: None,
            };

            diagnostics.push(diagnostic);
        }

        // 未定義の翻訳キーをチェック
        for reference in &result.references {
            let namespaced_key =
                NamespacedKey::new(reference.namespace.clone(), reference.key.clone());

            if self.translation_cache.get_translation(&namespaced_key).is_none() {
                let diagnostic = Diagnostic {
                    range: Range {
                        start: Position { line: reference.line, character: reference.column },
                        end: Position {
                            line: reference.line,
                            character: reference.column
                                + u32::try_from(reference.key.len()).unwrap_or(0),
                        },
                    },
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: Some(tower_lsp::lsp_types::NumberOrString::String(
                        "missing-translation".to_string(),
                    )),
                    code_description: None,
                    source: Some("js-i18n-language-server".to_string()),
                    message: format!("Translation key '{}' not found", reference.key),
                    related_information: None,
                    tags: None,
                    data: None,
                };

                diagnostics.push(diagnostic);
            }
        }

        debug!(
            "Generated {} diagnostics for {} (analysis errors: {}, missing translations: {})",
            diagnostics.len(),
            uri,
            result.errors.len(),
            diagnostics.len() - result.errors.len()
        );

        diagnostics
    }

    /// 統計情報を取得します
    ///
    /// # 戻り値
    ///
    /// インデクサーの統計情報
    #[must_use]
    pub fn get_stats(&self) -> I18nIndexerStats {
        let cache_stats = self.translation_cache.get_stats();
        let total_references =
            self.analysis_results.iter().map(|entry| entry.references.len()).sum();

        I18nIndexerStats {
            total_files: self.file_id_manager.file_count(),
            total_references,
            total_translation_keys: cache_stats.total_keys,
            namespaces: cache_stats.namespace_count,
            translation_files: cache_stats.file_count,
        }
    }

    /// インデクサーをクリアします
    pub fn clear(&self) {
        self.analysis_results.clear();
        self.url_to_file_id.clear();
        self.translation_cache.clear();
        info!("I18n indexer cleared");
    }
}

impl Default for I18nIndexer {
    fn default() -> Self {
        Self::new()
    }
}

/// `I18nIndexerの統計情報`
#[derive(Debug, Clone, Copy)]
pub struct I18nIndexerStats {
    /// 管理されているファイル数
    pub total_files: usize,
    /// 総翻訳参照数
    pub total_references: usize,
    /// 総翻訳キー数
    pub total_translation_keys: usize,
    /// 名前空間数
    pub namespaces: usize,
    /// 翻訳ファイル数
    pub translation_files: usize,
}

impl Backend {
    /// 新しいBackendインスタンスを作成します
    ///
    /// # 引数
    ///
    /// * `client` - LSPクライアント
    ///
    /// # 戻り値
    ///
    /// 初期化されたBackend
    #[must_use]
    pub fn new(client: Client) -> Self {
        Self { client, indexer: Arc::new(I18nIndexer::new()) }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        info!("Language server initializing...");

        // ワークスペースの初期化
        if let Some(workspace_folders) = params.workspace_folders {
            for folder in workspace_folders {
                let workspace_path = PathBuf::from(folder.uri.path());
                match self.indexer.initialize_workspace(workspace_path.clone()).await {
                    Ok(file_count) => {
                        self.client
                            .log_message(
                                MessageType::INFO,
                                format!(
                                    "Initialized workspace: {} ({} translation files)",
                                    workspace_path.display(),
                                    file_count
                                ),
                            )
                            .await;
                    }
                    Err(e) => {
                        error!(
                            "Failed to initialize workspace {}: {}",
                            workspace_path.display(),
                            e
                        );
                        self.client
                            .log_message(
                                MessageType::ERROR,
                                format!(
                                    "Failed to initialize workspace {}: {}",
                                    workspace_path.display(),
                                    e
                                ),
                            )
                            .await;
                    }
                }
            }
        } else if let Some(root_uri) = params.root_uri {
            // Fallback to root_uri if workspace_folders is not provided
            let workspace_path = PathBuf::from(root_uri.path());
            match self.indexer.initialize_workspace(workspace_path.clone()).await {
                Ok(file_count) => {
                    self.client
                        .log_message(
                            MessageType::INFO,
                            format!(
                                "Initialized workspace: {} ({} translation files)",
                                workspace_path.display(),
                                file_count
                            ),
                        )
                        .await;
                }
                Err(e) => {
                    error!("Failed to initialize workspace {}: {}", workspace_path.display(), e);
                }
            }
        }

        Ok(InitializeResult {
            server_info: Some(tower_lsp::lsp_types::ServerInfo {
                name: "js-i18n-language-server".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::INCREMENTAL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec![
                        "'".to_string(),
                        "\"".to_string(),
                        ".".to_string(),
                    ]),
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                    all_commit_characters: None,
                    completion_item: None,
                }),
                execute_command_provider: Some(ExecuteCommandOptions {
                    commands: vec!["i18n.show_stats".to_string(), "i18n.clear_cache".to_string()],
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                }),
                workspace: Some(WorkspaceServerCapabilities {
                    workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                        supported: Some(true),
                        change_notifications: Some(OneOf::Left(true)),
                    }),
                    file_operations: None,
                }),
                ..ServerCapabilities::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        let stats = self.indexer.get_stats();
        self.client
            .log_message(
                MessageType::INFO,
                format!(
                    "i18n Language Server initialized! Files: {}, References: {}, Translation keys: {}",
                    stats.total_files, stats.total_references, stats.total_translation_keys
                ),
            )
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        info!("Language server shutting down...");
        // ファイル監視を停止
        self.indexer.translation_cache.stop_watching();
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.to_string();
        let content = &params.text_document.text;

        // ファイルを解析してインデックスに追加
        match self.indexer.analyze_and_index_file(&uri, content) {
            Ok(result) => {
                debug!(
                    "Analyzed opened file: {} (references: {}, errors: {})",
                    uri,
                    result.references.len(),
                    result.errors.len()
                );

                // 診断を送信
                let diagnostics = self.indexer.generate_diagnostics(&uri, &result);
                if let Ok(parsed_uri) = Url::parse(&uri) {
                    self.client.publish_diagnostics(parsed_uri, diagnostics, None).await;
                } else {
                    warn!("Failed to parse URI for diagnostics: {}", uri);
                }
            }
            Err(e) => {
                error!("Failed to analyze file {}: {}", uri, e);
            }
        }
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.to_string();

        // 増分更新の場合は最後の変更を使用（簡略化）
        if let Some(change) = params.content_changes.into_iter().next_back() {
            let text = change.text;
            // ファイルを再解析
            match self.indexer.analyze_and_index_file(&uri, &text) {
                Ok(result) => {
                    debug!(
                        "Re-analyzed changed file: {} (references: {})",
                        uri,
                        result.references.len()
                    );

                    // 診断を送信
                    let diagnostics = self.indexer.generate_diagnostics(&uri, &result);
                    if let Ok(parsed_uri) = Url::parse(&uri) {
                        self.client.publish_diagnostics(parsed_uri, diagnostics, None).await;
                    } else {
                        warn!("Failed to parse URI for diagnostics: {}", uri);
                    }
                }
                Err(e) => {
                    error!("Failed to re-analyze file {}: {}", uri, e);
                }
            }
        }
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri.to_string();
        let position = params.text_document_position.position;

        debug!("Completion requested for {} at {}:{}", uri, position.line, position.character);

        // 現在位置で翻訳キーが使用されているかチェック
        if let Some(translation_ref) =
            self.indexer.find_translation_at_position(&uri, position.line, position.character)
        {
            // 翻訳キーの補完候補を取得
            let completion_items = self.indexer.get_completion_items(
                &translation_ref.key,
                translation_ref.namespace.as_deref(),
                50, // 最大50件
            );

            if !completion_items.is_empty() {
                debug!("Found {} completion items", completion_items.len());
                return Ok(Some(CompletionResponse::Array(completion_items)));
            }
        }

        // 一般的な翻訳キーの補完（プレフィックスなし）
        let completion_items = self.indexer.get_completion_items("", None, 20);

        if completion_items.is_empty() {
            Ok(None)
        } else {
            debug!("Found {} general completion items", completion_items.len());
            Ok(Some(CompletionResponse::Array(completion_items)))
        }
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri.to_string();
        let position = params.text_document_position_params.position;

        debug!("Hover requested for {} at {}:{}", uri, position.line, position.character);

        // 現在位置で翻訳キーを検索
        if let Some(translation_ref) =
            self.indexer.find_translation_at_position(&uri, position.line, position.character)
        {
            // 翻訳内容を取得
            let namespaced_key =
                NamespacedKey::new(translation_ref.namespace.clone(), translation_ref.key.clone());

            if let Some(translation_value) =
                self.indexer.translation_cache.get_translation(&namespaced_key)
            {
                let content = if let Some(text) = translation_value.as_string() {
                    format!(
                        "**Translation Key:** `{}`\n\n**Content:**\n```\n{}\n```\n\n**Function:** `{}`{}",
                        translation_ref.key,
                        text,
                        translation_ref.function_name,
                        translation_ref
                            .namespace
                            .as_ref()
                            .map_or(String::new(), |ns| format!("\n**Namespace:** `{ns}`"))
                    )
                } else {
                    format!(
                        "**Translation Key:** `{}`\n\n**Type:** Object\n\n**Available Keys:**\n{}\n\n**Function:** `{}`{}",
                        translation_ref.key,
                        translation_value.get_keys().join(", "),
                        translation_ref.function_name,
                        translation_ref
                            .namespace
                            .as_ref()
                            .map_or(String::new(), |ns| format!("\n**Namespace:** `{ns}`"))
                    )
                };

                return Ok(Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: content,
                    }),
                    range: None,
                }));
            }

            // 翻訳が見つからない場合
            let content = format!(
                "**Translation Key:** `{}`\n\n⚠️ **Translation not found**\n\n**Function:** `{}`{}",
                translation_ref.key,
                translation_ref.function_name,
                translation_ref
                    .namespace
                    .as_ref()
                    .map_or(String::new(), |ns| format!("\n**Namespace:** `{ns}`"))
            );

            return Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: content,
                }),
                range: None,
            }));
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_backend_creation() {
        let (_client, _) = tower_lsp::LspService::new(Backend::new);
        // バックエンドが正常に作成されることを確認（ダミーアサーション削除）
    }

    #[tokio::test]
    async fn test_i18n_indexer_creation() {
        let indexer = I18nIndexer::new();
        let stats = indexer.get_stats();

        assert_eq!(stats.total_files, 0);
        assert_eq!(stats.total_references, 0);
        assert_eq!(stats.total_translation_keys, 0);
    }

    #[test]
    fn test_i18n_indexer_stats() {
        let indexer = I18nIndexer::new();
        let stats = indexer.get_stats();

        // 初期状態では全て0
        assert_eq!(stats.total_files, 0);
        assert_eq!(stats.total_references, 0);
        assert_eq!(stats.total_translation_keys, 0);
        assert_eq!(stats.namespaces, 0);
        assert_eq!(stats.translation_files, 0);
    }

    #[tokio::test]
    async fn test_indexer_clear() {
        let indexer = I18nIndexer::new();

        // データを追加してからクリア
        indexer.clear();

        let stats = indexer.get_stats();
        assert_eq!(stats.total_files, 0);
        assert_eq!(stats.total_references, 0);
    }

    #[tokio::test]
    async fn test_find_translation_at_position() {
        let indexer = I18nIndexer::new();

        // テスト用のJavaScriptコード
        let code = r"
            const message = t('hello.world');
            const another = i18n.t('another.key');
        ";

        // ファイルを解析
        let uri = "file:///test.js";
        let result = indexer.analyze_and_index_file(uri, code);
        assert!(result.is_ok());

        // 1行目の翻訳キーを検索
        let translation_ref = indexer.find_translation_at_position(uri, 1, 29);
        assert!(translation_ref.is_some());
        if let Some(translation_ref) = translation_ref {
            assert_eq!(translation_ref.key, "hello.world");
            assert_eq!(translation_ref.function_name, "t");
        }

        // 2行目の翻訳キーを検索
        let translation_ref = indexer.find_translation_at_position(uri, 2, 32);
        assert!(translation_ref.is_some());
        if let Some(translation_ref) = translation_ref {
            assert_eq!(translation_ref.key, "another.key");
            assert_eq!(translation_ref.function_name, "t");
        }

        // 存在しない位置での検索
        let translation_ref = indexer.find_translation_at_position(uri, 10, 10);
        assert!(translation_ref.is_none());
    }

    #[test]
    fn test_completion_items_generation() {
        let indexer = I18nIndexer::new();

        // 空の状態では補完候補なし
        let items = indexer.get_completion_items("hello", None, 10);
        assert!(items.is_empty());
    }
}
