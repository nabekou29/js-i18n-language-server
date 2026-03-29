//! svelte-i18n library support.

use super::{
    I18nLibrary,
    PluralStrategy,
};

#[derive(Debug, Clone, Copy)]
pub struct SvelteI18n;

impl I18nLibrary for SvelteI18n {
    fn known_global_trans_fns(&self) -> &'static [&'static str] {
        &["$_", "$t", "$format", "$json"]
    }

    fn allowed_trans_fn_methods(&self) -> &'static [&'static str] {
        &[]
    }

    fn plural_strategy(&self) -> PluralStrategy {
        PluralStrategy::Icu
    }
}
