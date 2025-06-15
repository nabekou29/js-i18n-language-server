//! i18n言語サーバーの基本データ構造定義
//!
//! このモジュールは、JavaScript/TypeScript向けi18n言語サーバーで使用される
//! 基本的なデータ構造を定義します。ADR-003に準拠したFileId型とTranslationReference構造体、
//! およびスコープ管理、解析結果、ファイル種別の型定義を含みます。
//!
//! # 設計原則
//!
//! - メモリ効率化のためFileId型を使用してファイルパスを数値で管理
//! - 厳格なエラーハンドリングでpanic禁止、Result型を適切に使用
//! - すべてのパブリック項目に文書化を提供
//!
//! # 作成者
//! @nabekou29
//!
//! # 作成日
//! 2025-06-15
//!
//! # 更新日
//! 2025-06-15 - エラーハンドリング強化実装

use std::collections::HashMap;
use std::fmt;
use std::path::{
    Path,
    PathBuf,
};
use std::time::SystemTime;

use thiserror::Error;
use tracing::{
    Level,
    event,
};

/// ファイルパスを数値で表現するID型
///
/// メモリ効率化のために、ファイルパスを数値IDで管理します。
/// ADR-003で決定された設計に従い、文字列の代わりに数値を使用することで
/// メモリ使用量を削減し、比較処理を高速化します。
///
/// # 例
///
/// ```rust
/// use js_i18n_language_server::types::FileId;
///
/// let file_id = FileId(1);
/// assert_eq!(file_id.0, 1);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FileId(pub u32);

impl FileId {
    /// `新しいFileIdを作成します`
    ///
    /// # 引数
    ///
    /// * `id` - ファイルを識別する数値ID
    ///
    /// # 戻り値
    ///
    /// `指定されたIDを持つFileId`
    #[must_use]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// `FileIdの数値を取得します`
    ///
    /// # 戻り値
    ///
    /// `FileIdの内部数値`
    #[must_use]
    pub const fn as_u32(&self) -> u32 {
        self.0
    }
}

/// 文言取得関数の呼び出し参照情報
///
/// i18n関数呼び出し（`i18n.t('key')`, `t('key')`, `<Trans key='key' />`等）の
/// 詳細な参照情報を保持します。位置情報、使用されているキー、名前空間などの
/// 情報を含み、補完や診断機能の基盤となります。
///
/// # フィールド
///
/// * `file_id` - 参照が存在するファイルのID
/// * `line` - 行番号（0ベース）
/// * `column` - 列番号（0ベース）
/// * `key` - 使用されている翻訳キー
/// * `namespace` - 名前空間（存在する場合）
/// * `function_name` - 呼び出し関数名（t, i18n.t, Trans等）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranslationReference {
    /// 参照が存在するファイルのID
    pub file_id: FileId,
    /// 行番号（0ベース）
    pub line: u32,
    /// 列番号（0ベース）
    pub column: u32,
    /// 使用されている翻訳キー
    pub key: String,
    /// 名前空間（i18nextなどで使用）
    pub namespace: Option<String>,
    /// 呼び出し関数名（t, i18n.t, Trans等）
    pub function_name: String,
}

impl TranslationReference {
    /// `新しいTranslationReferenceを作成します`
    ///
    /// # 引数
    ///
    /// * `file_id` - 参照が存在するファイルのID
    /// * `line` - 行番号（0ベース）
    /// * `column` - 列番号（0ベース）
    /// * `key` - 使用されている翻訳キー
    /// * `namespace` - 名前空間（オプション）
    /// * `function_name` - 呼び出し関数名
    ///
    /// # 戻り値
    ///
    /// `新しいTranslationReference`
    #[must_use]
    pub const fn new(
        file_id: FileId,
        line: u32,
        column: u32,
        key: String,
        namespace: Option<String>,
        function_name: String,
    ) -> Self {
        Self { file_id, line, column, key, namespace, function_name }
    }
}

/// スコープ管理情報
///
/// JavaScript/TypeScriptのスコープ階層を管理し、変数の有効範囲や
/// import文の影響範囲を追跡します。i18n関数のインポート状況や
/// 別名定義などを含みます。
///
/// # フィールド
///
/// * `file_id` - スコープが存在するファイルのID
/// * `start_line` - スコープ開始行（0ベース）
/// * `end_line` - スコープ終了行（0ベース）
/// * `imported_functions` - インポートされたi18n関数のマップ（別名 -> 元の名前）
/// * `parent_scope` - 親スコープのID（存在する場合）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeInfo {
    /// スコープが存在するファイルのID
    pub file_id: FileId,
    /// スコープ開始行（0ベース）
    pub start_line: u32,
    /// スコープ終了行（0ベース）
    pub end_line: u32,
    /// インポートされたi18n関数のマップ（別名 -> 元の名前）
    pub imported_functions: HashMap<String, String>,
    /// 親スコープのID（存在する場合）
    pub parent_scope: Option<u32>,
}

