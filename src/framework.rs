//! i18n library definitions and framework configuration.

pub mod i18next;
pub mod next_intl;
pub mod svelte_i18n;
pub mod vue_i18n;

use std::sync::OnceLock;

use crate::input::source::ProgrammingLanguage;

/// How plural keys are handled for a given framework.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluralStrategy {
    /// i18next: plural variants use key suffixes (`_one`, `_other`, etc.)
    SuffixBased,
    /// `ICU` `MessageFormat` (`next-intl`, `svelte-i18n`): plurals are embedded in
    /// values, not keys. No suffix-based plural handling needed.
    Icu,
}

/// Defines i18n library-specific behavior.
///
/// Each supported library implements this trait, co-locating all
/// library-specific constants and logic in one place.
pub trait I18nLibrary: Send + Sync {
    /// Global translation function names (e.g., `i18next.t`, `$_`).
    fn known_global_trans_fns(&self) -> &'static [&'static str];

    /// Allowed method names on translation functions (e.g., `t.rich()`).
    fn allowed_trans_fn_methods(&self) -> &'static [&'static str];

    /// How this library handles plural keys.
    fn plural_strategy(&self) -> PluralStrategy;

    /// Parse library-specific arguments from a `get_trans_fn` capture.
    ///
    /// Returns `None` if this library doesn't handle the given `func_name`.
    fn parse_get_trans_fn_args(
        &self,
        _func_name: &str,
        _string_args: &[Option<String>],
    ) -> Option<ParsedTransFnArgs> {
        None
    }
}

/// Parsed result from library-specific `get_trans_fn` argument handling.
#[derive(Debug, Clone, Default)]
pub struct ParsedTransFnArgs {
    pub namespace: Option<String>,
    pub key_prefix: Option<String>,
}

/// Returns the applicable i18n libraries for a programming language.
#[must_use]
pub fn applicable_libraries(lang: ProgrammingLanguage) -> &'static [&'static dyn I18nLibrary] {
    match lang {
        ProgrammingLanguage::Jsx | ProgrammingLanguage::Tsx => {
            &[&i18next::I18next, &next_intl::NextIntl]
        }
        ProgrammingLanguage::JavaScript | ProgrammingLanguage::TypeScript => {
            &[&i18next::I18next, &next_intl::NextIntl, &svelte_i18n::SvelteI18n, &vue_i18n::VueI18n]
        }
        ProgrammingLanguage::Svelte => &[&svelte_i18n::SvelteI18n],
        ProgrammingLanguage::Vue => &[&vue_i18n::VueI18n],
    }
}

/// Merged configuration from all applicable i18n libraries for a language.
///
/// Computed once per language variant and cached for the process lifetime.
/// Merge rules: union of globals/methods, conservative plural strategy
/// (`SuffixBased` wins if any library uses it).
pub struct FrameworkConfig {
    pub known_global_trans_fns: Vec<&'static str>,
    pub allowed_trans_fn_methods: Vec<&'static str>,
    pub plural_strategy: PluralStrategy,
    libraries: &'static [&'static dyn I18nLibrary],
}

impl std::fmt::Debug for FrameworkConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FrameworkConfig")
            .field("known_global_trans_fns", &self.known_global_trans_fns)
            .field("allowed_trans_fn_methods", &self.allowed_trans_fn_methods)
            .field("plural_strategy", &self.plural_strategy)
            .finish_non_exhaustive()
    }
}

impl FrameworkConfig {
    fn build(lang: ProgrammingLanguage) -> Self {
        let libraries = applicable_libraries(lang);
        let mut known = Vec::new();
        let mut methods = Vec::new();
        let mut plural = PluralStrategy::Icu;

        for lib in libraries {
            for &g in lib.known_global_trans_fns() {
                if !known.contains(&g) {
                    known.push(g);
                }
            }
            for &m in lib.allowed_trans_fn_methods() {
                if !methods.contains(&m) {
                    methods.push(m);
                }
            }
            if lib.plural_strategy() == PluralStrategy::SuffixBased {
                plural = PluralStrategy::SuffixBased;
            }
        }
        Self {
            known_global_trans_fns: known,
            allowed_trans_fn_methods: methods,
            plural_strategy: plural,
            libraries,
        }
    }

