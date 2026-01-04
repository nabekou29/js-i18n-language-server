//! ファイルパターンマッチャー
//!
//! ワークスペースルートからの相対パスでパターンマッチングを行う型安全な実装を提供します。
//! ソースファイルと翻訳ファイルの両方を判定できます。

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

/// ファイルパターンマッチングのエラー
#[derive(Debug, thiserror::Error)]
pub enum MatcherError {
    /// 無効なソースファイル include パターン
    #[error("Invalid source include pattern '{pattern}': {source}")]
    InvalidSourceIncludePattern {
        pattern: String,
        #[source]
        source: globset::Error,
    },

    /// 無効な exclude パターン
    #[error("Invalid exclude pattern '{pattern}': {source}")]
    InvalidExcludePattern {
        pattern: String,
        #[source]
        source: globset::Error,
    },

    /// 無効な翻訳ファイルパターン
    #[error("Invalid translation file pattern '{pattern}': {source}")]
    InvalidTranslationPattern {
        pattern: String,
        #[source]
        source: globset::Error,
    },

    /// `GlobSet` のビルドエラー
    #[error("Failed to build glob set: {0}")]
    GlobSetBuild(#[from] globset::Error),
}

/// ファイルパターンマッチャー
///
/// ワークスペースルートからの相対パスでパターンマッチングを行います。
/// ソースファイルと翻訳ファイルの両方を判定できます。
///
/// # Example
///
/// ```ignore
/// let matcher = FileMatcher::new(PathBuf::from("/workspace"), &settings)?;
///
/// // ソースファイル判定
/// assert!(matcher.is_source_file(Path::new("/workspace/src/index.ts")));
/// assert!(!matcher.is_source_file(Path::new("/workspace/node_modules/foo.ts")));
///
/// // 翻訳ファイル判定
/// assert!(matcher.is_translation_file(Path::new("/workspace/locales/en.json")));
/// ```
#[derive(Debug, Clone)]
pub struct FileMatcher {
    /// ワークスペースルート
    workspace_root: PathBuf,

    /// ソースファイル include パターンセット
    source_include_set: GlobSet,
    /// exclude パターンセット（ソース・翻訳共通）
    exclude_set: GlobSet,

    /// 翻訳ファイルパターンセット
    translation_set: GlobSet,
}

impl FileMatcher {
    /// 設定からマッチャーを構築
    ///
    /// # Arguments
    /// * `workspace_root` - ワークスペースのルートパス
    /// * `settings` - i18n 設定
    ///
    /// # Errors
    /// パターンが無効な場合にエラーを返します。
    pub fn new(workspace_root: PathBuf, settings: &I18nSettings) -> Result<Self, MatcherError> {
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

        Ok(Self { workspace_root, source_include_set, exclude_set, translation_set })
    }

    /// パターンリストから `GlobSet` を構築
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

    /// ワークスペースルートを取得
    #[must_use]
    pub fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }

    /// 絶対パスがソースファイルかどうか判定
    ///
    /// `includePatterns` にマッチし、かつ `excludePatterns` にマッチしないファイルを
    /// ソースファイルとして判定します。
    /// ワークスペース外のパスは常に `false` を返します。
    #[must_use]
    pub fn is_source_file(&self, absolute_path: &Path) -> bool {
        let Some(relative_path) = absolute_path.strip_prefix(&self.workspace_root).ok() else {
            return false;
        };

        self.is_source_file_relative(relative_path)
    }

    /// 相対パスがソースファイルかどうか判定
    #[must_use]
    pub fn is_source_file_relative(&self, relative_path: &Path) -> bool {
        self.source_include_set.is_match(relative_path) && !self.exclude_set.is_match(relative_path)
    }

    /// 絶対パスが翻訳ファイルかどうか判定
    ///
    /// `translationFiles.filePattern` にマッチし、かつ `excludePatterns` にマッチしない
    /// ファイルを翻訳ファイルとして判定します。
    /// ワークスペース外のパスは常に `false` を返します。
    #[must_use]
    pub fn is_translation_file(&self, absolute_path: &Path) -> bool {
        let Some(relative_path) = absolute_path.strip_prefix(&self.workspace_root).ok() else {
            return false;
        };

        self.is_translation_file_relative(relative_path)
    }

    /// 相対パスが翻訳ファイルかどうか判定
    #[must_use]
    pub fn is_translation_file_relative(&self, relative_path: &Path) -> bool {
        self.translation_set.is_match(relative_path) && !self.exclude_set.is_match(relative_path)
    }
}

#[cfg(test)]
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
        let mut settings = I18nSettings::default();
        settings.include_patterns = source_include.iter().map(|s| s.to_string()).collect();
        settings.exclude_patterns = exclude.iter().map(|s| s.to_string()).collect();
        settings.translation_files.file_pattern = translation_pattern.to_string();
        settings
    }

    // ===== ソースファイル判定テスト =====

    #[rstest]
    fn is_source_file_with_default_patterns() {
        let settings = I18nSettings::default();
        let matcher =
            FileMatcher::new(PathBuf::from("/workspace"), &settings).expect("valid patterns");

        // デフォルトの include パターン: **/*.{js,jsx,ts,tsx}
        assert!(matcher.is_source_file(Path::new("/workspace/src/index.ts")));
        assert!(matcher.is_source_file(Path::new("/workspace/src/App.tsx")));
        assert!(matcher.is_source_file(Path::new("/workspace/lib/utils.js")));
        assert!(matcher.is_source_file(Path::new("/workspace/components/Button.jsx")));

        // マッチしないファイル
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

    // ===== 翻訳ファイル判定テスト =====

    #[rstest]
    fn is_translation_file_with_default_pattern() {
        let settings = I18nSettings::default();
        let matcher =
            FileMatcher::new(PathBuf::from("/workspace"), &settings).expect("valid patterns");

        // デフォルトパターン: **/locales/**/*.json
        assert!(matcher.is_translation_file(Path::new("/workspace/locales/en.json")));
        assert!(matcher.is_translation_file(Path::new("/workspace/src/locales/ja/common.json")));

        // マッチしないファイル
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

    // ===== エラーケース =====

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
    fn workspace_root_accessor() {
        let settings = I18nSettings::default();
        let matcher =
            FileMatcher::new(PathBuf::from("/workspace"), &settings).expect("valid patterns");

        assert_eq!(matcher.workspace_root(), Path::new("/workspace"));
    }
}