impl ScopeInfo {
    /// `新しいScopeInfoを作成します`
    ///
    /// # 引数
    ///
    /// * `file_id` - スコープが存在するファイルのID
    /// * `start_line` - スコープ開始行
    /// * `end_line` - スコープ終了行
    ///
    /// # 戻り値
    ///
    /// `新しいScopeInfo`
    #[must_use]
    pub fn new(file_id: FileId, start_line: u32, end_line: u32) -> Self {
        Self {
            file_id,
            start_line,
            end_line,
            imported_functions: HashMap::new(),
            parent_scope: None,
        }
    }

    /// インポートされた関数を追加します
    ///
    /// # 引数
    ///
    /// * `alias` - 関数の別名
    /// * `original` - 元の関数名
    pub fn add_imported_function(&mut self, alias: String, original: String) {
        self.imported_functions.insert(alias, original);
    }

    /// 指定された行がこのスコープ内にあるかを判定します
    ///
    /// # 引数
    ///
    /// * `line` - 判定する行番号
    ///
    /// # 戻り値
    ///
    /// スコープ内にある場合はtrue
    #[must_use]
    pub const fn contains_line(&self, line: u32) -> bool {
        line >= self.start_line && line <= self.end_line
    }
}

/// ファイル解析結果
///
/// 単一ファイルの解析結果を保持します。見つかった翻訳参照、
/// スコープ情報、エラー情報を含みます。
///
/// # フィールド
///
/// * `file_id` - 解析対象ファイルのID
/// * `references` - 見つかった翻訳参照のリスト
/// * `scopes` - ファイル内のスコープ情報のリスト
/// * `errors` - 解析中に発生したエラーのリスト
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalysisResult {
    /// 解析対象ファイルのID
    pub file_id: FileId,
    /// 見つかった翻訳参照のリスト
    pub references: Vec<TranslationReference>,
    /// ファイル内のスコープ情報のリスト
    pub scopes: Vec<ScopeInfo>,
    /// 解析中に発生したエラーのリスト
    pub errors: Vec<AnalysisError>,
}

impl AnalysisResult {
    /// `新しいAnalysisResultを作成します`
    ///
    /// # 引数
    ///
    /// * `file_id` - 解析対象ファイルのID
    ///
    /// # 戻り値
    ///
    /// `新しいAnalysisResult`
    #[must_use]
    pub const fn new(file_id: FileId) -> Self {
        Self { file_id, references: Vec::new(), scopes: Vec::new(), errors: Vec::new() }
    }

    /// 翻訳参照を追加します
    ///
    /// # 引数
    ///
    /// * `reference` - 追加する翻訳参照
    pub fn add_reference(&mut self, reference: TranslationReference) {
        self.references.push(reference);
    }

    /// スコープ情報を追加します
    ///
    /// # 引数
    ///
    /// * `scope` - 追加するスコープ情報
    pub fn add_scope(&mut self, scope: ScopeInfo) {
        self.scopes.push(scope);
    }

    /// エラーを追加します
    ///
    /// # 引数
    ///
    /// * `error` - 追加するエラー
    pub fn add_error(&mut self, error: AnalysisError) {
        self.errors.push(error);
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
}

/// サポートされるファイル種別
///
/// JavaScript/TypeScriptエコシステムでサポートされるファイル形式を
/// 定義します。各ファイル種別に応じて異なる解析ルールが適用されます。
///
/// # バリアント
///
/// * `JavaScript` - .jsファイル
/// * `TypeScript` - .tsファイル
/// * `JavaScriptReact` - .jsxファイル（React JSX）
/// * `TypeScriptReact` - .tsxファイル（React TSX）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileType {
    /// .jsファイル
    JavaScript,
    /// .tsファイル
    TypeScript,
    /// .jsxファイル（React JSX）
    JavaScriptReact,
    /// .tsxファイル（React TSX）
    TypeScriptReact,
}

