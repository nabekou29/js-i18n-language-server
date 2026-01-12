use serde::{
    Deserialize,
    Serialize,
};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Configuration error in '{field_path}': {message}")]
pub struct ValidationError {
    /// JSON path to the field (e.g., "includePatterns[0]")
    pub field_path: String,
    pub message: String,
}

impl ValidationError {
    #[must_use]
    pub fn new(field_path: impl Into<String>, message: impl Into<String>) -> Self {
        Self { field_path: field_path.into(), message: message.into() }
    }
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Configuration validation failed:\n{}", format_validation_errors(.0))]
    ValidationErrors(Vec<ValidationError>),

    #[error("Failed to load configuration file: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Failed to parse configuration: {0}")]
    ParseError(#[from] serde_json::Error),
}

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
    pub translation_files: TranslationFilesConfig,

    pub include_patterns: Vec<String>,
    pub exclude_patterns: Vec<String>,

    pub key_separator: String,
    pub namespace_separator: Option<String>,
    /// Used when no explicit namespace is specified in code.
    /// If unset, searches all translation files (backward compatibility).
    pub default_namespace: Option<String>,

    pub indexing: IndexingConfig,

    /// Languages that require translations.
    ///
    /// - `None`: All detected languages are required (default)
    /// - `Some([...])`: Only specified languages are required
    ///
    /// Mutually exclusive with `optional_languages`.
    pub required_languages: Option<Vec<String>>,

    /// Languages where missing translations are ignored.
    ///
    /// Mutually exclusive with `required_languages`.
    pub optional_languages: Option<Vec<String>>,

    pub virtual_text: VirtualTextConfig,
    pub diagnostics: DiagnosticsConfig,

    /// Fallback language priority when `currentLanguage` is unset.
    pub primary_languages: Option<Vec<String>>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct IndexingConfig {
    /// Parallel thread count for indexing.
    /// Default: 80% of CPU cores (minimum 1).
    pub num_threads: Option<usize>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct VirtualTextConfig {
    /// Max characters before truncation with ellipsis.
    pub max_length: usize,
}

impl Default for VirtualTextConfig {
    fn default() -> Self {
        Self { max_length: 30 }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DiagnosticsConfig {
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
    /// # Errors
    /// - Required field is empty
    /// - Invalid glob pattern
    /// - Invalid separator
    pub fn validate(&self) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();

        if self.key_separator.is_empty() {
            errors.push(ValidationError::new(
                "keySeparator",
                "The separator cannot be empty. Please specify a separator, for example: \".\" (dot)",
            ));
        }

        if let Some(sep) = &self.namespace_separator
            && sep.is_empty()
        {
            errors.push(ValidationError::new(
                    "namespaceSeparator",
                    "The separator cannot be empty. Please specify a separator (e.g., \":\"), or remove this field",
                ));
        }

        if self.include_patterns.is_empty() {
            errors.push(ValidationError::new(
                "includePatterns",
                "At least one pattern is required. Example: [\"**/*.{js,ts,tsx}\"]",
            ));
        }

        for (index, pattern) in self.include_patterns.iter().enumerate() {
            if let Err(e) = globset::Glob::new(pattern) {
                errors.push(ValidationError::new(
                    format!("includePatterns[{index}]"),
                    format!("Invalid glob pattern '{pattern}': {e}"),
                ));
            }
        }

        for (index, pattern) in self.exclude_patterns.iter().enumerate() {
            if let Err(e) = globset::Glob::new(pattern) {
                errors.push(ValidationError::new(
                    format!("excludePatterns[{index}]"),
                    format!("Invalid glob pattern '{pattern}': {e}"),
                ));
            }
        }

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
            default_namespace: None,
            indexing: IndexingConfig::default(),
            required_languages: None,
            optional_languages: None,
            virtual_text: VirtualTextConfig::default(),
            diagnostics: DiagnosticsConfig::default(),
            primary_languages: None,
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

        assert_that!(settings.validate(), ok(anything()));
    }

    #[rstest]
    fn deserialize_partial_settings() {
        let json = r#"{"namespaceSeparator": ":"}"#;

        let settings: I18nSettings = serde_json::from_str(json).unwrap();

        assert_that!(settings.key_separator, eq("."));
        assert_that!(settings.include_patterns, len(eq(1)));
        assert_that!(settings.namespace_separator, some(eq(":")));
    }

    #[rstest]
    fn deserialize_empty_settings() {
        let json = "{}";

        let settings: I18nSettings = serde_json::from_str(json).unwrap();

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
        let settings = I18nSettings { include_patterns: vec![], ..I18nSettings::default() };
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
            include_patterns: vec!["**/*.{js,ts".to_string()],
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
            ],
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
                file_pattern: "**/{locales,messages/*.json".to_string(),
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
        let settings = I18nSettings {
            key_separator: String::new(),
            include_patterns: vec![],
            ..I18nSettings::default()
        };

        let validation_result = settings.validate();
        let errors = validation_result.unwrap_err();
        let config_error = ConfigError::ValidationErrors(errors);

        let error_message = format!("{config_error}");
        assert_that!(error_message, contains_substring("Configuration validation failed"));
        assert_that!(error_message, contains_substring("1. keySeparator"));
        assert_that!(error_message, contains_substring("cannot be empty"));
        assert_that!(error_message, contains_substring("2. includePatterns"));
        assert_that!(error_message, contains_substring("At least one pattern"));
    }
}
