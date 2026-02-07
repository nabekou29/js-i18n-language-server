//! File pattern matcher for source and translation files.

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
#[derive(Debug, Clone)]
pub struct FileMatcher {
    workspace_root: PathBuf,
    source_include_set: GlobSet,
    exclude_set: GlobSet,
    translation_set: GlobSet,
    translation_exclude_set: GlobSet,
}

impl FileMatcher {
    /// Creates a new matcher from settings.
    pub fn new(workspace_root: PathBuf, settings: &I18nSettings) -> Result<Self, MatcherError> {
        let source_include_set =
            Self::build_glob_set(&settings.include_patterns, |pattern, source| {
                MatcherError::InvalidSourceIncludePattern { pattern, source }
            })?;

        let exclude_set = Self::build_glob_set(&settings.exclude_patterns, |pattern, source| {
            MatcherError::InvalidExcludePattern { pattern, source }
        })?;

        let translation_set = Self::build_glob_set(
            &settings.translation_files.include_patterns,
            |pattern, source| MatcherError::InvalidTranslationPattern { pattern, source },
        )?;

        let translation_exclude_set = Self::build_glob_set(
            &settings.translation_files.exclude_patterns,
            |pattern, source| MatcherError::InvalidExcludePattern { pattern, source },
        )?;

        Ok(Self {
            workspace_root,
            source_include_set,
            exclude_set,
            translation_set,
            translation_exclude_set,
        })
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

    #[must_use]
    pub fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }

    /// Returns true if the path matches `includePatterns` but not `excludePatterns`.
    ///
    /// The path must be absolute and under the workspace root.
    #[must_use]
    pub fn is_source_file(&self, absolute_path: &Path) -> bool {
        let Some(relative_path) = absolute_path.strip_prefix(&self.workspace_root).ok() else {
            return false;
        };

        self.is_source_file_relative(relative_path)
    }

    /// Returns true if the path matches `includePatterns` but not `excludePatterns`.
    ///
    /// The path must be relative to the workspace root.
    #[must_use]
    pub fn is_source_file_relative(&self, relative_path: &Path) -> bool {
        self.source_include_set.is_match(relative_path) && !self.exclude_set.is_match(relative_path)
    }

    /// Returns true if the path matches `translationFiles.includePatterns`
    /// but not `excludePatterns` or `translationFiles.excludePatterns`.
    ///
    /// The path must be absolute and under the workspace root.
    #[must_use]
    pub fn is_translation_file(&self, absolute_path: &Path) -> bool {
        let Some(relative_path) = absolute_path.strip_prefix(&self.workspace_root).ok() else {
            return false;
        };

        self.is_translation_file_relative(relative_path)
    }

    /// Returns true if the path matches `translationFiles.includePatterns`
    /// but not `excludePatterns` or `translationFiles.excludePatterns`.
    ///
    /// The path must be relative to the workspace root.
    #[must_use]
    pub fn is_translation_file_relative(&self, relative_path: &Path) -> bool {
        self.translation_set.is_match(relative_path)
            && !self.exclude_set.is_match(relative_path)
            && !self.translation_exclude_set.is_match(relative_path)
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
        translation_patterns: &[&str],
    ) -> I18nSettings {
        I18nSettings {
            include_patterns: source_include.iter().copied().map(String::from).collect(),
            exclude_patterns: exclude.iter().copied().map(String::from).collect(),
            translation_files: crate::config::TranslationFilesConfig {
                include_patterns: translation_patterns.iter().copied().map(String::from).collect(),
                ..crate::config::TranslationFilesConfig::default()
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
            create_settings(&["**/*.ts"], &["**/node_modules/**", "**/dist/**"], &["**/*.json"]);
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
        let settings = create_settings(&["**/*.ts"], &[], &["**/*.json"]);
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
        let settings =
            create_settings(&["**/*.ts"], &["**/node_modules/**"], &["**/i18n/**/*.json"]);
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
        let settings = create_settings(&["**/*.ts"], &[], &["**/locales/**/*.json"]);
        let matcher =
            FileMatcher::new(PathBuf::from("/workspace"), &settings).expect("valid patterns");

        assert!(matcher.is_translation_file_relative(Path::new("locales/en.json")));
        assert!(!matcher.is_translation_file_relative(Path::new("src/config.json")));
    }

    #[rstest]
    fn new_with_invalid_source_include_pattern() {
        let settings = create_settings(&["**/*.{js,ts"], &[], &["**/*.json"]);

        let result = FileMatcher::new(PathBuf::from("/workspace"), &settings);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, MatcherError::InvalidSourceIncludePattern { .. }));
    }

    #[rstest]
    fn new_with_invalid_exclude_pattern() {
        let settings = create_settings(&["**/*.ts"], &["[invalid"], &["**/*.json"]);

        let result = FileMatcher::new(PathBuf::from("/workspace"), &settings);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, MatcherError::InvalidExcludePattern { .. }));
    }

    #[rstest]
    fn new_with_invalid_translation_pattern() {
        let settings = create_settings(&["**/*.ts"], &[], &["**/*.{json"]);

        let result = FileMatcher::new(PathBuf::from("/workspace"), &settings);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, MatcherError::InvalidTranslationPattern { .. }));
    }

    #[rstest]
    fn workspace_root_accessor() {
        let settings = I18nSettings::default();
        let matcher =
            FileMatcher::new(PathBuf::from("/workspace"), &settings).expect("valid patterns");

        assert_eq!(matcher.workspace_root(), Path::new("/workspace"));
    }
}