impl FileType {
    /// `ファイルパスからFileTypeを判定します`
    ///
    /// # 引数
    ///
    /// * `path` - 判定するファイルパス
    ///
    /// # 戻り値
    ///
    /// 判定結果のResult。サポートされていない拡張子の場合はエラー
    ///
    /// # Errors
    ///
    /// サポートされていないファイル拡張子の場合、`AnalysisError::UnsupportedFileType`を返します
    pub fn from_path(path: &Path) -> Result<Self, AnalysisError> {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("js") => Ok(Self::JavaScript),
            Some("ts") => Ok(Self::TypeScript),
            Some("jsx") => Ok(Self::JavaScriptReact),
            Some("tsx") => Ok(Self::TypeScriptReact),
            Some(ext) => Err(AnalysisError::UnsupportedFileType {
                extension: ext.to_string().into_boxed_str(),
                path: Box::new(path.to_path_buf()),
                context: Box::new(ErrorContext::new(
                    "FileType::from_path".to_string(),
                    ErrorSeverity::Error,
                )),
            }),
            None => Err(AnalysisError::UnsupportedFileType {
                extension: "none".to_string().into_boxed_str(),
                path: Box::new(path.to_path_buf()),
                context: Box::new(ErrorContext::new(
                    "FileType::from_path".to_string(),
                    ErrorSeverity::Error,
                )),
            }),
        }
    }

    /// ファイル種別が TypeScript系（.ts, .tsx）かを判定します
    ///
    /// # 戻り値
    ///
    /// `TypeScript系の場合はtrue`
    #[must_use]
    pub const fn is_typescript(&self) -> bool {
        matches!(self, Self::TypeScript | Self::TypeScriptReact)
    }

    /// ファイル種別がReact系（.jsx, .tsx）かを判定します
    ///
    /// # 戻り値
    ///
    /// React系の場合はtrue
    #[must_use]
    pub const fn is_react(&self) -> bool {
        matches!(self, Self::JavaScriptReact | Self::TypeScriptReact)
    }

    /// ファイル種別の拡張子を取得します
    ///
    /// # 戻り値
    ///
    /// ファイル拡張子の文字列
    #[must_use]
    pub const fn extension(&self) -> &'static str {
        match self {
            Self::JavaScript => "js",
            Self::TypeScript => "ts",
            Self::JavaScriptReact => "jsx",
            Self::TypeScriptReact => "tsx",
        }
    }
}

/// エラーの詳細位置情報
///
/// ソースコード内での正確な位置を表現します。
/// 1ベースの行・列番号とバイトオフセットを含みます。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    /// 行番号（1ベース）
    pub line: u32,
    /// 列番号（1ベース）
    pub column: u32,
    /// ファイル内のバイトオフセット（0ベース）
    pub byte_offset: u32,
}

impl Position {
    /// 新しいPositionを作成します
    ///
    /// # 引数
    ///
    /// * `line` - 行番号（1ベース）
    /// * `column` - 列番号（1ベース）
    /// * `byte_offset` - バイトオフセット（0ベース）
    ///
    /// # 戻り値
    ///
    /// 新しいPosition
    #[must_use]
    pub const fn new(line: u32, column: u32, byte_offset: u32) -> Self {
        Self { line, column, byte_offset }
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

/// ソースコードの範囲情報
///
/// エラーが発生したソースコードの範囲を表現します。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceRange {
    /// 開始位置
    pub start: Position,
    /// 終了位置
    pub end: Position,
}

impl SourceRange {
    /// `新しいSourceRangeを作成します`
    ///
    /// # 引数
    ///
    /// * `start` - 開始位置
    /// * `end` - 終了位置
    ///
    /// # 戻り値
    ///
    /// `新しいSourceRange`
    #[must_use]
    pub const fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }

    /// `単一位置からSourceRangeを作成します`
    ///
    /// # 引数
    ///
    /// * `position` - 位置情報
    ///
    /// # 戻り値
    ///
    /// 同じ開始・終了位置を持つSourceRange
    #[must_use]
    pub const fn from_position(position: Position) -> Self {
        Self { start: position, end: position }
    }
}

impl fmt::Display for SourceRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.start == self.end {
            write!(f, "{}", self.start)
        } else {
            write!(f, "{}-{}", self.start, self.end)
        }
    }
}

/// エラーの重要度
///
/// エラーの深刻度を分類し、適切な処理レベルを決定します。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorSeverity {
    /// 致命的エラー（処理継続不可能）
    Fatal,
    /// エラー（機能に影響あり）
    Error,
    /// 警告（軽微な問題）
    Warning,
    /// 情報（参考情報）
    Info,
}

