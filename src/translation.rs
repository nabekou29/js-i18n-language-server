//! 翻訳リソース管理システム
//!
//! このモジュールは以下の機能を提供します：
//! - JSON/YAMLの翻訳ファイル読み込み
//! - DashMapベースの高速キャッシュ
//! - ネームスペース対応
//! - ファイル監視機能
//! - 各種i18nライブラリ形式対応

use std::{
    collections::HashMap,
    path::{
        Path,
        PathBuf,
    },
    sync::{
        Arc,
        RwLock,
        atomic::{
            AtomicBool,
            Ordering,
        },
    },
    time::SystemTime,
};

use dashmap::DashMap;
use notify::{
    Config,
    Event,
    EventKind,
    RecommendedWatcher,
    RecursiveMode,
    Watcher,
};
use serde::{
    Deserialize,
    Serialize,
};
use thiserror::Error;
use tokio::sync::mpsc;
use tracing::{
    debug,
    error,
    info,
    warn,
};
use walkdir::WalkDir;

/// 翻訳リソース管理に関するエラー型
#[derive(Error, Debug)]
pub enum TranslationError {
    /// ファイル読み込みエラー
    #[error("Failed to read file {path}: {source}")]
    FileRead {
        /// ファイルパス
        path: PathBuf,
        /// 元のエラー
        source: std::io::Error,
    },

    /// JSON解析エラー
    #[error("Failed to parse JSON file {path}: {source}")]
    JsonParse {
        /// ファイルパス
        path: PathBuf,
        /// 元のエラー
        source: serde_json::Error,
    },

    /// YAML解析エラー
    #[error("Failed to parse YAML file {path}: {source}")]
    YamlParse {
        /// ファイルパス
        path: PathBuf,
        /// 元のエラー
        source: serde_yaml::Error,
    },

    /// ファイル監視エラー
    #[error("Failed to watch file {path}: {source}")]
    FileWatch {
        /// ファイルパス
        path: PathBuf,
        /// 元のエラー
        source: notify::Error,
    },

    /// 無効なキー形式エラー
    #[error("Invalid translation key format: {key}")]
    InvalidKey {
        /// 無効なキー
        key: String,
    },

    /// 内部エラー
    #[error("Internal error: {message}")]
    InternalError {
        /// エラーメッセージ
        message: String,
    },

    /// ネームスペースが見つからないエラー
    #[error("Namespace not found: {namespace}")]
    NamespaceNotFound {
        /// 見つからないネームスペース
        namespace: String,
    },
}

/// 翻訳値を表す型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum TranslationValue {
    /// 単純な文字列値
    String(String),
    /// ネストしたオブジェクト
    Object(HashMap<String, TranslationValue>),
    /// 配列値（pluralization対応）
    Array(Vec<String>),
}

impl TranslationValue {
    /// 値を文字列として取得する
    ///
    /// # Returns
    /// - 文字列値の場合: その値
    /// - オブジェクトの場合: None
    /// - 配列の場合: 最初の要素（あれば）
    pub fn as_string(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            Self::Array(arr) => arr.first().map(String::as_str),
            Self::Object(_) => None,
        }
    }

    /// 値がオブジェクトかどうかを判定する
    ///
    /// # Returns
    /// オブジェクトの場合true
    #[must_use]
    pub const fn is_object(&self) -> bool {
        matches!(self, Self::Object(_))
    }

    /// オブジェクトの全キーを取得する
    ///
    /// # Returns
    /// オブジェクトの場合はキーのベクタ、それ以外は空のベクタ
    #[must_use]
    pub fn get_keys(&self) -> Vec<String> {
        match self {
            Self::Object(obj) => obj.keys().cloned().collect(),
            _ => Vec::new(),
        }
    }
}

/// 翻訳ファイルのメタデータ
#[derive(Debug, Clone)]
pub struct TranslationFileMetadata {
    /// ファイルパス
    pub path: PathBuf,
    /// 最終更新時刻
    pub last_modified: SystemTime,
    /// ファイル形式
    pub format: TranslationFileFormat,
    /// ネームスペース（指定されている場合）
    pub namespace: Option<String>,
}

/// 翻訳ファイルの形式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranslationFileFormat {
    /// JSON形式
    Json,
    /// YAML形式
    Yaml,
}

