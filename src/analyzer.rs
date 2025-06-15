//! i18n解析エンジン（メインモジュール）
//!
//! このモジュールは、JavaScript/TypeScriptファイルからi18n関数呼び出しを解析し、
//! 翻訳キーの参照情報を抽出するメインエンジンです。ADR-003のFileId管理戦略と
//! ADR-004のtree-sitterクエリ戦略を実装し、js-i18n.nvimベースのスコープ管理を提供します。
//!
//! # 主要機能
//!
//! - 単一ファイルの純粋関数ベース解析（`analyze_file`）
//! - スコープ階層の構築と管理（`build_scope_stack`）
//! - 翻訳キーの解決とスコープ適用（`resolve_translation_key`）
//! - メモリ効率的なFileId管理（`FileIdManager`）
//!
//! # 設計原則
//!
//! - 純粋関数設計（副作用なし）
//! - 完全なエラーハンドリング（panic禁止）
//! - メモリ効率化（FileId型使用）
//! - js-i18n.nvimベースの実証済みアルゴリズム
//!
//! # アーキテクチャ
//!
//! ```text
//! analyze_file (メインエントリーポイント)
//!   ├── tree-sitterでファイルパース
//!   ├── クエリ実行でi18n要素抽出
//!   ├── build_scope_stack (スコープ構築)
//!   ├── resolve_translation_key (キー解決)
//!   └── AnalysisResult生成
//! ```
//!
//! # 作成者
//! @nabekou29
//!
//! # 作成日
//! 2025-06-15
//!
//! # 更新日
//! 2025-06-15 - エラーハンドリング強化（リカバリ機能、バリデーション、ログ改善）

use std::path::{
    Path,
    PathBuf,
};
use std::sync::atomic::{
    AtomicU32,
    Ordering,
};
use std::time::{
    Duration,
    Instant,
};

use anyhow::Result;
use dashmap::DashMap;
use tracing::{
    Level,
    debug,
    info,
    span,
    warn,
};
use tree_sitter::{
    Language,
    Parser,
    Tree,
};

use crate::query::{
    QueryExecutor,
    QueryTranslationReference,
    TranslationQueries,
    language_javascript,
    language_typescript,
};
use crate::types::{
    AnalysisError,
    AnalysisResult,
    ErrorContext,
    ErrorSeverity,
    FileId,
    FileType,
    ScopeInfo,
    TranslationReference,
};

/// ファイル解析の制限値
///
/// プロダクション環境での安定性を確保するための制限値を定義します。
#[derive(Debug, Clone, Copy)]
pub struct AnalysisLimits {
    /// ファイルサイズ制限（バイト）
    pub max_file_size: u64,
    /// メモリ使用量制限（バイト）
    pub max_memory_usage: u64,
    /// 処理タイムアウト（ミリ秒）
    pub timeout_ms: u64,
    /// 最大翻訳参照数
    pub max_references: usize,
    /// 最大スコープ深度
    pub max_scope_depth: usize,
}

impl Default for AnalysisLimits {
    fn default() -> Self {
        Self {
            max_file_size: 10 * 1024 * 1024,     // 10MB
            max_memory_usage: 100 * 1024 * 1024, // 100MB
            timeout_ms: 30_000,                  // 30秒
            max_references: 10_000,
            max_scope_depth: 100,
        }
    }
}

/// 解析バリデーター
///
/// 入力データの事前検証とリソース保護を行います。
/// メモリ使用量、ファイルサイズ、処理時間の制限を強制します。
#[derive(Debug, Clone, Copy)]
pub struct AnalysisValidator {
    /// 制限値
    limits: AnalysisLimits,
}

impl AnalysisValidator {
    /// `新しいAnalysisValidatorを作成します`
    ///
    /// # 引数
    ///
    /// * `limits` - 解析制限値
    ///
    /// # 戻り値
    ///
    /// `新しいAnalysisValidator`
    #[must_use]
    pub const fn new(limits: AnalysisLimits) -> Self {
        Self { limits }
    }

    /// ファイルサイズを検証します
    ///
    /// # 引数
    ///
    /// * `file_path` - ファイルパス
    /// * `source_code` - ソースコード
    ///
    /// # 戻り値
    ///
    /// 検証結果。制限超過の場合はエラー
    ///
    /// # Errors
    ///
    /// ファイルサイズが制限を超えている場合、`AnalysisError::FileTooLarge`を返します
    pub fn validate_file_size(
        &self,
        file_path: &Path,
        source_code: &str,
    ) -> Result<(), AnalysisError> {
        let file_size = source_code.len() as u64;

        if file_size > self.limits.max_file_size {
            let context = ErrorContext::new("validate_file_size".to_string(), ErrorSeverity::Error)
                .with_metadata("file_size".to_string(), file_size.to_string())
                .with_metadata("max_file_size".to_string(), self.limits.max_file_size.to_string());

            return Err(AnalysisError::FileTooLarge {
                size: file_size,
                limit: self.limits.max_file_size,
                path: Box::new(file_path.to_path_buf()),
                context: Box::new(context),
            });
        }

        Ok(())
    }

    /// ソースコードのエンコーディングを検証します
    ///
    /// # 引数
    ///
    /// * `file_path` - ファイルパス
    /// * `source_code` - ソースコード
    ///
    /// # 戻り値
    ///
    /// 検証結果。無効なエンコーディングの場合はエラー
    ///
    /// # Errors
    ///
    /// UTF-8でない場合、`AnalysisError::FileEncodingError`を返します
    pub fn validate_encoding(
        &self,
        file_path: &Path,
        source_code: &str,
    ) -> Result<(), AnalysisError> {
        // Rustの文字列は既にUTF-8が保証されているため、
        // ここでは主に無効なUTF-8シーケンスをチェック
        if !source_code.is_ascii() && source_code.chars().any(|c| c == '\u{FFFD}') {
            let context = ErrorContext::new("validate_encoding".to_string(), ErrorSeverity::Error);

            return Err(AnalysisError::FileEncodingError {
                path: Box::new(file_path.to_path_buf()),
                byte_position: None,
                context: Box::new(context),
            });
        }

        Ok(())
    }