impl fmt::Display for ErrorSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fatal => write!(f, "FATAL"),
            Self::Error => write!(f, "ERROR"),
            Self::Warning => write!(f, "WARNING"),
            Self::Info => write!(f, "INFO"),
        }
    }
}

/// エラーコンテキスト情報
///
/// エラーが発生した際の詳細なコンテキスト情報を保持します。
/// トレーサブルなエラー処理とデバッグ支援のために使用されます。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorContext {
    /// ファイルID
    pub file_id: Option<FileId>,
    /// ファイルパス
    pub file_path: Option<PathBuf>,
    /// 発生時刻（UNIX時間）
    pub timestamp: u64,
    /// エラー発生関数
    pub function_name: String,
    /// エラーの重要度
    pub severity: ErrorSeverity,
    /// 追加のメタデータ
    pub metadata: HashMap<String, String>,
}

impl ErrorContext {
    /// `新しいErrorContextを作成します`
    ///
    /// # 引数
    ///
    /// * `function_name` - エラー発生関数名
    /// * `severity` - エラーの重要度
    ///
    /// # 戻り値
    ///
    /// `新しいErrorContext`
    #[must_use]
    pub fn new(function_name: String, severity: ErrorSeverity) -> Self {
        Self {
            file_id: None,
            file_path: None,
            timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            function_name,
            severity,
            metadata: HashMap::new(),
        }
    }

    /// ファイル情報を設定します
    ///
    /// # 引数
    ///
    /// * `file_id` - ファイルID
    /// * `file_path` - ファイルパス
    ///
    /// # 戻り値
    ///
    /// `更新されたErrorContext`
    #[must_use]
    pub fn with_file(mut self, file_id: FileId, file_path: PathBuf) -> Self {
        self.file_id = Some(file_id);
        self.file_path = Some(file_path);
        self
    }

