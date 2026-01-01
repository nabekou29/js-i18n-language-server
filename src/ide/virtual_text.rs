//! Virtual Text（翻訳置換表示）の実装
//!
//! 翻訳キーの位置と翻訳値をエディタ拡張向けに提供する。

use serde::{
    Deserialize,
    Serialize,
};
use tower_lsp::lsp_types::Range;

use crate::db::I18nDatabase;
use crate::input::source::SourceFile;
use crate::input::translation::Translation;

/// ドキュメント内の翻訳装飾情報
///
/// エディタ拡張がこの情報を使用して、キー文字列を翻訳値で置換表示する。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationDecoration {
    /// キー文字列の位置（例: `'common.hello'` の範囲、クォートを含む）
    pub range: Range,
    /// 翻訳キー
    pub key: String,
    /// 翻訳値（切り詰め済み）
    pub value: String,
}

/// ドキュメントの翻訳装飾情報を生成
///
/// # Arguments
/// * `db` - Salsa データベース
/// * `source_file` - ソースファイル
/// * `translations` - 翻訳データ
/// * `language` - 表示する言語（None の場合は最初に見つかった翻訳を使用）
/// * `max_length` - 最大表示文字数（超過時は省略記号を追加）
#[must_use]
pub fn get_translation_decorations(
    db: &dyn I18nDatabase,
    source_file: SourceFile,
    translations: &[Translation],
    language: Option<&str>,
    max_length: usize,
) -> Vec<TranslationDecoration> {
    // ソースファイルからキー使用箇所を取得
    let key_usages = crate::syntax::analyze_source(db, source_file);

    let mut decorations = Vec::new();

    for usage in key_usages {
        let key = usage.key(db);
        let key_text = key.text(db);
        let range: Range = usage.range(db).into();

        // 指定された言語の翻訳値を取得
        let value = get_translation_value(db, translations, key_text, language);

        if let Some(value) = value {
            let truncated_value = truncate_value(&value, max_length);
            decorations.push(TranslationDecoration {
                range,
                key: key_text.clone(),
                value: truncated_value,
            });
        }
    }

    decorations
}

/// 翻訳値を取得
///
/// 指定された言語の翻訳値を返す。言語が指定されていない場合は最初に見つかった翻訳を返す。
fn get_translation_value(
    db: &dyn I18nDatabase,
    translations: &[Translation],
    key_text: &str,
    language: Option<&str>,
) -> Option<String> {
    if let Some(lang) = language {
        // 指定された言語の翻訳を検索
        for translation in translations {
            if translation.language(db) == lang
                && let Some(value) = translation.keys(db).get(key_text)
            {
                return Some(value.clone());
            }
        }
    } else {
        // 言語未指定の場合は最初に見つかった翻訳を返す
        for translation in translations {
            if let Some(value) = translation.keys(db).get(key_text) {
                return Some(value.clone());
            }
        }
    }
    None
}

/// 翻訳値を切り詰め
///
/// `max_length` を超える場合は省略記号を追加して切り詰める。
fn truncate_value(value: &str, max_length: usize) -> String {
    let char_count = value.chars().count();
    if char_count <= max_length {
        value.to_string()
    } else {
        let truncated: String = value.chars().take(max_length.saturating_sub(1)).collect();
        format!("{truncated}…")
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::collections::HashMap;

    use googletest::prelude::*;
    use rstest::*;

    use super::*;
    use crate::db::I18nDatabaseImpl;
    use crate::input::source::ProgrammingLanguage;

    /// テスト用の Translation を作成するヘルパー関数
    fn create_translation(
        db: &I18nDatabaseImpl,
        language: &str,
        file_path: &str,
        keys: HashMap<String, String>,
    ) -> Translation {
        Translation::new(
            db,
            language.to_string(),
            file_path.to_string(),
            keys,
            "{}".to_string(),
            HashMap::new(),
            HashMap::new(),
        )
    }

    /// テスト用の SourceFile を作成するヘルパー関数
    fn create_source_file(db: &I18nDatabaseImpl, content: &str) -> SourceFile {
        SourceFile::new(
            db,
            "file:///test/app.tsx".to_string(),
            content.to_string(),
            ProgrammingLanguage::Tsx,
        )
    }

    #[rstest]
    fn truncate_value_short_text() {
        let result = truncate_value("Hello", 30);
        assert_that!(result, eq("Hello"));
    }

    #[rstest]
    fn truncate_value_exact_length() {
        let result = truncate_value("Hello World", 11);
        assert_that!(result, eq("Hello World"));
    }

    #[rstest]
    fn truncate_value_long_text() {
        let result = truncate_value("This is a very long message that should be truncated", 20);
        assert_that!(result, eq("This is a very long…"));
    }

    #[rstest]
    fn truncate_value_japanese_text() {
        // 日本語のマルチバイト文字も正しく処理されることを確認
        // 入力: 12文字, max_length: 10 → 9文字 + 省略記号
        let result = truncate_value("これは長いメッセージです", 10);
        assert_that!(result, eq("これは長いメッセー…"));
    }

    #[rstest]
    fn get_decorations_basic() {
        let db = I18nDatabaseImpl::default();

        let source_file = create_source_file(&db, r#"const msg = t("common.hello");"#);

        let translation = create_translation(
            &db,
            "ja",
            "/test/locales/ja.json",
            HashMap::from([("common.hello".to_string(), "こんにちは".to_string())]),
        );

        let decorations =
            get_translation_decorations(&db, source_file, &[translation], Some("ja"), 30);

        assert_that!(decorations, len(eq(1)));
        assert_that!(decorations[0].key, eq("common.hello"));
        assert_that!(decorations[0].value, eq("こんにちは"));
    }

    #[rstest]
    fn get_decorations_with_truncation() {
        let db = I18nDatabaseImpl::default();

        let source_file = create_source_file(&db, r#"const msg = t("common.hello");"#);

        let translation = create_translation(
            &db,
            "ja",
            "/test/locales/ja.json",
            HashMap::from([(
                "common.hello".to_string(),
                "これは非常に長いメッセージで切り詰める必要があります".to_string(),
            )]),
        );

        let decorations =
            get_translation_decorations(&db, source_file, &[translation], Some("ja"), 10);

        assert_that!(decorations, len(eq(1)));
        assert_that!(decorations[0].value, eq("これは非常に長いメ…"));
    }

    #[rstest]
    fn get_decorations_no_language_match() {
        let db = I18nDatabaseImpl::default();

        let source_file = create_source_file(&db, r#"const msg = t("common.hello");"#);

        let translation = create_translation(
            &db,
            "en",
            "/test/locales/en.json",
            HashMap::from([("common.hello".to_string(), "Hello".to_string())]),
        );

        // 存在しない言語を指定
        let decorations =
            get_translation_decorations(&db, source_file, &[translation], Some("fr"), 30);

        assert_that!(decorations, is_empty());
    }

    #[rstest]
    fn get_decorations_no_language_specified() {
        let db = I18nDatabaseImpl::default();

        let source_file = create_source_file(&db, r#"const msg = t("common.hello");"#);

        let translation = create_translation(
            &db,
            "en",
            "/test/locales/en.json",
            HashMap::from([("common.hello".to_string(), "Hello".to_string())]),
        );

        // 言語未指定の場合は最初に見つかった翻訳を使用
        let decorations = get_translation_decorations(&db, source_file, &[translation], None, 30);

        assert_that!(decorations, len(eq(1)));
        assert_that!(decorations[0].value, eq("Hello"));
    }
}
