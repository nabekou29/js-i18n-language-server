//! Salsa データベース定義

/// I18n LSP のデータベーストレイト
#[salsa::db]
pub trait I18nDatabase: salsa::Database {}

/// I18n データベースの実装
#[salsa::db]
#[derive(Clone)]
pub struct I18nDatabaseImpl {
    /// Salsa のストレージ
    storage: salsa::Storage<Self>,
}

#[salsa::db]
impl salsa::Database for I18nDatabaseImpl {}

#[salsa::db]
impl I18nDatabase for I18nDatabaseImpl {}