    /// メタデータを追加します
    ///
    /// # 引数
    ///
    /// * `key` - メタデータキー
    /// * `value` - メタデータ値
    ///
    /// # 戻り値
    ///
    /// `更新されたErrorContext`
    #[must_use]
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

impl Default for ErrorContext {
    fn default() -> Self {
        Self::new("unknown".to_string(), ErrorSeverity::Error)
    }
}

/// 解析エラー型定義
///
/// i18n言語サーバーで発生する可能性のあるエラーを定義します。
/// thiserrorクレートを使用して適切なエラーハンドリングを提供します。
/// エラーコンテキスト情報、種別の細分化、位置情報の詳細化を含みます。
///
/// # パフォーマンス最適化
///
/// 大きなフィールド（ErrorContext、String、PathBuf等）をBox化することで
/// enum全体のサイズを削減し、メモリ効率を向上させています。
///
/// # バリアント
///
/// ## ファイル関連エラー
/// * `UnsupportedFileType` - サポートされていないファイル種別
/// * `FileTooLarge` - ファイルサイズ制限超過
/// * `FileReadError` - ファイル読み込みエラー
/// * `FileEncodingError` - ファイルエンコーディングエラー
///
/// ## 構文解析エラー
/// * `ParseError` - 構文解析エラー
/// * `TreeSitterError` - tree-sitter解析エラー
/// * `QueryExecutionError` - クエリ実行エラー
///
/// ## 翻訳関連エラー
/// * `InvalidTranslationKey` - 無効な翻訳キー
/// * `MissingTranslation` - 翻訳が見つからない
/// * `DuplicateTranslationKey` - 重複する翻訳キー
/// * `InvalidNamespace` - 無効な名前空間
/// * `TranslationResourceError` - 翻訳リソースエラー
///
/// ## システムエラー
/// * `MemoryLimitExceeded` - メモリ使用量制限超過
/// * `TimeoutError` - 処理タイムアウト
/// * `InternalError` - 内部エラー
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum AnalysisError {
    /// サポートされていないファイル種別エラー
    #[error("Unsupported file type: .{extension} (file: {path:?})")]
    UnsupportedFileType {
        /// ファイル拡張子
        extension: Box<str>,
        /// ファイルパス
        path: Box<PathBuf>,
        /// エラーコンテキスト
        context: Box<ErrorContext>,
    },

    /// ファイルサイズ制限超過エラー
    #[error("File too large: {size} bytes exceeds limit of {limit} bytes (file: {path:?})")]
    FileTooLarge {
        /// ファイルサイズ（バイト）
        size: u64,
        /// サイズ制限（バイト）
        limit: u64,
        /// ファイルパス
        path: Box<PathBuf>,
        /// エラーコンテキスト
        context: Box<ErrorContext>,
    },

    /// ファイル読み込みエラー
    #[error("Failed to read file: {path:?} - {reason}")]
    FileReadError {
        /// ファイルパス
        path: Box<PathBuf>,
        /// エラー理由
        reason: Box<str>,
        /// エラーコンテキスト
        context: Box<ErrorContext>,
    },

    /// ファイルエンコーディングエラー
    #[error("Invalid file encoding: {path:?} - expected UTF-8")]
    FileEncodingError {
        /// ファイルパス
        path: Box<PathBuf>,
        /// 無効なバイト位置
        byte_position: Option<usize>,
        /// エラーコンテキスト
        context: Box<ErrorContext>,
    },

    /// 構文解析エラー
    #[error("Parse error at line {line}, column {column}: {message}")]
    ParseError {
        /// エラーが発生した行番号（1ベース）
        line: u32,
        /// エラーが発生した列番号（1ベース）
        column: u32,
        /// エラーメッセージ
        message: Box<str>,
        /// ソースコード範囲
        source_range: Option<Box<SourceRange>>,
        /// エラーコンテキスト
        context: Box<ErrorContext>,
    },

    /// tree-sitter解析エラー
    #[error("Tree-sitter parsing failed: {reason}")]
    TreeSitterError {
        /// エラー理由
        reason: Box<str>,
        /// 失敗した位置
        position: Option<Position>,
        /// エラーコンテキスト
        context: Box<ErrorContext>,
    },

    /// クエリ実行エラー
    #[error("Query execution failed: {query_name} - {reason}")]
    QueryExecutionError {
        /// クエリ名
        query_name: Box<str>,
        /// エラー理由
        reason: Box<str>,
        /// エラーコンテキスト
        context: Box<ErrorContext>,
    },

    /// 無効な翻訳キーエラー
    #[error("Invalid translation key: '{key}' at line {line}, column {column} - {reason}")]
    InvalidTranslationKey {
        /// 無効な翻訳キー
        key: Box<str>,
        /// エラーが発生した行番号（1ベース）
        line: u32,
        /// エラーが発生した列番号（1ベース）
        column: u32,
        /// 無効である理由
        reason: Box<str>,
        /// ソースコード範囲
        source_range: Option<Box<SourceRange>>,
        /// エラーコンテキスト
        context: Box<ErrorContext>,
    },

    /// 翻訳が見つからないエラー
    #[error("Missing translation for key: '{key}' in namespace: '{namespace:?}'")]
    MissingTranslation {
        /// 見つからない翻訳キー
        key: Box<str>,
        /// 名前空間
        namespace: Option<Box<str>>,
        /// 検索されたロケール
        locale: Option<Box<str>>,
        /// 使用位置
        usage_location: Option<Position>,
        /// エラーコンテキスト
        context: Box<ErrorContext>,
    },

    /// 重複する翻訳キーエラー
    #[error("Duplicate translation key: '{key}' found in multiple locations")]
    DuplicateTranslationKey {
        /// 重複する翻訳キー
        key: Box<str>,
        /// 最初の定義位置
        first_location: Option<Position>,
        /// 重複した定義位置
        duplicate_location: Option<Position>,
        /// エラーコンテキスト
        context: Box<ErrorContext>,
    },

    /// 無効な名前空間エラー
    #[error("Invalid namespace: '{namespace}' at line {line}, column {column} - {reason}")]
    InvalidNamespace {
        /// 無効な名前空間
        namespace: Box<str>,
        /// エラーが発生した行番号（1ベース）
        line: u32,
        /// エラーが発生した列番号（1ベース）
        column: u32,
        /// 無効である理由
        reason: Box<str>,
        /// エラーコンテキスト
        context: Box<ErrorContext>,
    },

    /// 翻訳リソースエラー
    #[error("Translation resource error: {resource_path:?} - {reason}")]
    TranslationResourceError {
        /// リソースファイルパス
        resource_path: Box<PathBuf>,
        /// エラー理由
        reason: Box<str>,
        /// エラーコンテキスト
        context: Box<ErrorContext>,
    },

    /// メモリ使用量制限超過エラー
    #[error("Memory limit exceeded: {current_usage} bytes exceeds limit of {limit} bytes")]
    MemoryLimitExceeded {
        /// 現在のメモリ使用量（バイト）
        current_usage: u64,
        /// メモリ制限（バイト）
        limit: u64,
        /// エラーコンテキスト
        context: Box<ErrorContext>,
    },

    /// 処理タイムアウトエラー
    #[error("Processing timeout: operation took longer than {timeout_ms}ms")]
    TimeoutError {
        /// タイムアウト時間（ミリ秒）
        timeout_ms: u64,
        /// 実行中だった操作
        operation: Box<str>,
        /// エラーコンテキスト
        context: Box<ErrorContext>,
    },

    /// 内部エラー
    #[error("Internal error: {message}")]
    InternalError {
        /// エラーメッセージ
        message: Box<str>,
        /// エラーコンテキスト
        context: Box<ErrorContext>,
    },
}

impl AnalysisError {
    /// `ParseErrorを作成します`
    ///
    /// # 引数
    ///
    /// * `line` - エラーが発生した行番号（1ベース）
    /// * `column` - エラーが発生した列番号（1ベース）
    /// * `message` - エラーメッセージ
    ///
    /// # 戻り値
    ///
    /// `新しいParseError`
    #[must_use]
    pub fn parse_error(line: u32, column: u32, message: String) -> Self {
        Self::ParseError {
            line,
            column,
            message: message.into_boxed_str(),
            source_range: None,
            context: Box::new(ErrorContext::new("parse_error".to_string(), ErrorSeverity::Error)),
        }
    }

