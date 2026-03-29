//! vue-i18n library support.

use super::{
    I18nLibrary,
    PluralStrategy,
};

#[derive(Debug, Clone, Copy)]
pub struct VueI18n;

impl I18nLibrary for VueI18n {
    fn known_global_trans_fns(&self) -> &'static [&'static str] {
        // $t, $tc, $te, $tm are global template functions in Options API and templates.
        // Bare `t` is handled by the universal convention in is_trans_fn().
        &["$t", "$tc", "$te", "$tm"]
    }

    fn allowed_trans_fn_methods(&self) -> &'static [&'static str] {
        &[]
    }

    fn plural_strategy(&self) -> PluralStrategy {
        // vue-i18n uses pipe-separated plurals in values, not key suffixes.
        PluralStrategy::Icu
    }
}