impl TranslationFileFormat {
    /// ファイル拡張子から形式を判定する
    ///
    /// # Arguments
    /// * `path` - ファイルパス
    ///
    /// # Returns
    /// 判定された形式（不明な場合はNone）
    #[must_use]
    pub fn from_path(path: &Path) -> Option<Self> {
        let ext = path.extension()?.to_str()?;
        match ext.to_lowercase().as_str() {
            "json" => Some(Self::Json),
            "yaml" | "yml" => Some(Self::Yaml),
            _ => None,
        }
    }
}

/// ネームスペース付きキー
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NamespacedKey {
    /// ネームスペース（オプション）
    pub namespace: Option<String>,
    /// キー
    pub key: String,
}

impl NamespacedKey {
    /// 新しいネームスペース付きキーを作成する
    ///
    /// # Arguments
    /// * `namespace` - ネームスペース
    /// * `key` - キー
    #[must_use]
    pub const fn new(namespace: Option<String>, key: String) -> Self {
        Self { namespace, key }
    }

    /// フル形式の文字列表現を取得する
    ///
    /// # Returns
    /// "namespace:key" または "key" 形式の文字列
    #[must_use]
    pub fn full_key(&self) -> String {
        self.namespace.as_ref().map_or_else(|| self.key.clone(), |ns| format!("{ns}:{}", self.key))
    }

    /// 文字列からネームスペース付きキーを解析する
    ///
    /// # Arguments
    /// * `input` - 解析する文字列
    ///
    /// # Returns
    /// 解析されたキー、または無効な場合はエラー
    ///
    /// # Errors
    /// 空の文字列や無効なフォーマットの場合
    pub fn parse(input: &str) -> Result<Self, TranslationError> {
        if input.is_empty() {
            return Err(TranslationError::InvalidKey { key: input.to_string() });
        }

        if let Some((namespace, key)) = input.split_once(':') {
            if namespace.is_empty() || key.is_empty() {
                return Err(TranslationError::InvalidKey { key: input.to_string() });
            }
            Ok(Self::new(Some(namespace.to_string()), key.to_string()))
        } else {
            Ok(Self::new(None, input.to_string()))
        }
    }
}

/// 翻訳キャッシュシステム
pub struct TranslationCache {
    /// 翻訳データのキャッシュ（ネームスペース -> キー -> 値）
    cache: DashMap<String, DashMap<String, TranslationValue>>,
    /// ファイルメタデータのキャッシュ
    file_metadata: DashMap<PathBuf, TranslationFileMetadata>,
    /// 監視対象ディレクトリ
    watched_dirs: RwLock<Vec<PathBuf>>,
    /// ファイル監視の停止フラグ
    should_stop_watching: Arc<AtomicBool>,
    /// ファイル監視タスクのハンドル
    watcher_handle: RwLock<Option<tokio::task::JoinHandle<()>>>,
}

impl Default for TranslationCache {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for TranslationCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TranslationCache")
            .field("cache", &self.cache)
            .field("file_metadata", &self.file_metadata)
            .field("watched_dirs", &self.watched_dirs)
            .field("should_stop_watching", &self.should_stop_watching)
            .field("watcher_handle", &"<JoinHandle>")
            .finish()
    }
}

impl TranslationCache {
    /// 新しい翻訳キャッシュを作成する
    #[must_use]
    pub fn new() -> Self {
        Self {
            cache: DashMap::new(),
            file_metadata: DashMap::new(),
            watched_dirs: RwLock::new(Vec::new()),
            should_stop_watching: Arc::new(AtomicBool::new(false)),
            watcher_handle: RwLock::new(None),
        }
    }

    /// 指定されたディレクトリから翻訳ファイルをロードする
    ///
    /// # Arguments
    /// * `dir_path` - 翻訳ファイルのディレクトリパス
    ///
    /// # Returns
    /// ロードされたファイル数、またはエラー
    ///
    /// # Errors
    /// ディレクトリの読み込みまたはファイルの解析に失敗した場合
    pub async fn load_directory(&self, dir_path: &Path) -> Result<usize, TranslationError> {
        info!("Loading translations from directory: {}", dir_path.display());

        let mut file_count = 0;

        for entry in WalkDir::new(dir_path) {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    warn!("Failed to read directory entry: {}", e);
                    continue;
                }
            };

            if !entry.file_type().is_file() {
                continue;
            }

            let path = entry.path();
            let Some(format) = TranslationFileFormat::from_path(path) else {
                continue;
            };