    /// `ParseErrorを詳細情報付きで作成します`
    ///
    /// # 引数
    ///
    /// * `line` - エラーが発生した行番号（1ベース）
    /// * `column` - エラーが発生した列番号（1ベース）
    /// * `message` - エラーメッセージ
    /// * `source_range` - ソースコード範囲
    /// * `context` - エラーコンテキスト
    ///
    /// # 戻り値
    ///
    /// `新しいParseError`
    #[must_use]
    pub fn parse_error_with_context(
        line: u32,
        column: u32,
        message: String,
        source_range: Option<SourceRange>,
        context: ErrorContext,
    ) -> Self {
        Self::ParseError {
            line,
            column,
            message: message.into_boxed_str(),
            source_range: source_range.map(Box::new),
            context: Box::new(context),
        }
    }

    /// `InvalidTranslationKeyを作成します`
    ///
    /// # 引数
    ///
    /// * `key` - 無効な翻訳キー
    /// * `line` - エラーが発生した行番号（1ベース）
    /// * `column` - エラーが発生した列番号（1ベース）
    /// * `reason` - 無効である理由
    ///
    /// # 戻り値
    ///
    /// `新しいInvalidTranslationKey`
    #[must_use]
    pub fn invalid_translation_key(key: String, line: u32, column: u32, reason: String) -> Self {
        Self::InvalidTranslationKey {
            key: key.into_boxed_str(),
            line,
            column,
            reason: reason.into_boxed_str(),
            source_range: None,
            context: Box::new(ErrorContext::new(
                "invalid_translation_key".to_string(),
                ErrorSeverity::Error,
            )),
        }
    }

    /// `FileTooLargeエラーを作成します`
    ///
    /// # 引数
    ///
    /// * `size` - ファイルサイズ（バイト）
    /// * `limit` - サイズ制限（バイト）
    /// * `path` - ファイルパス
    ///
    /// # 戻り値
    ///
    /// `新しいFileTooLarge`
    #[must_use]
    pub fn file_too_large(size: u64, limit: u64, path: PathBuf) -> Self {
        Self::FileTooLarge {
            size,
            limit,
            path: Box::new(path),
            context: Box::new(ErrorContext::new(
                "file_too_large".to_string(),
                ErrorSeverity::Error,
            )),
        }
    }

    /// `TreeSitterErrorを作成します`
    ///
    /// # 引数
    ///
    /// * `reason` - エラー理由
    ///
    /// # 戻り値
    ///
    /// `新しいTreeSitterError`
    #[must_use]
    pub fn tree_sitter_error(reason: String) -> Self {
        Self::TreeSitterError {
            reason: reason.into_boxed_str(),
            position: None,
            context: Box::new(ErrorContext::new(
                "tree_sitter_error".to_string(),
                ErrorSeverity::Error,
            )),
        }
    }