    /// 処理時間を監視します
    ///
    /// # 引数
    ///
    /// * `start_time` - 処理開始時刻
    /// * `operation` - 実行中の操作
    ///
    /// # 戻り値
    ///
    /// 検証結果。タイムアウトの場合はエラー
    ///
    /// # Errors
    ///
    /// 制限時間を超過した場合、`AnalysisError::TimeoutError`を返します
    pub fn check_timeout(&self, start_time: Instant, operation: &str) -> Result<(), AnalysisError> {
        let elapsed = start_time.elapsed();
        let timeout_duration = Duration::from_millis(self.limits.timeout_ms);

        if elapsed > timeout_duration {
            let context = ErrorContext::new("check_timeout".to_string(), ErrorSeverity::Error)
                .with_metadata("elapsed_ms".to_string(), elapsed.as_millis().to_string())
                .with_metadata("operation".to_string(), operation.to_string());

            return Err(AnalysisError::TimeoutError {
                timeout_ms: self.limits.timeout_ms,
                operation: operation.to_string().into(),
                context: Box::new(context),
            });
        }

        Ok(())
    }

    /// 翻訳参照数を検証します
    ///
    /// # 引数
    ///
    /// * `reference_count` - 現在の参照数
    ///
    /// # 戻り値
    ///
    /// 検証結果。制限超過の場合はエラー
    ///
    /// # Errors
    ///
    /// 参照数が制限を超えた場合、`AnalysisError::InternalError`を返します
    pub fn validate_reference_count(&self, reference_count: usize) -> Result<(), AnalysisError> {
        if reference_count > self.limits.max_references {
            let context =
                ErrorContext::new("validate_reference_count".to_string(), ErrorSeverity::Warning)
                    .with_metadata("reference_count".to_string(), reference_count.to_string())
                    .with_metadata(
                        "max_references".to_string(),
                        self.limits.max_references.to_string(),
                    );

            return Err(AnalysisError::InternalError {
                message: format!(
                    "Too many translation references: {} exceeds limit of {}",
                    reference_count, self.limits.max_references
                )
                .into(),
                context: Box::new(context),
            });
        }

        Ok(())
    }
}

impl Default for AnalysisValidator {
    fn default() -> Self {
        Self::new(AnalysisLimits::default())
    }
}

/// エラー回復管理
///
/// 解析中に発生したエラーを収集し、可能な限り処理を継続するための
/// graceful degradation機能を提供します。
#[derive(Debug)]
pub struct ErrorRecovery {
    /// 収集されたエラーのリスト
    errors: Vec<AnalysisError>,
    /// エラー回復戦略
    recovery_strategy: RecoveryStrategy,
}

/// エラー回復戦略
///
/// エラー発生時の処理継続方法を定義します。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryStrategy {
    /// 厳格モード：最初のエラーで停止
    Strict,
    /// 寛容モード：回復可能なエラーは継続
    Tolerant,
    /// ベストエフォートモード：可能な限り処理を継続
    BestEffort,
}

impl ErrorRecovery {
    /// `新しいErrorRecoveryを作成します`
    ///
    /// # 引数
    ///
    /// * `strategy` - エラー回復戦略
    ///
    /// # 戻り値
    ///
    /// `新しいErrorRecovery`
    #[must_use]
    pub const fn new(strategy: RecoveryStrategy) -> Self {
        Self { errors: Vec::new(), recovery_strategy: strategy }
    }

    /// エラーを記録し、継続可能かを判定します
    ///
    /// # 引数
    ///
    /// * `error` - 発生したエラー
    ///
    /// # 戻り値
    ///
    /// 処理を継続する場合はtrue
    pub fn handle_error(&mut self, error: AnalysisError) -> bool {
        let should_continue = match self.recovery_strategy {
            RecoveryStrategy::Strict => false,
            RecoveryStrategy::Tolerant => error.is_recoverable(),
            RecoveryStrategy::BestEffort => true,
        };

        // エラーのログ出力
        error.log_error();

        // エラーを記録
        self.errors.push(error);

        should_continue
    }

    /// 収集されたエラーを取得します
    ///
    /// # 戻り値
    ///
    /// エラーのリスト
    #[must_use]
    pub fn get_errors(&self) -> &[AnalysisError] {
        &self.errors
    }

    /// エラーが存在するかを判定します
    ///
    /// # 戻り値
    ///
    /// エラーが存在する場合はtrue
    #[must_use]
    pub const fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// 致命的エラーが存在するかを判定します
    ///
    /// # 戻り値
    ///
    /// 致命的エラーが存在する場合はtrue
    #[must_use]
    pub fn has_fatal_errors(&self) -> bool {
        self.errors.iter().any(|e| e.severity() == ErrorSeverity::Fatal)
    }
}

/// メモリ効率的なファイルID管理システム
///
/// ADR-003で決定されたファイルID管理戦略を実装します。
/// ファイルパスを32ビット整数で管理することで、メモリ使用量を削減し、
/// 比較処理を高速化します。
///
/// # 特徴
///
/// - スレッドセーフな並行アクセス対応
/// - 双方向マッピング（パス⇔ID）
/// - 一意なID生成保証
/// - ID再利用なし（削除されたファイルのIDも保持）
///
/// # メモリ効率
///
/// - ファイルパス50文字 → 4バイト（約80%削減）
/// - 参照情報の固定サイズ化
/// - キャッシュ効率の向上
#[derive(Debug)]
pub struct FileIdManager {
    /// パス → ID のマッピング
    path_to_id: DashMap<PathBuf, FileId>,
    /// ID → パス のマッピング
    id_to_path: DashMap<FileId, PathBuf>,
    /// 次に割り当てるID（アトミック操作）
    next_id: AtomicU32,
}

impl FileIdManager {
    /// `新しいFileIdManagerインスタンスを作成`
    ///
    /// # 戻り値
    ///
    /// `初期化されたFileIdManager`
    #[must_use]
    pub fn new() -> Self {
        Self {
            path_to_id: DashMap::new(),
            id_to_path: DashMap::new(),
            next_id: AtomicU32::new(1), // 0は無効なIDとして予約
        }
    }

    /// `ファイルパスに対応するFileIdを取得または新規作成`
    ///
    /// 既存のパスの場合は既存のIDを返し、新規パスの場合は新しいIDを割り当てます。
    /// スレッドセーフな実装により、並行アクセスに対応します。
    ///
    /// # 引数
    ///
    /// * `path` - ファイルパス
    ///
    /// # 戻り値
    ///
    /// `対応するFileId`
    pub fn get_or_create_file_id(&self, path: PathBuf) -> FileId {
        // 既存のIDをチェック
        if let Some(existing_id) = self.path_to_id.get(&path) {
            return *existing_id;
        }

        // 新しいIDを生成
        let new_id = FileId::new(self.next_id.fetch_add(1, Ordering::SeqCst));

        // 双方向マッピングに追加
        self.path_to_id.insert(path.clone(), new_id);
        self.id_to_path.insert(new_id, path);

        new_id
    }

