//! File pattern matcher for source and translation files.
//!
//! Performs pattern matching against config-relative paths.
//! When a config file exists, patterns are relative to its directory.

use std::path::{
    Path,
    PathBuf,
};

use globset::{
    Glob,
    GlobSet,
    GlobSetBuilder,
};

use super::I18nSettings;

#[derive(Debug, thiserror::Error)]
pub enum MatcherError {
    #[error("Invalid source include pattern '{pattern}': {source}")]
    InvalidSourceIncludePattern {
        pattern: String,
        #[source]
        source: globset::Error,
    },

    #[error("Invalid exclude pattern '{pattern}': {source}")]
    InvalidExcludePattern {
        pattern: String,
        #[source]
        source: globset::Error,
    },

    #[error("Invalid translation file pattern '{pattern}': {source}")]
    InvalidTranslationPattern {
        pattern: String,
        #[source]
        source: globset::Error,
    },

    #[error("Failed to build glob set: {0}")]
    GlobSetBuild(#[from] globset::Error),
}

/// Matches files against configured glob patterns.
///
/// Patterns are relative to `pattern_base` (typically the directory containing `.js-i18n.json`).
/// When checking paths relative to a different workspace root, paths are adjusted automatically.
#[derive(Debug, Clone)]
pub struct FileMatcher {
    /// Base directory for pattern matching (config directory or workspace root).
    pattern_base: PathBuf,
    source_include_set: GlobSet,
    exclude_set: GlobSet,
    translation_set: GlobSet,
}

impl FileMatcher {
    /// Creates a new matcher from settings.
    ///
    /// `pattern_base` is the base directory for pattern matching (typically the config directory).
    pub fn new(pattern_base: PathBuf, settings: &I18nSettings) -> Result<Self, MatcherError> {
        let source_include_set =
            Self::build_glob_set(&settings.include_patterns, |pattern, source| {
                MatcherError::InvalidSourceIncludePattern { pattern, source }
            })?;

        let exclude_set = Self::build_glob_set(&settings.exclude_patterns, |pattern, source| {
            MatcherError::InvalidExcludePattern { pattern, source }
        })?;

        let translation_set = Self::build_glob_set(
            std::slice::from_ref(&settings.translation_files.file_pattern),
            |pattern, source| MatcherError::InvalidTranslationPattern { pattern, source },
        )?;

        Ok(Self { pattern_base, source_include_set, exclude_set, translation_set })
    }

    fn build_glob_set<F>(patterns: &[String], make_error: F) -> Result<GlobSet, MatcherError>
    where
        F: Fn(String, globset::Error) -> MatcherError,
    {
        let mut builder = GlobSetBuilder::new();
        for pattern in patterns {
            let glob = Glob::new(pattern).map_err(|e| make_error(pattern.clone(), e))?;
            builder.add(glob);
        }
        Ok(builder.build()?)
    }

    /// Returns the base directory for pattern matching.
    #[must_use]
    pub fn pattern_base(&self) -> &Path {
        &self.pattern_base
    }

    #[must_use]
    #[deprecated(since = "0.0.2", note = "use `pattern_base()` instead")]
    pub fn workspace_root(&self) -> &Path {
        &self.pattern_base
    }

    /// Returns true if the path matches `includePatterns` but not `excludePatterns`.
    ///
    /// The path must be absolute and under the pattern base directory.
    #[must_use]
    pub fn is_source_file(&self, absolute_path: &Path) -> bool {
        let Some(relative_path) = absolute_path.strip_prefix(&self.pattern_base).ok() else {
            return false;
        };

        self.is_source_file_relative(relative_path)
    }

    /// Returns true if the path matches `includePatterns` but not `excludePatterns`.
    ///
    /// The path must be relative to the pattern base directory.
    #[must_use]
    pub fn is_source_file_relative(&self, relative_path: &Path) -> bool {
        self.source_include_set.is_match(relative_path) && !self.exclude_set.is_match(relative_path)
    }

    /// Check if a workspace-relative path matches source patterns.
    ///
    /// When the pattern base differs from the workspace root, this method
    /// adjusts the path accordingly. Files outside the pattern base return false.
    #[must_use]
    pub fn is_source_file_from_workspace(
        &self,
        workspace_root: &Path,
        workspace_relative_path: &Path,
    ) -> bool {
        let absolute_path = workspace_root.join(workspace_relative_path);
        self.is_source_file(&absolute_path)
    }

    /// Returns true if the path matches `translationFiles.filePattern` but not `excludePatterns`.
    ///
    /// The path must be absolute and under the pattern base directory.
    #[must_use]
    pub fn is_translation_file(&self, absolute_path: &Path) -> bool {
        let Some(relative_path) = absolute_path.strip_prefix(&self.pattern_base).ok() else {
            return false;
        };

        self.is_translation_file_relative(relative_path)
    }

    /// Returns true if the path matches `translationFiles.filePattern` but not `excludePatterns`.
    ///
    /// The path must be relative to the pattern base directory.
    #[must_use]
    pub fn is_translation_file_relative(&self, relative_path: &Path) -> bool {
        self.translation_set.is_match(relative_path) && !self.exclude_set.is_match(relative_path)
    }