    /// `InternalErrorを作成します`
    ///
    /// # 引数
    ///
    /// * `message` - エラーメッセージ
    ///
    /// # 戻り値
    ///
    /// `新しいInternalError`
    #[must_use]
    pub fn internal_error(message: String) -> Self {
        Self::InternalError {
            message: message.into_boxed_str(),
            context: Box::new(ErrorContext::new(
                "internal_error".to_string(),
                ErrorSeverity::Fatal,
            )),
        }
    }

    /// エラーの重要度を取得します
    ///
    /// # 戻り値
    ///
    /// エラーの重要度
    #[must_use]
    pub const fn severity(&self) -> ErrorSeverity {
        match self {
            Self::UnsupportedFileType { context, .. }
            | Self::FileTooLarge { context, .. }
            | Self::FileReadError { context, .. }
            | Self::FileEncodingError { context, .. }
            | Self::ParseError { context, .. }
            | Self::TreeSitterError { context, .. }
            | Self::QueryExecutionError { context, .. }
            | Self::InvalidTranslationKey { context, .. }
            | Self::MissingTranslation { context, .. }
            | Self::DuplicateTranslationKey { context, .. }
            | Self::InvalidNamespace { context, .. }
            | Self::TranslationResourceError { context, .. }
            | Self::MemoryLimitExceeded { context, .. }
            | Self::TimeoutError { context, .. }
            | Self::InternalError { context, .. } => context.severity,
        }
    }

    /// エラーログを出力します
    ///
    /// エラー内容と重要度に応じて適切なログレベルで出力します。
    pub fn log_error(&self) {
        match self.severity() {
            ErrorSeverity::Fatal => {
                event!(Level::ERROR, error = %self, "Fatal error occurred");
            }
            ErrorSeverity::Error => {
                event!(Level::ERROR, error = %self, "Error occurred");
            }
            ErrorSeverity::Warning => {
                event!(Level::WARN, error = %self, "Warning occurred");
            }
            ErrorSeverity::Info => {
                event!(Level::INFO, error = %self, "Info message");
            }
        }
    }

    /// エラーが回復可能かどうかを判定します
    ///
    /// # 戻り値
    ///
    /// 回復可能な場合はtrue
    #[must_use]
    pub const fn is_recoverable(&self) -> bool {
        !matches!(
            self,
            Self::InternalError { .. }
                | Self::MemoryLimitExceeded { .. }
                | Self::FileEncodingError { .. }
        )
    }

