//! Test utilities.

use std::collections::HashMap;

use crate::db::I18nDatabaseImpl;
use crate::input::translation::Translation;

#[allow(clippy::redundant_pub_crate)]
pub(crate) fn create_translation(
    db: &I18nDatabaseImpl,
    language: &str,
    file_path: &str,
    keys: HashMap<String, String>,
) -> Translation {
    create_translation_with_namespace(db, language, None, file_path, keys)
}

#[allow(clippy::redundant_pub_crate)]
pub(crate) fn create_translation_with_namespace(
    db: &I18nDatabaseImpl,
    language: &str,
    namespace: Option<&str>,
    file_path: &str,
    keys: HashMap<String, String>,
) -> Translation {
    Translation::new(
        db,
        language.to_string(),
        namespace.map(String::from),
        file_path.to_string(),
        keys,
        "{}".to_string(),
        HashMap::new(),
        HashMap::new(),
    )
}