    /// Check if a workspace-relative path matches translation patterns.
    ///
    /// When the pattern base differs from the workspace root, this method
    /// adjusts the path accordingly. Files outside the pattern base return false.
    #[must_use]
    pub fn is_translation_file_from_workspace(
        &self,
        workspace_root: &Path,
        workspace_relative_path: &Path,
    ) -> bool {
        let absolute_path = workspace_root.join(workspace_relative_path);
        self.is_translation_file(&absolute_path)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use std::path::PathBuf;

    use rstest::rstest;

    use super::*;
    use crate::config::I18nSettings;

    fn create_settings(
        source_include: &[&str],
        exclude: &[&str],
        translation_pattern: &str,
    ) -> I18nSettings {
        I18nSettings {
            include_patterns: source_include.iter().copied().map(String::from).collect(),
            exclude_patterns: exclude.iter().copied().map(String::from).collect(),
            translation_files: crate::config::TranslationFilesConfig {
                file_pattern: translation_pattern.to_string(),
            },
            ..I18nSettings::default()
        }
    }

    #[rstest]
    fn is_source_file_with_default_patterns() {
        let settings = I18nSettings::default();
        let matcher =
            FileMatcher::new(PathBuf::from("/workspace"), &settings).expect("valid patterns");

        assert!(matcher.is_source_file(Path::new("/workspace/src/index.ts")));
        assert!(matcher.is_source_file(Path::new("/workspace/src/App.tsx")));
        assert!(matcher.is_source_file(Path::new("/workspace/lib/utils.js")));
        assert!(matcher.is_source_file(Path::new("/workspace/components/Button.jsx")));

        assert!(!matcher.is_source_file(Path::new("/workspace/README.md")));
        assert!(!matcher.is_source_file(Path::new("/workspace/package.json")));
    }

    #[rstest]
    fn is_source_file_with_exclude_patterns() {
        let settings =
            create_settings(&["**/*.ts"], &["**/node_modules/**", "**/dist/**"], "**/*.json");
        let matcher =
            FileMatcher::new(PathBuf::from("/workspace"), &settings).expect("valid patterns");

        assert!(matcher.is_source_file(Path::new("/workspace/src/index.ts")));
        assert!(!matcher.is_source_file(Path::new("/workspace/node_modules/foo/index.ts")));
        assert!(!matcher.is_source_file(Path::new("/workspace/dist/bundle.ts")));
    }

    #[rstest]
    fn is_source_file_outside_workspace() {
        let settings = I18nSettings::default();
        let matcher =
            FileMatcher::new(PathBuf::from("/workspace"), &settings).expect("valid patterns");

        assert!(!matcher.is_source_file(Path::new("/other/src/index.ts")));
        assert!(!matcher.is_source_file(Path::new("/index.ts")));
    }

    #[rstest]
    fn is_source_file_relative_works() {
        let settings = create_settings(&["**/*.ts"], &[], "**/*.json");
        let matcher =
            FileMatcher::new(PathBuf::from("/workspace"), &settings).expect("valid patterns");

        assert!(matcher.is_source_file_relative(Path::new("src/index.ts")));
        assert!(!matcher.is_source_file_relative(Path::new("src/index.js")));
    }

    #[rstest]
    fn is_translation_file_with_default_pattern() {
        let settings = I18nSettings::default();
        let matcher =
            FileMatcher::new(PathBuf::from("/workspace"), &settings).expect("valid patterns");

        assert!(matcher.is_translation_file(Path::new("/workspace/locales/en.json")));
        assert!(matcher.is_translation_file(Path::new("/workspace/src/locales/ja/common.json")));

        assert!(!matcher.is_translation_file(Path::new("/workspace/package.json")));
        assert!(!matcher.is_translation_file(Path::new("/workspace/src/config.json")));
    }

    #[rstest]
    fn is_translation_file_with_exclude() {
        let settings = create_settings(&["**/*.ts"], &["**/node_modules/**"], "**/i18n/**/*.json");
        let matcher =
            FileMatcher::new(PathBuf::from("/workspace"), &settings).expect("valid patterns");

        assert!(matcher.is_translation_file(Path::new("/workspace/i18n/en.json")));
        assert!(!matcher.is_translation_file(Path::new("/workspace/node_modules/i18n/en.json")));
    }

    #[rstest]
    fn is_translation_file_outside_workspace() {
        let settings = I18nSettings::default();
        let matcher =
            FileMatcher::new(PathBuf::from("/workspace"), &settings).expect("valid patterns");

        assert!(!matcher.is_translation_file(Path::new("/other/locales/en.json")));
    }

    #[rstest]
    fn is_translation_file_relative_works() {
        let settings = create_settings(&["**/*.ts"], &[], "**/locales/**/*.json");
        let matcher =
            FileMatcher::new(PathBuf::from("/workspace"), &settings).expect("valid patterns");

        assert!(matcher.is_translation_file_relative(Path::new("locales/en.json")));
        assert!(!matcher.is_translation_file_relative(Path::new("src/config.json")));
    }

    #[rstest]
    fn new_with_invalid_source_include_pattern() {
        let settings = create_settings(&["**/*.{js,ts"], &[], "**/*.json");

        let result = FileMatcher::new(PathBuf::from("/workspace"), &settings);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, MatcherError::InvalidSourceIncludePattern { .. }));
    }

    #[rstest]
    fn new_with_invalid_exclude_pattern() {
        let settings = create_settings(&["**/*.ts"], &["[invalid"], "**/*.json");

        let result = FileMatcher::new(PathBuf::from("/workspace"), &settings);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, MatcherError::InvalidExcludePattern { .. }));
    }

    #[rstest]
    fn new_with_invalid_translation_pattern() {
        let settings = create_settings(&["**/*.ts"], &[], "**/*.{json");

        let result = FileMatcher::new(PathBuf::from("/workspace"), &settings);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, MatcherError::InvalidTranslationPattern { .. }));
    }

    #[rstest]
    fn pattern_base_accessor() {
        let settings = I18nSettings::default();
        let matcher =
            FileMatcher::new(PathBuf::from("/workspace"), &settings).expect("valid patterns");

        assert_eq!(matcher.pattern_base(), Path::new("/workspace"));
    }
}
