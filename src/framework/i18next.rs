//! i18next / react-i18next library support.

use super::{
    I18nLibrary,
    ParsedTransFnArgs,
    PluralStrategy,
};

#[derive(Debug, Clone, Copy)]
pub struct I18next;

impl I18nLibrary for I18next {
    fn known_global_trans_fns(&self) -> &'static [&'static str] {
        &["i18next.t", "i18n.t"]
    }

    // i18next's t() has no method chain API (rich/markup/raw are next-intl only)
    fn allowed_trans_fn_methods(&self) -> &'static [&'static str] {
        &[]
    }

    fn plural_strategy(&self) -> PluralStrategy {
        PluralStrategy::SuffixBased
    }

    fn parse_get_trans_fn_args(
        &self,
        func_name: &str,
        string_args: &[Option<String>],
    ) -> Option<ParsedTransFnArgs> {
        if !func_name.ends_with("getFixedT") {
            return None;
        }
        // getFixedT(lang, ns?, keyPrefix?)
        Some(ParsedTransFnArgs {
            namespace: string_args.get(1).and_then(Clone::clone),
            key_prefix: string_args.get(2).and_then(Clone::clone),
        })
    }
}