    /// Get cached config for a language.
    #[must_use]
    pub fn for_language(lang: ProgrammingLanguage) -> &'static Self {
        static JS: OnceLock<FrameworkConfig> = OnceLock::new();
        static JSX: OnceLock<FrameworkConfig> = OnceLock::new();
        static TS: OnceLock<FrameworkConfig> = OnceLock::new();
        static TSX: OnceLock<FrameworkConfig> = OnceLock::new();
        static SVELTE: OnceLock<FrameworkConfig> = OnceLock::new();
        static VUE: OnceLock<FrameworkConfig> = OnceLock::new();

        match lang {
            ProgrammingLanguage::JavaScript => {
                JS.get_or_init(|| Self::build(ProgrammingLanguage::JavaScript))
            }
            ProgrammingLanguage::Jsx => JSX.get_or_init(|| Self::build(ProgrammingLanguage::Jsx)),
            ProgrammingLanguage::TypeScript => {
                TS.get_or_init(|| Self::build(ProgrammingLanguage::TypeScript))
            }
            ProgrammingLanguage::Tsx => TSX.get_or_init(|| Self::build(ProgrammingLanguage::Tsx)),
            ProgrammingLanguage::Svelte => {
                SVELTE.get_or_init(|| Self::build(ProgrammingLanguage::Svelte))
            }
            ProgrammingLanguage::Vue => VUE.get_or_init(|| Self::build(ProgrammingLanguage::Vue)),
        }
    }

    /// Delegate argument parsing to the first library that handles this `func_name`.
    #[must_use]
    pub fn parse_get_trans_fn_args(
        &self,
        func_name: &str,
        string_args: &[Option<String>],
    ) -> Option<ParsedTransFnArgs> {
        self.libraries.iter().find_map(|lib| lib.parse_get_trans_fn_args(func_name, string_args))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::expect_used, clippy::panic)]
mod tests {
    use googletest::prelude::*;
    use rstest::*;

    use super::*;

    // --- applicable_libraries ---

    #[rstest]
    #[case::jsx(ProgrammingLanguage::Jsx, 2)]
    #[case::tsx(ProgrammingLanguage::Tsx, 2)]
    #[case::js(ProgrammingLanguage::JavaScript, 4)]
    #[case::ts(ProgrammingLanguage::TypeScript, 4)]
    #[case::svelte(ProgrammingLanguage::Svelte, 1)]
    #[case::vue(ProgrammingLanguage::Vue, 1)]
    fn applicable_libraries_count(#[case] lang: ProgrammingLanguage, #[case] expected: usize) {
        assert_that!(applicable_libraries(lang).len(), eq(expected));
    }

    // --- FrameworkConfig merge ---

    #[rstest]
    fn jsx_config_has_i18next_globals_but_not_svelte() {
        let config = FrameworkConfig::for_language(ProgrammingLanguage::Jsx);

        assert!(config.known_global_trans_fns.contains(&"i18next.t"));
        assert!(config.known_global_trans_fns.contains(&"i18n.t"));
        assert!(!config.known_global_trans_fns.contains(&"$_"));
    }

    #[rstest]
    fn jsx_config_has_next_intl_methods() {
        let config = FrameworkConfig::for_language(ProgrammingLanguage::Jsx);

        assert!(config.allowed_trans_fn_methods.contains(&"rich"));
        assert!(config.allowed_trans_fn_methods.contains(&"markup"));
        assert!(config.allowed_trans_fn_methods.contains(&"raw"));
    }

    #[rstest]
    fn js_config_includes_all_frameworks() {
        let config = FrameworkConfig::for_language(ProgrammingLanguage::JavaScript);

        assert!(config.known_global_trans_fns.contains(&"i18next.t"));
        assert!(config.known_global_trans_fns.contains(&"$_"));
        assert!(config.known_global_trans_fns.contains(&"$t"));
        assert!(config.allowed_trans_fn_methods.contains(&"rich"));
    }

    #[rstest]
    fn svelte_config_has_only_svelte_i18n() {
        let config = FrameworkConfig::for_language(ProgrammingLanguage::Svelte);

        assert!(config.known_global_trans_fns.contains(&"$_"));
        assert!(!config.known_global_trans_fns.contains(&"i18next.t"));
        assert_that!(config.allowed_trans_fn_methods, is_empty());
    }

