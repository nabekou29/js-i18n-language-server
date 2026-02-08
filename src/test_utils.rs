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
    create_translation_with_json(db, language, namespace, file_path, keys, "{}")
}

/// Creates a `Translation` with custom JSON text content.
#[allow(clippy::redundant_pub_crate)]
pub(crate) fn create_translation_with_json(
    db: &I18nDatabaseImpl,
    language: &str,
    namespace: Option<&str>,
    file_path: &str,
    keys: HashMap<String, String>,
    json_text: &str,
) -> Translation {
    Translation::new(
        db,
        language.to_string(),
        namespace.map(String::from),
        file_path.to_string(),
        keys,
        json_text.to_string(),
        HashMap::new(),
        HashMap::new(),
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use googletest::prelude::*;
    use rstest::*;

    use super::*;

    #[rstest]
    fn create_translation_with_json_stores_json_text() {
        let db = I18nDatabaseImpl::default();
        let json = r#"{"hello": "world"}"#;
        let t = create_translation_with_json(
            &db,
            "en",
            None,
            "/locales/en.json",
            HashMap::from([("hello".to_string(), "world".to_string())]),
            json,
        );
        assert_that!(t.json_text(&db), eq(json));
    }

    #[rstest]
    fn create_translation_with_namespace_uses_empty_json() {
        let db = I18nDatabaseImpl::default();
        let t = create_translation_with_namespace(
            &db,
            "en",
            Some("common"),
            "/locales/en/common.json",
            HashMap::new(),
        );
        assert_that!(t.json_text(&db), eq("{}"));
        assert_that!(t.namespace(&db).as_deref(), some(eq("common")));
    }
}