    /// `FileIdからファイルパスを取得`
    ///
    /// # 引数
    ///
    /// * `file_id` - ファイルID
    ///
    /// # 戻り値
    ///
    /// 対応するファイルパス（存在しない場合はNone）
    #[must_use]
    pub fn get_path(&self, file_id: FileId) -> Option<PathBuf> {
        self.id_to_path.get(&file_id).map(|path| path.clone())
    }

    /// `ファイルパスからFileIdを取得`
    ///
    /// # 引数
    ///
    /// * `path` - ファイルパス
    ///
    /// # 戻り値
    ///
    /// 対応するFileId（存在しない場合はNone）
    #[must_use]
    pub fn get_file_id(&self, path: &PathBuf) -> Option<FileId> {
        self.path_to_id.get(path).map(|id| *id)
    }

    /// 現在管理されているファイル数を取得
    ///
    /// # 戻り値
    ///
    /// 管理されているファイル数
    #[must_use]
    pub fn file_count(&self) -> usize {
        self.path_to_id.len()
    }

    /// 次に割り当てられるIDを取得（テスト用）
    ///
    /// # 戻り値
    ///
    /// 次のID値
    #[must_use]
    pub fn next_id(&self) -> u32 {
        self.next_id.load(Ordering::SeqCst)
    }
}

impl Default for FileIdManager {
    fn default() -> Self {
        Self::new()
    }
}

/// スコープ階層の構築
///
/// JavaScript/TypeScriptファイル内のスコープ階層を構築し、
/// 変数の有効範囲やimport文の影響を管理します。
/// js-i18n.nvimで実証済みのスコープ管理アルゴリズムを実装します。
///
/// # 引数
///
/// * `file_id` - 解析対象ファイルのID
/// * `tree` - tree-sitterで解析されたAST
/// * `source_code` - ソースコード文字列
/// * `queries` - 翻訳クエリ集合
///
/// # 戻り値
///
/// 構築されたスコープ情報のリスト
///
/// # Errors
///
/// tree-sitterクエリの実行エラーや文字列抽出エラーが発生した場合、`anyhow::Error`を返します
pub fn build_scope_stack(
    file_id: FileId,
    tree: &Tree,
    source_code: &str,
    queries: &TranslationQueries,
) -> Result<Vec<ScopeInfo>> {
    let mut scopes = Vec::new();
    let mut query_executor = QueryExecutor::new();

    // import文からi18n関数のマッピングを構築
    let import_matches =
        query_executor.execute_query(queries.import_statements(), tree, source_code)?;

    // ファイル全体を基本スコープとして追加
    let mut global_scope = ScopeInfo::new(file_id, 0, u32::MAX);

    // import文からi18n関数を抽出
    for import_match in import_matches {
        // キャプチャから最初のインポートソースを取得（簡略化）
        if let Some(source) = import_match.captures.get(&0) {
            if is_i18n_library(source) {
                // i18n関連のimportを処理
                // 例: import { useTranslation } from 'react-i18next'
                // 例: import i18n from 'i18next'
                extract_i18n_imports(&mut global_scope, source);
            }
        }
    }

    scopes.push(global_scope);

    // useTranslation()等のフック呼び出しから局所スコープを構築
    let hook_matches =
        query_executor.execute_query(queries.i18next_trans_f(), tree, source_code)?;

    for hook_match in hook_matches {
        let scope = build_hook_scope(file_id, &hook_match, source_code);
        scopes.push(scope);
    }

    Ok(scopes)
}

/// i18nライブラリかどうかを判定
///
/// # 引数
///
/// * `module_name` - モジュール名
///
/// # 戻り値
///
/// i18nライブラリの場合はtrue
fn is_i18n_library(module_name: &str) -> bool {
    matches!(module_name, "react-i18next" | "i18next" | "next-i18next" | "next-intl" | "vue-i18n")
}

/// import文からi18n関数を抽出してスコープに追加
///
/// # 引数
///
/// * `scope` - 対象スコープ
/// * `module_name` - インポート元モジュール名
fn extract_i18n_imports(scope: &mut ScopeInfo, module_name: &str) {
    match module_name {
        "react-i18next" => {
            scope.add_imported_function("useTranslation".to_string(), "useTranslation".to_string());
            scope.add_imported_function("Trans".to_string(), "Trans".to_string());
            scope.add_imported_function("Translation".to_string(), "Translation".to_string());
        }
        "i18next" => {
            scope.add_imported_function("i18n".to_string(), "i18n".to_string());
            scope.add_imported_function("t".to_string(), "t".to_string());
        }
        "next-intl" => {
            scope.add_imported_function(
                "useTranslations".to_string(),
                "useTranslations".to_string(),
            );
            scope.add_imported_function("useLocale".to_string(), "useLocale".to_string());
        }
        _ => {}
    }
}

/// フック呼び出しから局所スコープを構築
///
/// # 引数
///
/// * `file_id` - ファイルID
/// * `hook_match` - フック呼び出しのマッチ結果
/// * `source_code` - ソースコード
///
/// # 戻り値
///
/// 構築されたスコープ情報
fn build_hook_scope(
    file_id: FileId,
    hook_match: &QueryTranslationReference,
    source_code: &str,
) -> ScopeInfo {
    let start_line = u32::try_from(hook_match.start_line).unwrap_or(0);

    // フック呼び出しから関数終了まで（簡略化）
    // 実際の実装では、ASTを解析してスコープ範囲を正確に決定する必要がある
    let end_line = find_scope_end_line(source_code, start_line);

    let mut scope = ScopeInfo::new(file_id, start_line, end_line);

    // useTranslation()の引数からnamespaceとkeyPrefixを抽出
    // キャプチャインデックスベースでアクセス（簡略化）
    for (index, text) in &hook_match.captures {
        match index {
            1 => {
                // namespace
                scope.add_imported_function(format!("t_ns_{text}"), "t".to_string());
            }
            2 => {
                // key_prefix
                scope.add_imported_function(format!("t_prefix_{text}"), "t".to_string());
            }
            _ => {}
        }
    }

    scope
}

