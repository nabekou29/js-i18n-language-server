//! Salsa database definitions.

#[salsa::db]
pub trait I18nDatabase: salsa::Database + Send {}

#[salsa::db]
#[derive(Clone, Default)]
pub struct I18nDatabaseImpl {
    storage: salsa::Storage<Self>,
}

impl std::fmt::Debug for I18nDatabaseImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("I18nDatabaseImpl").finish_non_exhaustive()
    }
}

#[salsa::db]
impl salsa::Database for I18nDatabaseImpl {}

#[salsa::db]
impl I18nDatabase for I18nDatabaseImpl {}
