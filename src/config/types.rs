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

    pub diagnostics: DiagnosticsConfig,

    /// Fallback language priority when `currentLanguage` is unset.
    pub primary_languages: Option<Vec<String>>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct IndexingConfig {
    /// Parallel thread count for indexing.
    /// Default: 40% of CPU cores (minimum 1).
    pub num_threads: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Severity {
    Error,
    Warning,
    Information,
    Hint,
}

impl Severity {
    /// Convert to LSP `DiagnosticSeverity`.
    #[must_use]
    pub const fn to_lsp(self) -> tower_lsp::lsp_types::DiagnosticSeverity {
        match self {
            Self::Error => tower_lsp::lsp_types::DiagnosticSeverity::ERROR,
            Self::Warning => tower_lsp::lsp_types::DiagnosticSeverity::WARNING,
            Self::Information => tower_lsp::lsp_types::DiagnosticSeverity::INFORMATION,
            Self::Hint => tower_lsp::lsp_types::DiagnosticSeverity::HINT,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MissingTranslationConfig {
    pub enabled: bool,
    pub severity: Severity,
    /// Only check these languages. `None` = all detected languages.
    /// Mutually exclusive with `optional_languages`.
    pub required_languages: Option<Vec<String>>,
    /// Skip these languages. Mutually exclusive with `required_languages`.
    pub optional_languages: Option<Vec<String>>,
}

impl Default for MissingTranslationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            severity: Severity::Warning,
            required_languages: None,
            optional_languages: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UnusedTranslationConfig {
    pub enabled: bool,
    pub severity: Severity,
    /// Glob patterns for keys to exclude from unused diagnostics.
    pub ignore_patterns: Vec<String>,
}

impl Default for UnusedTranslationConfig {
    fn default() -> Self {
        Self { enabled: true, severity: Severity::Hint, ignore_patterns: Vec::new() }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
#[derive(Default)]
pub struct DiagnosticsConfig {
    pub missing_translation: MissingTranslationConfig,
    pub unused_translation: UnusedTranslationConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TranslationFilesConfig {
    pub include_patterns: Vec<String>,
    pub exclude_patterns: Vec<String>,
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

        validate_glob_patterns(&self.include_patterns, "includePatterns", &mut errors);
        validate_glob_patterns(&self.exclude_patterns, "excludePatterns", &mut errors);

        if self.translation_files.include_patterns.is_empty() {
            errors.push(ValidationError::new(
                "translationFiles.includePatterns",
                "At least one pattern is required. Example: [\"**/{locales,messages}/**/*.json\"]",
            ));
        }

        validate_glob_patterns(
            &self.translation_files.include_patterns,
            "translationFiles.includePatterns",
            &mut errors,
        );
        validate_glob_patterns(
            &self.translation_files.exclude_patterns,
            "translationFiles.excludePatterns",
            &mut errors,
        );

        let mt = &self.diagnostics.missing_translation;
        if mt.required_languages.is_some() && mt.optional_languages.is_some() {
            errors.push(ValidationError::new(
                "diagnostics.missingTranslation.requiredLanguages/optionalLanguages",
                "Cannot specify both 'requiredLanguages' and 'optionalLanguages'. Please use only one",
            ));
        }

        if errors.is_empty() { Ok(()) } else { Err(errors) }
    }
}

fn validate_glob_patterns(
    patterns: &[String],
    field_prefix: &str,
    errors: &mut Vec<ValidationError>,
) {
    for (index, pattern) in patterns.iter().enumerate() {
        if let Err(e) = globset::Glob::new(pattern) {
            errors.push(ValidationError::new(
                format!("{field_prefix}[{index}]"),
                format!("Invalid glob pattern '{pattern}': {e}"),
            ));
        }
    }
}

impl Default for TranslationFilesConfig {
    fn default() -> Self {
        Self {
            include_patterns: vec!["**/{locales,messages}/**/*.json".to_string()],
            exclude_patterns: vec![],
        }
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
    fn validate_required_and_optional_languages_mutually_exclusive() {
        let settings = I18nSettings {
            diagnostics: DiagnosticsConfig {
                missing_translation: MissingTranslationConfig {
                    required_languages: Some(vec!["en".to_string()]),
                    optional_languages: Some(vec!["zh".to_string()]),
                    ..MissingTranslationConfig::default()
                },
                ..DiagnosticsConfig::default()
            },
            ..I18nSettings::default()
        };
        let result = settings.validate();

        assert_that!(
            result,
            err(contains_each![all![
                field!(
                    ValidationError.field_path,
                    eq("diagnostics.missingTranslation.requiredLanguages/optionalLanguages")
                ),
                field!(ValidationError.message, contains_substring("Cannot specify both"))
            ]])
        );
    }

    #[rstest]
    fn deserialize_settings_with_new_diagnostics_structure() {
        let json = r#"{
            "diagnostics": {
                "missingTranslation": {
                    "requiredLanguages": ["en", "ja"]
                },
                "unusedTranslation": {
                    "enabled": false
                }
            }
        }"#;
        let settings: I18nSettings = serde_json::from_str(json).unwrap();

        assert_that!(settings.diagnostics.missing_translation.required_languages, some(len(eq(2))));
        assert_that!(settings.diagnostics.unused_translation.enabled, eq(false));
    }

    #[rstest]
    fn deserialize_diagnostics_config_defaults() {
        let json = "{}";
        let config: DiagnosticsConfig = serde_json::from_str(json).unwrap();

        assert_that!(config.missing_translation.enabled, eq(true));
        assert_that!(config.missing_translation.severity, eq(Severity::Warning));
        assert_that!(config.unused_translation.enabled, eq(true));
        assert_that!(config.unused_translation.severity, eq(Severity::Hint));
    }

    #[rstest]
    fn deserialize_diagnostics_config_nested() {
        let json = r#"{
            "missingTranslation": { "enabled": false },
            "unusedTranslation": { "severity": "error" }
        }"#;
        let config: DiagnosticsConfig = serde_json::from_str(json).unwrap();

        assert_that!(config.missing_translation.enabled, eq(false));
        assert_that!(config.unused_translation.severity, eq(Severity::Error));
    }

    #[rstest]
    fn deserialize_unused_translation_config_defaults() {
        let json = "{}";
        let config: UnusedTranslationConfig = serde_json::from_str(json).unwrap();

        assert_that!(config.enabled, eq(true));
        assert_that!(config.severity, eq(Severity::Hint));
    }

    #[rstest]
    fn deserialize_unused_translation_config_custom() {
        let json = r#"{"enabled": false, "severity": "warning"}"#;
        let config: UnusedTranslationConfig = serde_json::from_str(json).unwrap();

        assert_that!(config.enabled, eq(false));
        assert_that!(config.severity, eq(Severity::Warning));
    }

    #[rstest]
    fn deserialize_missing_translation_config_defaults() {
        let json = "{}";
        let config: MissingTranslationConfig = serde_json::from_str(json).unwrap();

        assert_that!(config.enabled, eq(true));
        assert_that!(config.severity, eq(Severity::Warning));
        assert_that!(config.required_languages, none());
        assert_that!(config.optional_languages, none());
    }

    #[rstest]
    fn deserialize_missing_translation_config_full() {
        let json = r#"{
            "enabled": false,
            "severity": "error",
            "requiredLanguages": ["en", "ja"]
        }"#;
        let config: MissingTranslationConfig = serde_json::from_str(json).unwrap();

        assert_that!(config.enabled, eq(false));
        assert_that!(config.severity, eq(Severity::Error));
        assert_that!(config.required_languages, some(len(eq(2))));
    }

    #[rstest]
    #[case("\"error\"", Severity::Error)]
    #[case("\"warning\"", Severity::Warning)]
    #[case("\"information\"", Severity::Information)]
    #[case("\"hint\"", Severity::Hint)]
    fn deserialize_severity(#[case] json: &str, #[case] expected: Severity) {
        let result: Severity = serde_json::from_str(json).unwrap();
        assert_that!(result, eq(expected));
    }

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
            settings.translation_files.include_patterns,
            elements_are![eq("**/{locales,messages}/**/*.json")]
        );
        assert_that!(settings.translation_files.exclude_patterns, is_empty());
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
    fn validate_invalid_translation_include_patterns_empty() {
        let settings = I18nSettings {
            translation_files: TranslationFilesConfig {
                include_patterns: vec![],
                ..TranslationFilesConfig::default()
            },
            ..I18nSettings::default()
        };

        let result = settings.validate();

        assert_that!(
            result,
            err(elements_are![all![
                field!(ValidationError.field_path, eq("translationFiles.includePatterns")),
                field!(ValidationError.message, contains_substring("At least one pattern"))
            ]])
        );
    }

    #[rstest]
    fn validate_invalid_translation_include_pattern_invalid_glob() {
        let settings = I18nSettings {
            translation_files: TranslationFilesConfig {
                include_patterns: vec!["**/{locales,messages/*.json".to_string()],
                ..TranslationFilesConfig::default()
            },
            ..I18nSettings::default()
        };

        let result = settings.validate();

        assert_that!(
            result,
            err(elements_are![all![
                field!(ValidationError.field_path, eq("translationFiles.includePatterns[0]")),
                field!(ValidationError.message, contains_substring("Invalid glob pattern"))
            ]])
        );
    }

    #[rstest]
    fn validate_invalid_translation_exclude_pattern_invalid_glob() {
        let settings = I18nSettings {
            translation_files: TranslationFilesConfig {
                exclude_patterns: vec!["invalid[pattern".to_string()],
                ..TranslationFilesConfig::default()
            },
            ..I18nSettings::default()
        };

        let result = settings.validate();

        assert_that!(
            result,
            err(elements_are![all![
                field!(ValidationError.field_path, eq("translationFiles.excludePatterns[0]")),
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
