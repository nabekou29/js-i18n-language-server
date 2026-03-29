//! next-intl library support.

use super::{
    I18nLibrary,
    ParsedTransFnArgs,
    PluralStrategy,
};

#[derive(Debug, Clone, Copy)]
pub struct NextIntl;

impl I18nLibrary for NextIntl {
    fn known_global_trans_fns(&self) -> &'static [&'static str] {
        &[]
    }

    // t.rich(), t.markup(), t.raw() are next-intl specific APIs
    fn allowed_trans_fn_methods(&self) -> &'static [&'static str] {
        &["rich", "markup", "raw"]
    }

    fn plural_strategy(&self) -> PluralStrategy {
        PluralStrategy::Icu
    }

    fn parse_get_trans_fn_args(
        &self,
        func_name: &str,
        string_args: &[Option<String>],
    ) -> Option<ParsedTransFnArgs> {
        if func_name != "useTranslations" {
            return None;
        }
        // useTranslations(namespace?)
        // In next-intl, the namespace parameter acts as a key prefix
        Some(ParsedTransFnArgs {
            namespace: None,
            key_prefix: string_args.first().and_then(Clone::clone),
        })
    }
}
