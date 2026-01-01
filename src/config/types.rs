use serde::{
    Deserialize,
    Serialize,
};
use thiserror::Error;

/// 設定のバリデーションエラー
#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Configuration error in '{field_path}': {message}")]
pub struct ValidationError {
    /// エラーが発生したフィールドのJSONパス（例: "includePatterns[0]"）
    pub field_path: String,
    /// エラーメッセージ
    pub message: String,
}

impl ValidationError {
    /// 新しいバリデーションエラーを作成
    #[must_use]
    pub fn new(field_path: impl Into<String>, message: impl Into<String>) -> Self {
        Self { field_path: field_path.into(), message: message.into() }
    }
}

/// 設定管理に関するエラー
#[derive(Error, Debug)]
pub enum ConfigError {
    /// バリデーションエラー（複数のエラーを含む）
    #[error("Configuration validation failed:\n{}", format_validation_errors(.0))]
    ValidationErrors(Vec<ValidationError>),

    /// ファイル読み込みエラー
    #[error("Failed to load configuration file: {0}")]
    IoError(#[from] std::io::Error),

    /// JSON パースエラー
    #[error("Failed to parse configuration: {0}")]
    ParseError(#[from] serde_json::Error),
}

/// `ValidationError` のリストを読みやすい文字列に整形
fn format_validation_errors(errors: &[ValidationError]) -> String {
    errors
        .iter()
        .enumerate()
        .map(|(i, err)| format!("  {}. {} - {}", i + 1, err.field_path, err.message))
        .collect::<Vec<_>>()
        .join("\n")
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerSettings {
    pub js_i18n: I18nSettings,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct I18nSettings {
    /// 翻訳ファイル
    pub translation_files: TranslationFilesConfig,

    /// ソースコードとして含むパターン
    pub include_patterns: Vec<String>,
    /// ソースコードから除外するパターン
    pub exclude_patterns: Vec<String>,

    /// キーの区切り文字（デフォルト: "."）
    pub key_separator: String,
    /// ネームスペースの区切り文字
    pub namespace_separator: Option<String>,

    /// インデックス設定
    pub indexing: IndexingConfig,

    /// 翻訳が必須の言語
    ///
    /// - `None`: 翻訳ファイルから検出されたすべての言語が必須（デフォルト）
    /// - `Some([...])`: 指定された言語のみが必須
    ///
    /// 必須言語で翻訳が欠けている場合は警告が表示されます。
    /// `optional_languages` と同時に指定することはできません。
    pub required_languages: Option<Vec<String>>,

    /// 翻訳が任意の言語
    ///
    /// この言語で翻訳が欠けていても診断は表示されません。
    /// `required_languages` と同時に指定することはできません。
    pub optional_languages: Option<Vec<String>>,

    /// Virtual Text（翻訳置換表示）の設定
    pub virtual_text: VirtualTextConfig,

    /// 診断（Diagnostics）の設定
    pub diagnostics: DiagnosticsConfig,
}

/// インデックス処理の設定
#[derive(Debug, Clone, Copy, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct IndexingConfig {
    /// インデックス処理の並列スレッド数
    ///
    /// - `None`: デフォルト（CPUコア数の80%、最低1スレッド）
    /// - `Some(n)`: 指定されたスレッド数を使用
    pub num_threads: Option<usize>,
}

/// Virtual Text（翻訳置換表示）の設定
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct VirtualTextConfig {
    /// 最大表示文字数（超過時は省略記号を追加）
    pub max_length: usize,
}

impl Default for VirtualTextConfig {
    fn default() -> Self {
        Self { max_length: 30 }
    }
}

/// 診断（Diagnostics）の設定
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DiagnosticsConfig {
    /// 未使用キーの警告を有効にするか（デフォルト: true）
    pub unused_keys: bool,
}

impl Default for DiagnosticsConfig {
    fn default() -> Self {
        Self { unused_keys: true }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TranslationFilesConfig {
    pub file_pattern: String,
}

impl I18nSettings {
    /// 設定をバリデーションする
    ///
    /// # Errors
    /// - 必須フィールドが空の場合
    /// - glob パターンが不正な場合
    /// - 区切り文字が無効な場合
    pub fn validate(&self) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();

        // key_separator のチェック
        if self.key_separator.is_empty() {
            errors.push(ValidationError::new(
                "keySeparator",
                "The separator cannot be empty. Please specify a separator, for example: \".\" (dot)",
            ));
        }

        // namespace_separator のチェック
        if let Some(sep) = &self.namespace_separator
            && sep.is_empty()
        {
            errors.push(ValidationError::new(
                    "namespaceSeparator",
                    "The separator cannot be empty. Please specify a separator (e.g., \":\"), or remove this field",
                ));
        }

        // include_patterns のチェック
        if self.include_patterns.is_empty() {
            errors.push(ValidationError::new(
                "includePatterns",
                "At least one pattern is required. Example: [\"**/*.{js,ts,tsx}\"]",
            ));
        }

        // include_patterns の glob パターン妥当性チェック
        for (index, pattern) in self.include_patterns.iter().enumerate() {
            if let Err(e) = globset::Glob::new(pattern) {
                errors.push(ValidationError::new(
                    format!("includePatterns[{index}]"),
                    format!("Invalid glob pattern '{pattern}': {e}"),
                ));
            }
        }

        // exclude_patterns の glob パターン妥当性チェック
        for (index, pattern) in self.exclude_patterns.iter().enumerate() {
            if let Err(e) = globset::Glob::new(pattern) {
                errors.push(ValidationError::new(
                    format!("excludePatterns[{index}]"),
                    format!("Invalid glob pattern '{pattern}': {e}"),
                ));
            }
        }

        // translation_files.file_pattern のバリデーション
        if self.translation_files.file_pattern.is_empty() {
            errors.push(ValidationError::new(
                "translationFiles.filePattern",
                "The pattern cannot be empty. Example: \"**/{locales,messages}/**/*.json\"",
            ));
        } else if let Err(e) = globset::Glob::new(&self.translation_files.file_pattern) {
            errors.push(ValidationError::new(
                "translationFiles.filePattern",
                format!("Invalid glob pattern '{}': {e}", self.translation_files.file_pattern),
            ));
        }

        // required_languages と optional_languages の排他チェック
        if self.required_languages.is_some() && self.optional_languages.is_some() {
            errors.push(ValidationError::new(
                "requiredLanguages/optionalLanguages",
                "Cannot specify both 'requiredLanguages' and 'optionalLanguages'. Please use only one",
            ));
        }

        if errors.is_empty() { Ok(()) } else { Err(errors) }
    }
}

impl Default for TranslationFilesConfig {
    fn default() -> Self {
        Self { file_pattern: "**/{locales,messages}/**/*.json".to_string() }
    }
}

impl Default for I18nSettings {
    fn default() -> Self {
        Self {
            translation_files: TranslationFilesConfig::default(),
            include_patterns: vec!["**/*.{js,jsx,ts,tsx}".to_string()],
            exclude_patterns: vec!["node_modules/**".to_string()],
            key_separator: ".".to_string(),
            namespace_separator: None,
            indexing: IndexingConfig::default(),
            required_languages: None,
            optional_languages: None,
            virtual_text: VirtualTextConfig::default(),
            diagnostics: DiagnosticsConfig::default(),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::expect_used, clippy::panic)]
mod tests {
    use googletest::prelude::*;
    use rstest::*;

    use super::*;

    #[rstest]
    fn validate_valid_settings() {
        let settings = I18nSettings::default();

        assert_that!(settings.validate(), ok(anything())); // Result型のOkチェックに最適
    }

    #[rstest]
    fn deserialize_partial_settings() {
        // 一部のフィールドのみを持つ JSON
        let json = r#"{"namespaceSeparator": ":"}"#;

        let settings: I18nSettings = serde_json::from_str(json).unwrap();

        // 省略されたフィールドはデフォルト値になる
        assert_that!(settings.key_separator, eq("."));
        assert_that!(settings.include_patterns, len(eq(1)));
        assert_that!(settings.namespace_separator, some(eq(":")));
    }

    #[rstest]
    fn deserialize_empty_settings() {
        // 空の JSON オブジェクト
        let json = "{}";

        let settings: I18nSettings = serde_json::from_str(json).unwrap();

        // 全てのフィールドがデフォルト値になる
        assert_that!(settings.key_separator, eq("."));
        assert_that!(settings.include_patterns, elements_are![eq("**/*.{js,jsx,ts,tsx}")]);
        assert_that!(settings.exclude_patterns, elements_are![eq("node_modules/**")]);
        assert_that!(
            settings.translation_files.file_pattern,
            eq("**/{locales,messages}/**/*.json")
        );
    }

    #[rstest]
    fn validate_invalid_key_separator_empty() {
        let settings = I18nSettings { key_separator: String::new(), ..I18nSettings::default() };
        let result = settings.validate();

        assert_that!(
            result,
            err(elements_are![all![
                field!(ValidationError.field_path, eq("keySeparator")),
                field!(ValidationError.message, contains_substring("cannot be empty"))
            ]])
        );
    }

    #[rstest]
    fn validate_invalid_namespace_separator_empty() {
        let settings =
            I18nSettings { namespace_separator: Some(String::new()), ..I18nSettings::default() };
        let result = settings.validate();

        assert_that!(
            result,
            err(elements_are![all![
                field!(ValidationError.field_path, eq("namespaceSeparator")),
                field!(ValidationError.message, contains_substring("cannot be empty"))
            ]])
        );
    }

    #[rstest]
    fn validate_invalid_include_patterns_empty() {
        let settings = I18nSettings {
            include_patterns: vec![], // 空のパターン
            ..I18nSettings::default()
        };
        let result = settings.validate();

        assert_that!(
            result,
            err(elements_are![all![
                field!(ValidationError.field_path, eq("includePatterns")),
                field!(ValidationError.message, contains_substring("At least one pattern"))
            ]])
        );
    }

    #[rstest]
    fn validate_invalid_include_pattern_invalid_glob() {
        let settings = I18nSettings {
            include_patterns: vec!["**/*.{js,ts".to_string()], // 不正なパターン
            ..I18nSettings::default()
        };

        let result = settings.validate();

        assert_that!(
            result,
            err(elements_are![all![
                field!(ValidationError.field_path, eq("includePatterns[0]")),
                field!(ValidationError.message, contains_substring("Invalid glob pattern")),
                field!(ValidationError.message, contains_substring("**/*.{js,ts"))
            ]])
        );
    }

    #[rstest]
    fn validate_invalid_exclude_pattern_invalid_glob() {
        let settings = I18nSettings {
            exclude_patterns: vec![
                "node_modules/**".to_string(),
                "dist/**".to_string(),
                "invalid[pattern".to_string(),
            ], // 不正なパターン
            ..I18nSettings::default()
        };

        let result = settings.validate();

        assert_that!(
            result,
            err(elements_are![all![
                field!(ValidationError.field_path, eq("excludePatterns[2]")),
                field!(ValidationError.message, contains_substring("Invalid glob pattern")),
                field!(ValidationError.message, contains_substring("invalid[pattern"))
            ]])
        );
    }

    #[rstest]
    fn validate_invalid_translation_file_pattern_empty() {
        let settings = I18nSettings {
            translation_files: TranslationFilesConfig { file_pattern: String::new() },
            ..I18nSettings::default()
        };

        let result = settings.validate();

        assert_that!(
            result,
            err(elements_are![all![
                field!(ValidationError.field_path, eq("translationFiles.filePattern")),
                field!(ValidationError.message, contains_substring("cannot be empty"))
            ]])
        );
    }

    #[rstest]
    fn validate_invalid_translation_file_pattern_invalid_glob() {
        let settings = I18nSettings {
            translation_files: TranslationFilesConfig {
                file_pattern: "**/{locales,messages/*.json".to_string(), // 不正なパターン
            },

            ..I18nSettings::default()
        };

        let result = settings.validate();

        assert_that!(
            result,
            err(elements_are![all![
                field!(ValidationError.field_path, eq("translationFiles.filePattern")),
                field!(ValidationError.message, contains_substring("Invalid glob pattern"))
            ]])
        );
    }

    #[rstest]
    fn config_error_validation_errors_format() {
        // 複数のバリデーションエラーを持つ設定
        let settings = I18nSettings {
            key_separator: String::new(), // エラー1
            include_patterns: vec![],     // エラー2
            ..I18nSettings::default()
        };

        let validation_result = settings.validate();
        let errors = validation_result.unwrap_err();
        let config_error = ConfigError::ValidationErrors(errors);

        let error_message = format!("{config_error}");
        // エラーメッセージに詳細が含まれていることを確認
        assert_that!(error_message, contains_substring("Configuration validation failed"));
        assert_that!(error_message, contains_substring("1. keySeparator"));
        assert_that!(error_message, contains_substring("cannot be empty"));
        assert_that!(error_message, contains_substring("2. includePatterns"));
        assert_that!(error_message, contains_substring("At least one pattern"));
    }
}