    #[rstest]
    fn vue_config_has_only_vue_i18n() {
        let config = FrameworkConfig::for_language(ProgrammingLanguage::Vue);

        assert!(config.known_global_trans_fns.contains(&"$t"));
        assert!(config.known_global_trans_fns.contains(&"$tc"));
        assert!(config.known_global_trans_fns.contains(&"$te"));
        assert!(config.known_global_trans_fns.contains(&"$tm"));
        assert!(!config.known_global_trans_fns.contains(&"i18next.t"));
        assert!(!config.known_global_trans_fns.contains(&"$_"));
        assert_that!(config.allowed_trans_fn_methods, is_empty());
    }

    // --- PluralStrategy merge ---

    #[rstest]
    fn js_plural_strategy_is_suffix_based() {
        // i18next (SuffixBased) wins over next-intl/svelte-i18n (Icu)
        let config = FrameworkConfig::for_language(ProgrammingLanguage::JavaScript);
        assert_that!(config.plural_strategy, eq(PluralStrategy::SuffixBased));
    }

    #[rstest]
    fn svelte_plural_strategy_is_icu() {
        let config = FrameworkConfig::for_language(ProgrammingLanguage::Svelte);
        assert_that!(config.plural_strategy, eq(PluralStrategy::Icu));
    }

    #[rstest]
    fn vue_plural_strategy_is_icu() {
        let config = FrameworkConfig::for_language(ProgrammingLanguage::Vue);
        assert_that!(config.plural_strategy, eq(PluralStrategy::Icu));
    }

    #[rstest]
    fn tsx_plural_strategy_is_suffix_based() {
        let config = FrameworkConfig::for_language(ProgrammingLanguage::Tsx);
        assert_that!(config.plural_strategy, eq(PluralStrategy::SuffixBased));
    }

    // --- parse_get_trans_fn_args delegation ---

    #[rstest]
    fn js_delegates_get_fixed_t_to_i18next() {
        let config = FrameworkConfig::for_language(ProgrammingLanguage::JavaScript);
        let args =
            vec![Some("en".to_string()), Some("common".to_string()), Some("prefix".to_string())];

        let parsed = config.parse_get_trans_fn_args("getFixedT", &args).unwrap();
        assert_that!(parsed.namespace, some(eq("common")));
        assert_that!(parsed.key_prefix, some(eq("prefix")));
    }

    #[rstest]
    fn js_delegates_use_translations_to_next_intl() {
        let config = FrameworkConfig::for_language(ProgrammingLanguage::JavaScript);
        let args = vec![Some("Messages".to_string())];

        let parsed = config.parse_get_trans_fn_args("useTranslations", &args).unwrap();
        assert_that!(parsed.namespace, none());
        assert_that!(parsed.key_prefix, some(eq("Messages")));
    }

    #[rstest]
    fn svelte_does_not_parse_get_fixed_t() {
        let config = FrameworkConfig::for_language(ProgrammingLanguage::Svelte);
        let args = vec![Some("en".to_string())];

        assert_that!(config.parse_get_trans_fn_args("getFixedT", &args), none());
    }

    #[rstest]
    fn unknown_func_returns_none() {
        let config = FrameworkConfig::for_language(ProgrammingLanguage::JavaScript);
        assert_that!(config.parse_get_trans_fn_args("unknownFunc", &[]), none());
    }

    // --- Deduplication ---

    #[rstest]
    fn no_duplicate_globals() {
        let config = FrameworkConfig::for_language(ProgrammingLanguage::JavaScript);
        let mut seen = std::collections::HashSet::new();
        for g in &config.known_global_trans_fns {
            assert!(seen.insert(g), "duplicate global: {g}");
        }
    }

    #[rstest]
    fn no_duplicate_methods() {
        let config = FrameworkConfig::for_language(ProgrammingLanguage::JavaScript);
        let mut seen = std::collections::HashSet::new();
        for m in &config.allowed_trans_fn_methods {
            assert!(seen.insert(m), "duplicate method: {m}");
        }
    }
}
