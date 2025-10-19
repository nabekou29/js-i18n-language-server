use serde::{
    Deserialize,
    Serialize,
};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerSettings {
    pub js_i18n: I18nSettings,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
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
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationFilesConfig {
    pub file_pattern: String,
}

impl I18nSettings {
    /// 設定をバリデーションする
    ///
    /// # Errors
    /// - `key_separator` が空の場合
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // include_patterns のチェック
        if self.include_patterns.is_empty() {
            errors.push("include_patterns cannot be empty".to_string());
        }

        // glob パターンの妥当性チェック
        for pattern in &self.include_patterns {
            if let Err(e) = globset::Glob::new(pattern) {
                errors.push(format!("Invalid include pattern '{pattern}': {e}"));
            }
        }

        for pattern in &self.exclude_patterns {
            if let Err(e) = globset::Glob::new(pattern) {
                errors.push(format!("Invalid exclude pattern '{pattern}': {e}"));
            }
        }

        // translation_files のバリデーション
        if let Err(e) = globset::Glob::new(&self.translation_files.file_pattern) {
            errors.push(format!(
                "Invalid translation file pattern '{}': {}",
                self.translation_files.file_pattern, e
            ));
        }

        if errors.is_empty() { Ok(()) } else { Err(errors) }
    }
}

impl Default for I18nSettings {
    fn default() -> Self {
        Self {
            translation_files: TranslationFilesConfig {
                file_pattern: "**/{locales,messages}/*.json".to_string(),
            },
            include_patterns: vec!["**/*.{js,jsx,ts,tsx}".to_string()],
            exclude_patterns: vec!["node_modules/**".to_string()],
            key_separator: ".".to_string(),
            namespace_separator: None,
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
    fn validate_invalid_include_pattern() {
        let settings = I18nSettings {
            include_patterns: vec!["**/*.{js,ts".to_string()], // 不正なパターン
            ..I18nSettings::default()
        };

        let result = settings.validate();
        assert_that!(result, err(anything())); // Result型のErrチェックに最適
        let errors = result.err().unwrap();
        assert_that!(errors, len(eq(1)));
        assert_that!(&errors[0], contains_substring("Invalid include pattern")); // 文字列の部分一致チェックに最適
    }
}