            match self.load_file(path, format).await {
                Ok(()) => {
                    file_count += 1;
                    debug!("Loaded translation file: {}", path.display());
                }
                Err(e) => {
                    error!("Failed to load translation file {}: {}", path.display(), e);
                }
            }
        }

        info!("Loaded {} translation files from {}", file_count, dir_path.display());
        Ok(file_count)
    }

    /// 単一の翻訳ファイルをロードする
    ///
    /// # Arguments
    /// * `file_path` - ファイルパス
    /// * `format` - ファイル形式
    ///
    /// # Returns
    /// 成功時は()、失敗時はエラー
    ///
    /// # Errors
    /// ファイルの読み込みまたは解析に失敗した場合
    pub async fn load_file(
        &self,
        file_path: &Path,
        format: TranslationFileFormat,
    ) -> Result<(), TranslationError> {
        let content = tokio::fs::read_to_string(file_path)
            .await
            .map_err(|e| TranslationError::FileRead { path: file_path.to_path_buf(), source: e })?;

        let translation_data: HashMap<String, TranslationValue> = match format {
            TranslationFileFormat::Json => serde_json::from_str(&content).map_err(|e| {
                TranslationError::JsonParse { path: file_path.to_path_buf(), source: e }
            })?,
            TranslationFileFormat::Yaml => serde_yaml::from_str(&content).map_err(|e| {
                TranslationError::YamlParse { path: file_path.to_path_buf(), source: e }
            })?,
        };

        let namespace = Self::extract_namespace_from_path(file_path);
        let namespace_key = namespace.clone().unwrap_or_else(|| "default".to_string());

        // キャッシュに翻訳データを保存
        let namespace_cache = self.cache.entry(namespace_key.clone()).or_default();
        Self::store_flattened_translations(&namespace_cache, &translation_data, "");

        // メタデータを保存
        let metadata = TranslationFileMetadata {
            path: file_path.to_path_buf(),
            last_modified: tokio::fs::metadata(file_path)
                .await
                .map_err(|e| TranslationError::FileRead {
                    path: file_path.to_path_buf(),
                    source: e,
                })?
                .modified()
                .unwrap_or(SystemTime::UNIX_EPOCH),
            format,
            namespace,
        };

        self.file_metadata.insert(file_path.to_path_buf(), metadata);

        debug!(
            "Loaded {} keys from {} (namespace: {})",
            namespace_cache.len(),
            file_path.display(),
            namespace_key
        );

        Ok(())
    }

    /// ネストした翻訳データを平坦化してキャッシュに保存する
    ///
    /// # Arguments
    /// * `cache` - 保存先のキャッシュ
    /// * `data` - 翻訳データ
    /// * `prefix` - キーのプレフィックス
    fn store_flattened_translations(
        cache: &DashMap<String, TranslationValue>,
        data: &HashMap<String, TranslationValue>,
        prefix: &str,
    ) {
        for (key, value) in data {
            let full_key = if prefix.is_empty() { key.clone() } else { format!("{prefix}.{key}") };

            match value {
                TranslationValue::Object(nested) => {
                    // ネストしたオブジェクトは再帰的に処理
                    Self::store_flattened_translations(cache, nested, &full_key);
                    // オブジェクト自体もキャッシュに保存（補完用）
                    cache.insert(full_key, value.clone());
                }
                _ => {
                    cache.insert(full_key, value.clone());
                }
            }
        }
    }

    /// ファイルパスからネームスペースを抽出する
    ///
    /// # Arguments
    /// * `file_path` - ファイルパス
    ///
    /// # Returns
    /// 抽出されたネームスペース（見つからない場合はNone）
    fn extract_namespace_from_path(file_path: &Path) -> Option<String> {
        file_path.file_stem()?.to_str().map(ToString::to_string)
    }

    /// 翻訳キーを検索する
    ///
    /// # Arguments
    /// * `key` - 検索するキー
    ///
    /// # Returns
    /// 見つかった翻訳値、または見つからない場合はNone
    pub fn get_translation(&self, key: &NamespacedKey) -> Option<TranslationValue> {
        let namespace = key.namespace.as_deref().unwrap_or("default");
        let namespace_cache = self.cache.get(namespace)?;
        namespace_cache.get(&key.key).map(|entry| entry.value().clone())
    }

    /// 指定されたプレフィックスで始まるキーを検索する（補完用）
    ///
    /// # Arguments
    /// * `namespace` - 検索対象のネームスペース
    /// * `prefix` - キーのプレフィックス
    /// * `limit` - 結果の最大数
    ///
    /// # Returns
    /// マッチしたキーのベクタ
    pub fn search_keys_with_prefix(
        &self,
        namespace: Option<&str>,
        prefix: &str,
        limit: usize,
    ) -> Vec<String> {
        let namespace = namespace.unwrap_or("default");

        let Some(namespace_cache) = self.cache.get(namespace) else {
            return Vec::new();
        };

        let mut results = Vec::new();

        for entry in namespace_cache.iter() {
            if results.len() >= limit {
                break;
            }

            let key = entry.key();
            if key.starts_with(prefix) {
                results.push(key.clone());
            }
        }

        results.sort();
        results
    }

    /// 指定されたネームスペースの全キーを取得する
    ///
    /// # Arguments
    /// * `namespace` - 対象のネームスペース
    ///
    /// # Returns
    /// キーのベクタ、またはネームスペースが見つからない場合はエラー
    ///
    /// # Errors
    /// 指定されたネームスペースが存在しない場合
    pub fn get_all_keys(&self, namespace: Option<&str>) -> Result<Vec<String>, TranslationError> {
        let namespace = namespace.unwrap_or("default");

        let Some(namespace_cache) = self.cache.get(namespace) else {
            return Err(TranslationError::NamespaceNotFound { namespace: namespace.to_string() });
        };

        let keys: Vec<String> = namespace_cache.iter().map(|entry| entry.key().clone()).collect();
        Ok(keys)
    }

    /// 利用可能なネームスペースの一覧を取得する
    ///
    /// # Returns
    /// ネームスペース名のベクタ
    pub fn get_namespaces(&self) -> Vec<String> {
        self.cache.iter().map(|entry| entry.key().clone()).collect()
    }

    /// ディレクトリの監視を開始する
    ///
    /// # Arguments
    /// * `dir_path` - 監視するディレクトリパス
    ///
    /// # Returns
    /// 成功時は()、失敗時はエラー
    ///
    /// # Errors
    /// ファイル監視の設定に失敗した場合
    pub fn start_watching(&self, dir_path: &Path) -> Result<(), TranslationError> {
        info!("Starting file watcher for directory: {}", dir_path.display());

        // 監視対象ディレクトリを追加
        {
            let mut watched_dirs = self.watched_dirs.write().map_err(|e| {
                error!("Failed to acquire write lock on watched_dirs: {}", e);
                TranslationError::InternalError { message: "Lock poisoned".to_string() }
            })?;
            if !watched_dirs.iter().any(|d| d == dir_path) {
                watched_dirs.push(dir_path.to_path_buf());
            }
        }

        // 既に監視が開始されている場合は何もしない
        if self.watcher_handle.read().map_or_else(
            |e| {
                error!("Failed to acquire read lock on watcher_handle: {}", e);
                false
            },
            |guard| guard.is_some(),
        ) {
            return Ok(());
        }

        let (tx, mut rx) = mpsc::unbounded_channel();
        let should_stop = Arc::clone(&self.should_stop_watching);

        // ファイル監視を設定
        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                if let Err(e) = tx.send(res) {
                    error!("Failed to send file event: {}", e);
                }
            },
            Config::default(),
        )
        .map_err(|e| TranslationError::FileWatch { path: dir_path.to_path_buf(), source: e })?;

        watcher
            .watch(dir_path, RecursiveMode::Recursive)
            .map_err(|e| TranslationError::FileWatch { path: dir_path.to_path_buf(), source: e })?;

        // 監視タスクを開始
        let cache_clone = self.cache.clone();
        let file_metadata_clone = self.file_metadata.clone();

        let handle = tokio::spawn(async move {
            // watcherを移動してタスク内で保持
            let _watcher = watcher;

            while let Some(event_result) = rx.recv().await {
                if should_stop.load(Ordering::Relaxed) {
                    break;
                }

                match event_result {
                    Ok(event) => {
                        Self::handle_file_event(event, &cache_clone, &file_metadata_clone).await;
                    }
                    Err(e) => {
                        error!("File watch error: {}", e);
                    }
                }
            }

            debug!("File watcher task terminated");
        });

        // ハンドルを保存
        if let Ok(mut watcher_handle) = self.watcher_handle.write() {
            *watcher_handle = Some(handle);
        } else {
            error!("Failed to acquire write lock on watcher_handle");
        }

        info!("File watcher started successfully");
        Ok(())
    }

    /// ファイル監視を停止する
    pub fn stop_watching(&self) {
        info!("Stopping file watcher");

        self.should_stop_watching.store(true, Ordering::Relaxed);

        if let Ok(mut handle_guard) = self.watcher_handle.write() {
            if let Some(handle) = handle_guard.take() {
                handle.abort();
                // joinを待たずに続行（abortなので）
            }
        } else {
            error!("Failed to acquire write lock on watcher_handle during stop");
        }

        debug!("File watcher stopped");
    }

    /// ファイル変更イベントを処理する
    ///
    /// # Arguments
    /// * `event` - ファイル変更イベント
    /// * `cache` - 翻訳キャッシュ
    /// * `file_metadata` - ファイルメタデータキャッシュ
    async fn handle_file_event(
        event: Event,
        cache: &DashMap<String, DashMap<String, TranslationValue>>,
        file_metadata: &DashMap<PathBuf, TranslationFileMetadata>,
    ) {
        if !matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_))
        {
            return;
        }

        for path in event.paths {
            let Some(format) = TranslationFileFormat::from_path(&path) else {
                continue;
            };

            match event.kind {
                EventKind::Create(_) | EventKind::Modify(_) => {
                    debug!("Translation file changed: {}", path.display());

                    // ファイルを再読み込み
                    if let Err(e) = Self::reload_file(&path, format, cache, file_metadata).await {
                        error!("Failed to reload translation file {}: {}", path.display(), e);
                    }
                }
                EventKind::Remove(_) => {
                    debug!("Translation file removed: {}", path.display());

                    // キャッシュからファイルのデータを削除
                    if let Some((_, metadata)) = file_metadata.remove(&path) {
                        if let Some(namespace) = metadata.namespace {
                            cache.remove(&namespace);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// 単一ファイルを再読み込みする
    ///
    /// # Arguments
    /// * `file_path` - ファイルパス
    /// * `format` - ファイル形式
    /// * `cache` - 翻訳キャッシュ
    /// * `file_metadata` - ファイルメタデータキャッシュ
    ///
    /// # Errors
    /// ファイルの読み込みまたは解析に失敗した場合
    async fn reload_file(
        file_path: &Path,
        format: TranslationFileFormat,
        cache: &DashMap<String, DashMap<String, TranslationValue>>,
        file_metadata: &DashMap<PathBuf, TranslationFileMetadata>,
    ) -> Result<(), TranslationError> {
        let content = tokio::fs::read_to_string(file_path)
            .await
            .map_err(|e| TranslationError::FileRead { path: file_path.to_path_buf(), source: e })?;

        let translation_data: HashMap<String, TranslationValue> = match format {
            TranslationFileFormat::Json => serde_json::from_str(&content).map_err(|e| {
                TranslationError::JsonParse { path: file_path.to_path_buf(), source: e }
            })?,
            TranslationFileFormat::Yaml => serde_yaml::from_str(&content).map_err(|e| {
                TranslationError::YamlParse { path: file_path.to_path_buf(), source: e }
            })?,
        };

        let namespace = Self::extract_namespace_from_path_static(file_path);
        let namespace_key = namespace.clone().unwrap_or_else(|| "default".to_string());

        // 既存のキャッシュをクリア
        if let Some(namespace_cache) = cache.get(&namespace_key) {
            namespace_cache.clear();
        }

        // 新しいデータでキャッシュを更新
        let namespace_cache = cache.entry(namespace_key).or_default();
        Self::store_flattened_translations_static(&namespace_cache, &translation_data, "");

        // メタデータを更新
        let metadata = TranslationFileMetadata {
            path: file_path.to_path_buf(),
            last_modified: tokio::fs::metadata(file_path)
                .await
                .map_err(|e| TranslationError::FileRead {
                    path: file_path.to_path_buf(),
                    source: e,
                })?
                .modified()
                .unwrap_or(SystemTime::UNIX_EPOCH),
            format,
            namespace,
        };

        file_metadata.insert(file_path.to_path_buf(), metadata);

        info!("Reloaded translation file: {}", file_path.display());
        Ok(())
    }

    /// ファイルパスからネームスペースを抽出する（静的メソッド版）
    ///
    /// # Arguments
    /// * `file_path` - ファイルパス
    ///
    /// # Returns
    /// 抽出されたネームスペース（見つからない場合はNone）
    fn extract_namespace_from_path_static(file_path: &Path) -> Option<String> {
        file_path.file_stem()?.to_str().map(ToString::to_string)
    }

    /// ネストした翻訳データを平坦化してキャッシュに保存する（静的メソッド版）
    ///
    /// # Arguments
    /// * `cache` - 保存先のキャッシュ
    /// * `data` - 翻訳データ
    /// * `prefix` - キーのプレフィックス
    fn store_flattened_translations_static(
        cache: &DashMap<String, TranslationValue>,
        data: &HashMap<String, TranslationValue>,
        prefix: &str,
    ) {
        for (key, value) in data {
            let full_key = if prefix.is_empty() { key.clone() } else { format!("{prefix}.{key}") };

            match value {
                TranslationValue::Object(nested) => {
                    // ネストしたオブジェクトは再帰的に処理
                    Self::store_flattened_translations_static(cache, nested, &full_key);
                    // オブジェクト自体もキャッシュに保存（補完用）
                    cache.insert(full_key, value.clone());
                }
                _ => {
                    cache.insert(full_key, value.clone());
                }
            }
        }
    }

    /// キャッシュをクリアする
    pub fn clear(&self) {
        self.cache.clear();
        self.file_metadata.clear();
        info!("Translation cache cleared");
    }

    /// 統計情報を取得する
    ///
    /// # Returns
    /// キャッシュ統計情報
    pub fn get_stats(&self) -> TranslationCacheStats {
        let mut total_keys = 0;
        let namespace_count = self.cache.len();

        for namespace_cache in &self.cache {
            total_keys += namespace_cache.value().len();
        }

        TranslationCacheStats { namespace_count, total_keys, file_count: self.file_metadata.len() }
    }
}

impl Drop for TranslationCache {
    fn drop(&mut self) {
        // 非同期処理なので、blocking呼び出しは使用しない
        self.should_stop_watching.store(true, Ordering::Relaxed);
    }
}

/// 翻訳キャッシュの統計情報
#[derive(Debug, Clone, Copy)]
pub struct TranslationCacheStats {
    /// ネームスペース数
    pub namespace_count: usize,
    /// 総キー数
    pub total_keys: usize,
    /// ファイル数
    pub file_count: usize,
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use std::collections::HashMap;

    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn test_translation_value_as_string() {
        let string_value = TranslationValue::String("test".to_string());
        assert_eq!(string_value.as_string(), Some("test"));

        let array_value = TranslationValue::Array(vec!["first".to_string(), "second".to_string()]);
        assert_eq!(array_value.as_string(), Some("first"));

        let object_value = TranslationValue::Object(HashMap::new());
        assert_eq!(object_value.as_string(), None);
    }

    #[test]
    fn test_translation_value_is_object() {
        let string_value = TranslationValue::String("test".to_string());
        assert!(!string_value.is_object());

        let object_value = TranslationValue::Object(HashMap::new());
        assert!(object_value.is_object());
    }

    #[test]
    fn test_translation_file_format_from_path() {
        let json_path = Path::new("test.json");
        assert_eq!(TranslationFileFormat::from_path(json_path), Some(TranslationFileFormat::Json));

        let yaml_path = Path::new("test.yaml");
        assert_eq!(TranslationFileFormat::from_path(yaml_path), Some(TranslationFileFormat::Yaml));

        let yml_path = Path::new("test.yml");
        assert_eq!(TranslationFileFormat::from_path(yml_path), Some(TranslationFileFormat::Yaml));

        let unknown_path = Path::new("test.txt");
        assert_eq!(TranslationFileFormat::from_path(unknown_path), None);
    }

    #[test]
    fn test_namespaced_key_parse() {
        let key = NamespacedKey::parse("common:hello");
        assert!(key.is_ok(), "Failed to parse namespaced key");
        let key = key.unwrap();
        assert_eq!(key.namespace, Some("common".to_string()));
        assert_eq!(key.key, "hello");
        assert_eq!(key.full_key(), "common:hello");

        let key_no_namespace = NamespacedKey::parse("hello");
        assert!(key_no_namespace.is_ok(), "Failed to parse key without namespace");
        let key_no_namespace = key_no_namespace.unwrap();
        assert_eq!(key_no_namespace.namespace, None);
        assert_eq!(key_no_namespace.key, "hello");
        assert_eq!(key_no_namespace.full_key(), "hello");

        // 無効なキーのテスト
        assert!(NamespacedKey::parse("").is_err());
        assert!(NamespacedKey::parse(":hello").is_err());
        assert!(NamespacedKey::parse("common:").is_err());
    }

    #[test]
    fn test_translation_cache_basic_operations() {
        let cache = TranslationCache::new();

        // テスト用の翻訳データを準備
        let mut test_data = HashMap::new();
        test_data
            .insert("hello".to_string(), TranslationValue::String("Hello, World!".to_string()));

        let mut nested_data = HashMap::new();
        nested_data
            .insert("greeting".to_string(), TranslationValue::String("Good morning".to_string()));
        test_data.insert("morning".to_string(), TranslationValue::Object(nested_data));

        // データをキャッシュに保存（Direct manipulation to avoid self-referencing calls）
        {
            let namespace_cache = cache.cache.entry("test".to_string()).or_default();
            TranslationCache::store_flattened_translations_static(&namespace_cache, &test_data, "");
        }

        // データを取得してテスト
        let key = NamespacedKey::new(Some("test".to_string()), "hello".to_string());
        let value = cache.get_translation(&key);
        assert!(value.is_some(), "Translation should exist");
        let value = value.unwrap();
        assert_eq!(value.as_string(), Some("Hello, World!"));

        // ネストしたキーのテスト
        let nested_key =
            NamespacedKey::new(Some("test".to_string()), "morning.greeting".to_string());
        let nested_value = cache.get_translation(&nested_key);
        assert!(nested_value.is_some(), "Nested translation should exist");
        let nested_value = nested_value.unwrap();
        assert_eq!(nested_value.as_string(), Some("Good morning"));

        // プレフィックス検索のテスト
        let results = cache.search_keys_with_prefix(Some("test"), "morn", 10);
        assert!(results.contains(&"morning".to_string()));
        assert!(results.contains(&"morning.greeting".to_string()));
    }

    #[test]
    fn test_translation_cache_stats() {
        let cache = TranslationCache::new();

        // 統計情報の初期状態をテスト
        let stats = cache.get_stats();
        assert_eq!(stats.namespace_count, 0);
        assert_eq!(stats.total_keys, 0);
        assert_eq!(stats.file_count, 0);

        // データを追加
        {
            let namespace_cache = cache.cache.entry("test".to_string()).or_default();
            namespace_cache
                .insert("key1".to_string(), TranslationValue::String("value1".to_string()));
            namespace_cache
                .insert("key2".to_string(), TranslationValue::String("value2".to_string()));
        }

        // 統計情報を再取得してテスト
        let stats = cache.get_stats();
        assert_eq!(stats.namespace_count, 1);
        assert_eq!(stats.total_keys, 2);
    }

    #[test]
    fn test_translation_cache_clear() {
        let cache = TranslationCache::new();

        // データを追加
        {
            let namespace_cache = cache.cache.entry("test".to_string()).or_default();
            namespace_cache
                .insert("key1".to_string(), TranslationValue::String("value1".to_string()));
        }

        // データが存在することを確認
        assert_eq!(cache.get_stats().total_keys, 1);

        // クリア
        cache.clear();

        // データがクリアされたことを確認
        assert_eq!(cache.get_stats().total_keys, 0);
        assert_eq!(cache.get_stats().namespace_count, 0);
    }

    #[test]
    fn test_namespaced_key_new() {
        let key = NamespacedKey::new(Some("ns".to_string()), "key".to_string());
        assert_eq!(key.namespace, Some("ns".to_string()));
        assert_eq!(key.key, "key");

        let key_no_ns = NamespacedKey::new(None, "key".to_string());
        assert_eq!(key_no_ns.namespace, None);
        assert_eq!(key_no_ns.key, "key");
    }

    #[test]
    fn test_translation_value_get_keys() {
        let mut obj = HashMap::new();
        obj.insert("key1".to_string(), TranslationValue::String("value1".to_string()));
        obj.insert("key2".to_string(), TranslationValue::String("value2".to_string()));

        let object_value = TranslationValue::Object(obj);
        let mut keys = object_value.get_keys();
        keys.sort();
        assert_eq!(keys, vec!["key1", "key2"]);

        let string_value = TranslationValue::String("test".to_string());
        assert_eq!(string_value.get_keys(), Vec::<String>::new());
    }
}
