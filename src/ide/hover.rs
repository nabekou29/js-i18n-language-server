//! Hover implementation

use crate::db::I18nDatabase;
use crate::input::translation::Translation;
use crate::interned::TransKey;

/// Generate hover content for a translation key
pub fn generate_hover_content(
    db: &dyn I18nDatabase,
    key: TransKey<'_>,
    translations: &[Translation],
) -> Option<String> {
    let key_text = key.text(db);

    // Collect translations for this key
    let mut translations_found = Vec::new();

    for translation in translations {
        let keys = translation.keys(db);
        if let Some(value) = keys.get(key_text) {
            let language = translation.language(db);
            translations_found.push((language.clone(), value.clone()));
        }
    }

    // No translations found
    if translations_found.is_empty() {
        return None;
    }

    // Format as markdown
    let mut content = format!("**Translation Key:** `{key_text}`\n\n");

    // Sort by language code
    translations_found.sort_by(|a, b| a.0.cmp(&b.0));

    for (language, value) in translations_found {
        content.push_str(&format!("**{language}**: {value}\n"));
    }

    Some(content)
}
