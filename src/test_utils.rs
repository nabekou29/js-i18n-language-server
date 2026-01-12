//! テスト用ユーティリティ関数
//!
//! 複数のテストモジュールで使用される共通のヘルパー関数を提供します。

use std::collections::HashMap;

use crate::db::I18nDatabaseImpl;
use crate::input::translation::Translation;

/// テスト用の Translation を作成する
///
/// # Arguments
/// * `db` - Salsa データベース
/// * `language` - 言語コード（例: "en", "ja"）
/// * `file_path` - 翻訳ファイルのパス
/// * `keys` - キーと値のマップ
///
/// # Returns
/// 作成された Translation
#[allow(clippy::redundant_pub_crate)]
pub(crate) fn create_translation(
    db: &I18nDatabaseImpl,
    language: &str,
    file_path: &str,
    keys: HashMap<String, String>,
) -> Translation {
    create_translation_with_namespace(db, language, None, file_path, keys)
}

/// テスト用の Translation を namespace 指定で作成する
#[allow(clippy::redundant_pub_crate)]
pub(crate) fn create_translation_with_namespace(
    db: &I18nDatabaseImpl,
    language: &str,
    namespace: Option<&str>,
    file_path: &str,
    keys: HashMap<String, String>,
) -> Translation {
    Translation::new(
        db,
        language.to_string(),
        namespace.map(String::from),
        file_path.to_string(),
        keys,
        "{}".to_string(),
        HashMap::new(),
        HashMap::new(),
    )
}