    /// エラーが発生した位置を取得します
    ///
    /// # 戻り値
    ///
    /// エラー位置情報（存在する場合）
    #[must_use]
    pub const fn position(&self) -> Option<Position> {
        match self {
            Self::ParseError { line, column, .. }
            | Self::InvalidTranslationKey { line, column, .. }
            | Self::InvalidNamespace { line, column, .. } => Some(Position::new(*line, *column, 0)),
            Self::TreeSitterError { position, .. } => *position,
            Self::MissingTranslation { usage_location, .. } => *usage_location,
            Self::DuplicateTranslationKey { first_location, .. } => *first_location,
            _ => None,
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_file_id_creation() {
        let file_id = FileId::new(42);
        assert_eq!(file_id.as_u32(), 42);
    }

    #[test]
    fn test_translation_reference_creation() {
        let reference = TranslationReference::new(
            FileId::new(1),
            10,
            5,
            "test.key".to_string(),
            Some("common".to_string()),
            "t".to_string(),
        );

        assert_eq!(reference.file_id, FileId::new(1));
        assert_eq!(reference.line, 10);
        assert_eq!(reference.column, 5);
        assert_eq!(reference.key, "test.key");
        assert_eq!(reference.namespace, Some("common".to_string()));
        assert_eq!(reference.function_name, "t");
    }

    #[test]
    fn test_scope_info_contains_line() {
        let scope = ScopeInfo::new(FileId::new(1), 5, 15);

        assert!(!scope.contains_line(4));
        assert!(scope.contains_line(5));
        assert!(scope.contains_line(10));
        assert!(scope.contains_line(15));
        assert!(!scope.contains_line(16));
    }

    #[test]
    fn test_file_type_from_path() {
        let js_path = PathBuf::from("test.js");
        let ts_path = PathBuf::from("test.ts");
        let jsx_path = PathBuf::from("test.jsx");
        let tsx_path = PathBuf::from("test.tsx");
        let unknown_path = PathBuf::from("test.py");

        assert!(matches!(FileType::from_path(&js_path), Ok(FileType::JavaScript)));
        assert!(matches!(FileType::from_path(&ts_path), Ok(FileType::TypeScript)));
        assert!(matches!(FileType::from_path(&jsx_path), Ok(FileType::JavaScriptReact)));
        assert!(matches!(FileType::from_path(&tsx_path), Ok(FileType::TypeScriptReact)));

        assert!(FileType::from_path(&unknown_path).is_err());
    }

    #[test]
    fn test_file_type_predicates() {
        assert!(!FileType::JavaScript.is_typescript());
        assert!(FileType::TypeScript.is_typescript());
        assert!(!FileType::JavaScriptReact.is_typescript());
        assert!(FileType::TypeScriptReact.is_typescript());

        assert!(!FileType::JavaScript.is_react());
        assert!(!FileType::TypeScript.is_react());
        assert!(FileType::JavaScriptReact.is_react());
        assert!(FileType::TypeScriptReact.is_react());
    }

    #[test]
    fn test_analysis_result_operations() {
        let mut result = AnalysisResult::new(FileId::new(1));

        assert!(!result.has_errors());

        result.add_error(AnalysisError::internal_error("Test error".to_string()));
        assert!(result.has_errors());

        let reference = TranslationReference::new(
            FileId::new(1),
            5,
            10,
            "test.key".to_string(),
            None,
            "t".to_string(),
        );
        result.add_reference(reference);

        assert_eq!(result.references.len(), 1);
    }

    #[test]
    fn test_position_creation() {
        let position = Position::new(10, 5, 100);
        assert_eq!(position.line, 10);
        assert_eq!(position.column, 5);
        assert_eq!(position.byte_offset, 100);
        assert_eq!(position.to_string(), "10:5");
    }

    #[test]
    fn test_source_range_creation() {
        let start = Position::new(10, 5, 100);
        let end = Position::new(10, 15, 110);
        let range = SourceRange::new(start, end);

        assert_eq!(range.start, start);
        assert_eq!(range.end, end);
        assert_eq!(range.to_string(), "10:5-10:15");

        let single_range = SourceRange::from_position(start);
        assert_eq!(single_range.start, start);
        assert_eq!(single_range.end, start);
        assert_eq!(single_range.to_string(), "10:5");
    }

    #[test]
    fn test_error_context_creation() {
        let context = ErrorContext::new("test_function".to_string(), ErrorSeverity::Error);

        assert_eq!(context.function_name, "test_function");
        assert_eq!(context.severity, ErrorSeverity::Error);
        assert!(context.file_id.is_none());

        let context_with_file = context.with_file(FileId::new(1), PathBuf::from("test.js"));

        assert_eq!(context_with_file.file_id, Some(FileId::new(1)));
        assert_eq!(context_with_file.file_path, Some(PathBuf::from("test.js")));
    }

    #[test]
    fn test_error_severity_display() {
        assert_eq!(ErrorSeverity::Fatal.to_string(), "FATAL");
        assert_eq!(ErrorSeverity::Error.to_string(), "ERROR");
        assert_eq!(ErrorSeverity::Warning.to_string(), "WARNING");
        assert_eq!(ErrorSeverity::Info.to_string(), "INFO");
    }

    #[test]
    fn test_analysis_error_severity() {
        let parse_error = AnalysisError::parse_error(10, 5, "Syntax error".to_string());
        assert_eq!(parse_error.severity(), ErrorSeverity::Error);

        let internal_error = AnalysisError::internal_error("System failure".to_string());
        assert_eq!(internal_error.severity(), ErrorSeverity::Fatal);
    }

    #[test]
    fn test_analysis_error_recoverability() {
        let parse_error = AnalysisError::parse_error(10, 5, "Syntax error".to_string());
        assert!(parse_error.is_recoverable());

        let internal_error = AnalysisError::internal_error("System failure".to_string());
        assert!(!internal_error.is_recoverable());

        let file_too_large =
            AnalysisError::file_too_large(1_000_000, 500_000, PathBuf::from("large.js"));
        assert!(file_too_large.is_recoverable());
    }

    #[test]
    fn test_enhanced_error_creation() {
        let error = AnalysisError::invalid_translation_key(
            "invalid..key".to_string(),
            10,
            5,
            "Key contains consecutive dots".to_string(),
        );

        match error {
            AnalysisError::InvalidTranslationKey { key, line, column, reason, .. } => {
                assert_eq!(&*key, "invalid..key");
                assert_eq!(line, 10);
                assert_eq!(column, 5);
                assert_eq!(&*reason, "Key contains consecutive dots");
            }
            _ => unreachable!("Expected InvalidTranslationKey"),
        }
    }
}
