//! i18n library definitions and framework configuration.

pub mod i18next;
pub mod next_intl;
pub mod svelte_i18n;

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
            &[&i18next::I18next, &next_intl::NextIntl, &svelte_i18n::SvelteI18n]
        }
        ProgrammingLanguage::Svelte => &[&svelte_i18n::SvelteI18n],
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
            known.extend_from_slice(lib.known_global_trans_fns());
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