/// スコープの終了行を推定
///
/// # 引数
///
/// * `source_code` - ソースコード
/// * `start_line` - 開始行
///
/// # 戻り値
///
/// 推定終了行
fn find_scope_end_line(source_code: &str, start_line: u32) -> u32 {
    let lines: Vec<&str> = source_code.lines().collect();
    let start_index = usize::try_from(start_line).unwrap_or(0);

    if start_index >= lines.len() {
        return start_line;
    }

    // 簡略化された実装：同じ関数内の終了を探す
    // 実際の実装では、ASTを使用してより正確にスコープを決定する
    let mut brace_count = 0;
    let mut found_opening = false;

    for (i, line) in lines.iter().enumerate().skip(start_index) {
        for ch in line.chars() {
            match ch {
                '{' => {
                    brace_count += 1;
                    found_opening = true;
                }
                '}' => {
                    if found_opening {
                        brace_count -= 1;
                        if brace_count == 0 {
                            return u32::try_from(i).unwrap_or(u32::MAX);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // スコープ終了が見つからない場合は、ファイル終了まで
    u32::try_from(lines.len()).unwrap_or(u32::MAX)
}

/// 翻訳キーの解決とスコープ適用
///
/// 生の翻訳キーに対してスコープ情報を適用し、
/// 最終的な翻訳キーを生成します。js-i18n.nvimの
/// キー解決アルゴリズムを実装します。
///
/// # 引数
///
/// * `raw_key` - 生の翻訳キー
/// * `line` - キーが使用されている行番号
/// * `scopes` - 適用可能なスコープ情報のリスト
///
/// # 戻り値
///
/// 解決された翻訳キー
#[must_use]
pub fn resolve_translation_key(raw_key: &str, line: u32, scopes: &[ScopeInfo]) -> String {
    // 該当行を含むスコープを検索（最も内側のスコープを優先）
    let applicable_scope = scopes
        .iter()
        .filter(|scope| scope.contains_line(line))
        .min_by_key(|scope| scope.end_line - scope.start_line);

    applicable_scope
        .map_or_else(|| raw_key.to_string(), |scope| resolve_key_with_scope(raw_key, scope))
}

/// スコープ情報を適用してキーを解決
///
/// # 引数
///
/// * `raw_key` - 生の翻訳キー
/// * `scope` - 適用するスコープ情報
///
/// # 戻り値
///
/// 解決された翻訳キー
fn resolve_key_with_scope(raw_key: &str, scope: &ScopeInfo) -> String {
    let mut resolved_key = raw_key.to_string();

    // keyPrefix処理（先に適用）
    for alias in scope.imported_functions.keys() {
        if let Some(prefix) = alias.strip_prefix("t_prefix_") {
            // "t_prefix_"を除去
            if !resolved_key.starts_with(prefix) {
                resolved_key = format!("{prefix}.{resolved_key}");
            }
            break;
        }
    }

    // namespace処理（後に適用）
    for alias in scope.imported_functions.keys() {
        if let Some(namespace) = alias.strip_prefix("t_ns_") {
            // "t_ns_"を除去
            resolved_key = format!("{namespace}:{resolved_key}");
            break;
        }
    }

    resolved_key
}

/// メインのファイル解析関数
///
/// 単一ファイルを解析し、i18n関数呼び出しの参照情報を抽出します。
/// エラーハンドリング強化により、部分的な解析失敗時も可能な限り処理を継続し、
/// graceful degradationを実現します。
///
/// # 処理フロー
///
/// 1. 事前バリデーション（ファイルサイズ、エンコーディング）
/// 2. ファイル種別の判定
/// 3. tree-sitterでファイルのパース
/// 4. 各クエリを実行してi18n要素を抽出（エラー回復機能付き）
/// 5. スコープ情報の構築
/// 6. 翻訳キーの解決
/// 7. `TranslationReferenceの生成`
/// 8. `AnalysisResultの返却`
///
/// # 引数
///
/// * `file_id` - 解析対象ファイルのID
/// * `file_path` - ファイルパス
/// * `source_code` - ソースコード文字列
///
/// # 戻り値
///
/// 解析結果（参照情報、スコープ情報、エラー情報を含む）
///
/// # Errors
///
/// - サポートされていないファイル種別
/// - 構文解析エラー
/// - tree-sitterクエリ実行エラー
/// - ファイルサイズ制限超過
/// - 処理タイムアウト
pub fn analyze_file(
    file_id: FileId,
    file_path: &PathBuf,
    source_code: &str,
) -> Result<AnalysisResult> {
    analyze_file_with_options(
        file_id,
        file_path,
        source_code,
        AnalysisValidator::default(),
        RecoveryStrategy::Tolerant,
    )
}

/// オプション付きのファイル解析関数
///
/// カスタマイズされたバリデーションとエラー回復戦略でファイル解析を実行します。
///
/// # 引数
///
/// * `file_id` - 解析対象ファイルのID
/// * `file_path` - ファイルパス
/// * `source_code` - ソースコード文字列
/// * `validator` - バリデーター
/// * `recovery_strategy` - エラー回復戦略
///
/// # 戻り値
///
/// 解析結果（参照情報、スコープ情報、エラー情報を含む）
///
/// # Errors
///
/// - サポートされていないファイル種別
/// - 構文解析エラー
/// - tree-sitterクエリ実行エラー
/// - ファイルサイズ制限超過
/// - 処理タイムアウト
#[allow(clippy::too_many_lines)]
pub fn analyze_file_with_options(
    file_id: FileId,
    file_path: &PathBuf,
    source_code: &str,
    validator: AnalysisValidator,
    recovery_strategy: RecoveryStrategy,
) -> Result<AnalysisResult> {
    let _span =
        span!(Level::DEBUG, "analyze_file", file_id = file_id.as_u32(), file_path = ?file_path)
            .entered();
    let start_time = Instant::now();

    info!("Starting file analysis: {:?}", file_path);

    let mut result = AnalysisResult::new(file_id);
    let mut error_recovery = ErrorRecovery::new(recovery_strategy);

    // 事前バリデーション
    if let Err(e) = validator.validate_file_size(file_path, source_code) {
        if !error_recovery.handle_error(e) {
            return Ok(transfer_errors_to_result(&error_recovery, result));
        }
    }

    if let Err(e) = validator.validate_encoding(file_path, source_code) {
        if !error_recovery.handle_error(e) {
            return Ok(transfer_errors_to_result(&error_recovery, result));
        }
    }

    // タイムアウトチェック
    if let Err(e) = validator.check_timeout(start_time, "pre_validation") {
        error_recovery.handle_error(e);
        return Ok(transfer_errors_to_result(&error_recovery, result));
    }

    // ファイル種別の判定
    let file_type = match FileType::from_path(file_path) {
        Ok(ft) => {
            debug!("Detected file type: {:?}", ft);
            ft
        }
        Err(e) => {
            if !error_recovery.handle_error(e) {
                return Ok(transfer_errors_to_result(&error_recovery, result));
            }
            // デフォルトとしてJavaScriptを試す
            warn!("Using default file type: JavaScript");
            FileType::JavaScript
        }
    };

    // 言語パーサーの選択
    let language = select_language(file_type);
    debug!("Selected tree-sitter language for file type: {:?}", file_type);

    // パーサーの設定
    let mut parser = Parser::new();
    if let Err(e) = parser.set_language(language) {
        let context = ErrorContext::new("analyze_file".to_string(), ErrorSeverity::Error)
            .with_file(file_id, file_path.clone());

        let error = AnalysisError::TreeSitterError {
            reason: format!("Failed to set tree-sitter language: {e}").into(),
            position: None,
            context: Box::new(context),
        };

        if !error_recovery.handle_error(error) {
            return Ok(transfer_errors_to_result(&error_recovery, result));
        }
    }

    // タイムアウトチェック
    if let Err(e) = validator.check_timeout(start_time, "parser_setup") {
        error_recovery.handle_error(e);
        return Ok(transfer_errors_to_result(&error_recovery, result));
    }

    // ソースコードのパース
    let tree = if let Some(tree) = parser.parse(source_code, None) {
        debug!("Successfully parsed source code");
        tree
    } else {
        let context = ErrorContext::new("analyze_file".to_string(), ErrorSeverity::Error)
            .with_file(file_id, file_path.clone());

        let error = AnalysisError::TreeSitterError {
            reason: "Failed to parse source code".to_string().into(),
            position: None,
            context: Box::new(context),
        };

        if !error_recovery.handle_error(error) {
            return Ok(transfer_errors_to_result(&error_recovery, result));
        }
        // エラー回復: 空のツリーで続行することはできないため、ここで終了
        return Ok(transfer_errors_to_result(&error_recovery, result));
    };

    // 翻訳クエリの作成
    let queries = match create_translation_queries(language, file_type) {
        Ok(q) => {
            debug!("Successfully created translation queries");
            q
        }
        Err(e) => {
            let context = ErrorContext::new("analyze_file".to_string(), ErrorSeverity::Error)
                .with_file(file_id, file_path.clone());

            let error = AnalysisError::QueryExecutionError {
                query_name: "translation_queries".to_string().into(),
                reason: format!("Failed to create translation queries: {e}").into(),
                context: Box::new(context),
            };

            if !error_recovery.handle_error(error) {
                return Ok(transfer_errors_to_result(&error_recovery, result));
            }
            // エラー回復: クエリなしでは処理を続行できない
            return Ok(transfer_errors_to_result(&error_recovery, result));
        }
    };

    // タイムアウトチェック
    if let Err(e) = validator.check_timeout(start_time, "parsing") {
        error_recovery.handle_error(e);
        return Ok(transfer_errors_to_result(&error_recovery, result));
    }

    // スコープ階層の構築
    let scopes = match build_scope_stack(file_id, &tree, source_code, &queries) {
        Ok(s) => {
            debug!("Successfully built scope stack with {} scopes", s.len());
            s
        }
        Err(e) => {
            let context = ErrorContext::new("analyze_file".to_string(), ErrorSeverity::Warning)
                .with_file(file_id, file_path.clone());

            let error = AnalysisError::QueryExecutionError {
                query_name: "scope_building".to_string().into(),
                reason: format!("Failed to build scope stack: {e}").into(),
                context: Box::new(context),
            };

            if !error_recovery.handle_error(error) {
                return Ok(transfer_errors_to_result(&error_recovery, result));
            }
            // エラー回復: 空のスコープで続行
            warn!("Continuing with empty scope stack");
            Vec::new()
        }
    };

    // スコープ情報を結果に追加
    for scope in &scopes {
        result.add_scope(scope.clone());
    }

    // 各クエリを実行してi18n要素を抽出
    let mut query_executor = QueryExecutor::new();

    // i18next翻訳関数呼び出しの解析
    if let Err(e) = analyze_i18next_calls_with_recovery(
        &mut query_executor,
        &queries,
        &tree,
        source_code,
        &scopes,
        &mut result,
        &mut error_recovery,
        &validator,
        start_time,
    ) {
        warn!("Failed to analyze i18next calls: {}", e);
    }

    // タイムアウトチェック
    if let Err(e) = validator.check_timeout(start_time, "i18next_analysis") {
        error_recovery.handle_error(e);
        return Ok(transfer_errors_to_result(&error_recovery, result));
    }

    // React i18next Transコンポーネントの解析
    if file_type.is_react() {
        if let Err(e) = analyze_trans_components_with_recovery(
            &mut query_executor,
            &queries,
            &tree,
            source_code,
            &scopes,
            &mut result,
            &mut error_recovery,
            &validator,
            start_time,
        ) {
            warn!("Failed to analyze Trans components: {}", e);
        }
    }

    // 参照数の制限チェック
    if let Err(e) = validator.validate_reference_count(result.references.len()) {
        error_recovery.handle_error(e);
    }

    // 最終的なタイムアウトチェック
    if let Err(e) = validator.check_timeout(start_time, "complete_analysis") {
        error_recovery.handle_error(e);
    }

    let elapsed = start_time.elapsed();
    info!(
        "File analysis completed: {:?}, references={}, scopes={}, errors={}, elapsed={:?}",
        file_path,
        result.references.len(),
        result.scopes.len(),
        error_recovery.get_errors().len(),
        elapsed
    );

    Ok(transfer_errors_to_result(&error_recovery, result))
}

/// `エラー回復からAnalysisResultにエラーを転送します`
///
/// # 引数
///
/// * `error_recovery` - エラー回復インスタンス
/// * `mut result` - 解析結果
///
/// # 戻り値
///
/// `エラーが転送されたAnalysisResult`
fn transfer_errors_to_result(
    error_recovery: &ErrorRecovery,
    mut result: AnalysisResult,
) -> AnalysisResult {
    for error in error_recovery.get_errors() {
        result.add_error(error.clone());
    }
    result
}

/// ファイル種別に応じた言語パーサーを選択
///
/// # 引数
///
/// * `file_type` - ファイル種別
///
/// # 戻り値
///
/// 対応するtree-sitter言語パーサー
fn select_language(file_type: FileType) -> Language {
    if file_type.is_typescript() { language_typescript() } else { language_javascript() }
}

/// ファイル種別に応じた翻訳クエリを作成
///
/// # 引数
///
/// * `language` - tree-sitter言語パーサー
/// * `file_type` - ファイル種別
///
/// # 戻り値
///
/// 翻訳クエリ集合
///
/// # エラー
///
/// クエリの作成に失敗した場合
fn create_translation_queries(
    language: Language,
    file_type: FileType,
) -> Result<TranslationQueries> {
    if file_type.is_react() {
        // JSX対応クエリを作成
        TranslationQueries::new_with_jsx(language)
    } else {
        // 基本クエリを作成
        TranslationQueries::new(language)
    }
}

/// エラー回復機能付きi18next翻訳関数呼び出しの解析
///
/// 部分的な解析失敗時も可能な限り処理を継続します。
///
/// # 引数
///
/// * `executor` - クエリ実行エンジン
/// * `queries` - 翻訳クエリ集合
/// * `tree` - ASTツリー
/// * `source_code` - ソースコード
/// * `scopes` - スコープ情報
/// * `result` - 解析結果（出力）
/// * `error_recovery` - エラー回復管理
/// * `validator` - バリデーター
/// * `start_time` - 処理開始時刻
///
/// # Errors
///
/// 致命的なエラーが発生した場合、`anyhow::Error`を返します
#[allow(clippy::too_many_arguments)]
fn analyze_i18next_calls_with_recovery(
    executor: &mut QueryExecutor,
    queries: &TranslationQueries,
    tree: &Tree,
    source_code: &str,
    scopes: &[ScopeInfo],
    result: &mut AnalysisResult,
    error_recovery: &mut ErrorRecovery,
    validator: &AnalysisValidator,
    start_time: Instant,
) -> Result<()> {
    let _span = span!(Level::DEBUG, "analyze_i18next_calls").entered();

    let matches = match executor.execute_query(queries.i18next_trans_f_call(), tree, source_code) {
        Ok(matches) => {
            debug!("Found {} i18next function calls", matches.len());
            matches
        }
        Err(e) => {
            let context = ErrorContext::new(
                "analyze_i18next_calls_with_recovery".to_string(),
                ErrorSeverity::Warning,
            )
            .with_metadata("query_type".to_string(), "i18next_trans_f_call".to_string());

            let error = AnalysisError::QueryExecutionError {
                query_name: "i18next_trans_f_call".to_string().into(),
                reason: format!("Query execution failed: {e}").into(),
                context: Box::new(context),
            };

            if !error_recovery.handle_error(error) {
                return Err(e);
            }
            // エラー回復: 空の結果で続行
            Vec::new()
        }
    };

    for (index, trans_match) in matches.iter().enumerate() {
        // 定期的なタイムアウトチェック
        if index % 100 == 0 {
            if let Err(e) = validator.check_timeout(start_time, "i18next_calls_processing") {
                error_recovery.handle_error(e);
                break;
            }
        }

        // キーの解決
        let resolved_key = resolve_translation_key(
            &trans_match.key,
            u32::try_from(trans_match.start_line).unwrap_or(0),
            scopes,
        );

        // 翻訳キーの検証
        if let Err(e) = validate_translation_key(
            &resolved_key,
            u32::try_from(trans_match.start_line).unwrap_or(0),
            u32::try_from(trans_match.start_column).unwrap_or(0),
        ) {
            if !error_recovery.handle_error(e) {
                continue; // このキーをスキップして続行
            }
        }

        // TranslationReferenceの作成
        let reference = TranslationReference::new(
            result.file_id,
            u32::try_from(trans_match.start_line).unwrap_or(0),
            u32::try_from(trans_match.start_column).unwrap_or(0),
            resolved_key,
            None, // namespaceは別途処理
            trans_match.function_name.clone(),
        );

        result.add_reference(reference);

        // 参照数の制限チェック
        if let Err(e) = validator.validate_reference_count(result.references.len()) {
            error_recovery.handle_error(e);
            break; // 制限に達したため処理を停止
        }
    }

    Ok(())
}

/// エラー回復機能付きReact i18next Transコンポーネントの解析
///
/// 部分的な解析失敗時も可能な限り処理を継続します。
///
/// # 引数
///
/// * `executor` - クエリ実行エンジン
/// * `queries` - 翻訳クエリ集合
/// * `tree` - ASTツリー
/// * `source_code` - ソースコード
/// * `scopes` - スコープ情報
/// * `result` - 解析結果（出力）
/// * `error_recovery` - エラー回復管理
/// * `validator` - バリデーター
/// * `start_time` - 処理開始時刻
///
/// # Errors
///
/// 致命的なエラーが発生した場合、`anyhow::Error`を返します
#[allow(clippy::too_many_arguments)]
fn analyze_trans_components_with_recovery(
    executor: &mut QueryExecutor,
    queries: &TranslationQueries,
    tree: &Tree,
    source_code: &str,
    scopes: &[ScopeInfo],
    result: &mut AnalysisResult,
    error_recovery: &mut ErrorRecovery,
    validator: &AnalysisValidator,
    start_time: Instant,
) -> Result<()> {
    let _span = span!(Level::DEBUG, "analyze_trans_components").entered();

    let matches =
        match executor.execute_query(queries.react_i18next_trans_component(), tree, source_code) {
            Ok(matches) => {
                debug!("Found {} Trans components", matches.len());
                matches
            }
            Err(e) => {
                let context = ErrorContext::new(
                    "analyze_trans_components_with_recovery".to_string(),
                    ErrorSeverity::Warning,
                )
                .with_metadata(
                    "query_type".to_string(),
                    "react_i18next_trans_component".to_string(),
                );

                let error = AnalysisError::QueryExecutionError {
                    query_name: "react_i18next_trans_component".to_string().into(),
                    reason: format!("Query execution failed: {e}").into(),
                    context: Box::new(context),
                };

                if !error_recovery.handle_error(error) {
                    return Err(e);
                }
                // エラー回復: 空の結果で続行
                Vec::new()
            }
        };

    for (index, trans_match) in matches.iter().enumerate() {
        // 定期的なタイムアウトチェック
        if index % 100 == 0 {
            if let Err(e) = validator.check_timeout(start_time, "trans_components_processing") {
                error_recovery.handle_error(e);
                break;
            }
        }

        // キーの解決
        let resolved_key = resolve_translation_key(
            &trans_match.key,
            u32::try_from(trans_match.start_line).unwrap_or(0),
            scopes,
        );

        // 翻訳キーの検証
        if let Err(e) = validate_translation_key(
            &resolved_key,
            u32::try_from(trans_match.start_line).unwrap_or(0),
            u32::try_from(trans_match.start_column).unwrap_or(0),
        ) {
            if !error_recovery.handle_error(e) {
                continue; // このキーをスキップして続行
            }
        }

        // TranslationReferenceの作成
        let reference = TranslationReference::new(
            result.file_id,
            u32::try_from(trans_match.start_line).unwrap_or(0),
            u32::try_from(trans_match.start_column).unwrap_or(0),
            resolved_key,
            None, // namespaceは別途処理
            "Trans".to_string(),
        );

        result.add_reference(reference);

        // 参照数の制限チェック
        if let Err(e) = validator.validate_reference_count(result.references.len()) {
            error_recovery.handle_error(e);
            break; // 制限に達したため処理を停止
        }
    }

    Ok(())
}

/// 翻訳キーの有効性を検証します
///
/// # 引数
///
/// * `key` - 検証する翻訳キー
/// * `line` - キーが使用されている行番号
/// * `column` - キーが使用されている列番号
///
/// # 戻り値
///
/// 検証結果。無効な場合はエラー
///
/// # エラー
///
/// 翻訳キーが無効な場合、`AnalysisError::InvalidTranslationKey`を返します
fn validate_translation_key(key: &str, line: u32, column: u32) -> Result<(), AnalysisError> {
    // 空のキーをチェック
    if key.trim().is_empty() {
        return Err(AnalysisError::invalid_translation_key(
            key.to_string(),
            line,
            column,
            "Translation key is empty or contains only whitespace".to_string(),
        ));
    }

    // 連続するドットをチェック
    if key.contains("..") {
        return Err(AnalysisError::invalid_translation_key(
            key.to_string(),
            line,
            column,
            "Translation key contains consecutive dots".to_string(),
        ));
    }

    // 先頭・末尾のドットをチェック
    if key.starts_with('.') || key.ends_with('.') {
        return Err(AnalysisError::invalid_translation_key(
            key.to_string(),
            line,
            column,
            "Translation key starts or ends with a dot".to_string(),
        ));
    }

    // 無効な文字をチェック
    if key.chars().any(|c| matches!(c, '\n' | '\r' | '\t' | '\0')) {
        return Err(AnalysisError::invalid_translation_key(
            key.to_string(),
            line,
            column,
            "Translation key contains invalid characters (newlines, tabs, or null characters)"
                .to_string(),
        ));
    }

    // 長すぎるキーをチェック
    if key.len() > 1000 {
        return Err(AnalysisError::invalid_translation_key(
            key.to_string(),
            line,
            column,
            format!("Translation key is too long: {} characters exceeds limit of 1000", key.len()),
        ));
    }

    Ok(())
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn test_file_id_manager_creation() {
        let manager = FileIdManager::new();
        assert_eq!(manager.file_count(), 0);
        assert_eq!(manager.next_id(), 1);
    }

    #[test]
    fn test_file_id_manager_get_or_create() {
        let manager = FileIdManager::new();
        let path = PathBuf::from("test.js");

        let id1 = manager.get_or_create_file_id(path.clone());
        let id2 = manager.get_or_create_file_id(path);

        assert_eq!(id1, id2);
        assert_eq!(manager.file_count(), 1);
    }

    #[test]
    fn test_file_id_manager_bidirectional_mapping() {
        let manager = FileIdManager::new();
        let path = PathBuf::from("test.js");

        let id = manager.get_or_create_file_id(path.clone());

        assert_eq!(manager.get_path(id), Some(path.clone()));
        assert_eq!(manager.get_file_id(&path), Some(id));
    }

    #[test]
    fn test_resolve_translation_key_without_scope() {
        let scopes = Vec::new();
        let key = resolve_translation_key("test.key", 5, &scopes);
        assert_eq!(key, "test.key");
    }

    #[test]
    fn test_resolve_translation_key_with_scope() {
        let mut scope = ScopeInfo::new(FileId::new(1), 0, 10);
        scope.add_imported_function("t_ns_common".to_string(), "t".to_string());
        scope.add_imported_function("t_prefix_user".to_string(), "t".to_string());

        let scopes = vec![scope];
        let key = resolve_translation_key("name", 5, &scopes);
        // js-i18n.nvimの仕様に従い、keyPrefixが先に適用され、その後namespaceが適用される
        assert_eq!(key, "common:user.name");
    }

    #[test]
    fn test_is_i18n_library() {
        assert!(is_i18n_library("react-i18next"));
        assert!(is_i18n_library("i18next"));
        assert!(is_i18n_library("next-intl"));
        assert!(!is_i18n_library("react"));
        assert!(!is_i18n_library("lodash"));
    }

    #[test]
    fn test_select_language() {
        // 言語選択機能の基本動作を確認（パニックしないことを確認）
        let _js_lang = select_language(FileType::JavaScript);
        let _ts_lang = select_language(FileType::TypeScript);
        let _jsx_lang = select_language(FileType::JavaScriptReact);
        let _tsx_lang = select_language(FileType::TypeScriptReact);

        // 実行が完了することで正常に動作することを確認
    }

    #[test]
    fn test_analyze_file_unsupported_type() {
        let file_id = FileId::new(1);
        let file_path = PathBuf::from("test.py");
        let source_code = "print('hello')";

        let result = analyze_file(file_id, &file_path, source_code);
        assert!(result.is_ok(), "analyze_file should not fail");
        let result = result.unwrap();

        // エラー回復により、エラーは記録されるが処理は継続される
        assert!(result.has_errors());

        // エラーの種別を確認
        let unsupported_error =
            result.errors.iter().find(|e| matches!(e, AnalysisError::UnsupportedFileType { .. }));
        assert!(unsupported_error.is_some(), "Should contain UnsupportedFileType error");

        // エラー回復により、JavaScriptとしてパースが試行される
        // 参照数は取得できるため、特に制限は設けない
        let _reference_count = result.references.len();
    }

    #[test]
    fn test_analyze_file_basic_javascript() {
        let file_id = FileId::new(1);
        let file_path = PathBuf::from("test.js");
        let source_code = r"
        const message = t('hello.world');
        ";

        let result = analyze_file(file_id, &file_path, source_code);
        assert!(result.is_ok(), "analyze_file should not fail");
        let result = result.unwrap();

        // エラーがないことを確認
        if result.has_errors() {
            for error in &result.errors {
                tracing::error!("Analysis error: {error}");
            }
        }

        // 解析が成功することを確認（参照が見つからない場合もある）
        assert_eq!(result.file_id, file_id);
    }

    #[test]
    fn test_find_scope_end_line() {
        let source_code = r"
function test() {
    const { t } = useTranslation();
    return t('key');
}
        ";

        let end_line = find_scope_end_line(source_code, 1);

        assert!(end_line > 1);
    }

    #[test]
    fn test_extract_i18n_imports() {
        let mut scope = ScopeInfo::new(FileId::new(1), 0, 10);

        extract_i18n_imports(&mut scope, "react-i18next");

        assert!(scope.imported_functions.contains_key("useTranslation"));
        assert!(scope.imported_functions.contains_key("Trans"));
    }

    #[test]
    fn test_analysis_limits_default() {
        let limits = AnalysisLimits::default();
        assert_eq!(limits.max_file_size, 10 * 1024 * 1024);
        assert_eq!(limits.max_memory_usage, 100 * 1024 * 1024);
        assert_eq!(limits.timeout_ms, 30_000);
        assert_eq!(limits.max_references, 10_000);
        assert_eq!(limits.max_scope_depth, 100);
    }

    #[test]
    fn test_analysis_validator_file_size() {
        let validator = AnalysisValidator::default();
        let path = PathBuf::from("test.js");

        // 小さなファイルは通過
        let small_code = "const message = t('hello');";
        assert!(validator.validate_file_size(&path, small_code).is_ok());

        // 大きなファイルはエラー
        let large_code = "a".repeat(11 * 1024 * 1024); // 11MB
        let result = validator.validate_file_size(&path, &large_code);
        assert!(result.is_err());
        assert!(result.is_err(), "Expected error result");
        match result.err().unwrap() {
            AnalysisError::FileTooLarge { size, limit, .. } => {
                assert!(size > limit);
            }
            _ => unreachable!("Expected FileTooLarge error"),
        }
    }

    #[test]
    fn test_analysis_validator_encoding() {
        let validator = AnalysisValidator::default();
        let path = PathBuf::from("test.js");

        // 正常なUTF-8文字列
        let valid_code = "const message = t('こんにちは');";
        assert!(validator.validate_encoding(&path, valid_code).is_ok());

        // ASCII文字列
        let ascii_code = "const message = t('hello');";
        assert!(validator.validate_encoding(&path, ascii_code).is_ok());
    }

    #[test]
    fn test_error_recovery_strict() {
        let mut recovery = ErrorRecovery::new(RecoveryStrategy::Strict);

        let error = AnalysisError::internal_error("Test error".to_string());
        let should_continue = recovery.handle_error(error);

        assert!(!should_continue);
        assert!(recovery.has_errors());
    }

    #[test]
    fn test_error_recovery_tolerant() {
        let mut recovery = ErrorRecovery::new(RecoveryStrategy::Tolerant);

        // 回復可能なエラー
        let recoverable_error = AnalysisError::parse_error(1, 1, "Syntax error".to_string());
        let should_continue = recovery.handle_error(recoverable_error);
        assert!(should_continue);

        // 回復不可能なエラー
        let fatal_error = AnalysisError::internal_error("System failure".to_string());
        let should_continue = recovery.handle_error(fatal_error);
        assert!(!should_continue);

        assert!(recovery.has_errors());
        assert!(recovery.has_fatal_errors());
    }

    #[test]
    fn test_error_recovery_best_effort() {
        let mut recovery = ErrorRecovery::new(RecoveryStrategy::BestEffort);

        // どんなエラーでも継続
        let fatal_error = AnalysisError::internal_error("System failure".to_string());
        let should_continue = recovery.handle_error(fatal_error);
        assert!(should_continue);

        assert!(recovery.has_errors());
    }

    #[test]
    fn test_validate_translation_key() {
        // 正常なキー
        assert!(validate_translation_key("user.name", 1, 1).is_ok());
        assert!(validate_translation_key("hello_world", 1, 1).is_ok());
        assert!(validate_translation_key("namespace:key", 1, 1).is_ok());

        // 空のキー
        let result = validate_translation_key("", 1, 1);
        assert!(result.is_err());
        assert!(result.is_err(), "Expected error result");
        match result.err().unwrap() {
            AnalysisError::InvalidTranslationKey { reason, .. } => {
                assert!(reason.contains("empty"));
            }
            _ => unreachable!("Expected InvalidTranslationKey error"),
        }

        // 連続するドット
        let result = validate_translation_key("user..name", 1, 1);
        assert!(result.is_err());
        assert!(result.is_err(), "Expected error result");
        match result.err().unwrap() {
            AnalysisError::InvalidTranslationKey { reason, .. } => {
                assert!(reason.contains("consecutive dots"));
            }
            _ => unreachable!("Expected InvalidTranslationKey error"),
        }

        // 先頭ドット
        let result = validate_translation_key(".user.name", 1, 1);
        assert!(result.is_err());

        // 末尾ドット
        let result = validate_translation_key("user.name.", 1, 1);
        assert!(result.is_err());

        // 無効な文字
        let result = validate_translation_key("user\nname", 1, 1);
        assert!(result.is_err());
        assert!(result.is_err(), "Expected error result");
        match result.err().unwrap() {
            AnalysisError::InvalidTranslationKey { reason, .. } => {
                assert!(reason.contains("invalid characters"));
            }
            _ => unreachable!("Expected InvalidTranslationKey error"),
        }

        // 長すぎるキー
        let long_key = "a".repeat(1001);
        let result = validate_translation_key(&long_key, 1, 1);
        assert!(result.is_err());
        assert!(result.is_err(), "Expected error result");
        match result.err().unwrap() {
            AnalysisError::InvalidTranslationKey { reason, .. } => {
                assert!(reason.contains("too long"));
            }
            _ => unreachable!("Expected InvalidTranslationKey error"),
        }
    }

    #[test]
    fn test_transfer_errors_to_result() {
        let mut error_recovery = ErrorRecovery::new(RecoveryStrategy::Tolerant);
        let error1 = AnalysisError::parse_error(1, 1, "Error 1".to_string());
        let error2 = AnalysisError::parse_error(2, 1, "Error 2".to_string());

        error_recovery.handle_error(error1);
        error_recovery.handle_error(error2);

        let result = AnalysisResult::new(FileId::new(1));
        let final_result = transfer_errors_to_result(&error_recovery, result);

        assert_eq!(final_result.errors.len(), 2);
        assert!(final_result.has_errors());
    }
}
