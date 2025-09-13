//! TODO
use std::path::Path;

use super::types::I18nSettings;

/// TODO: doc
pub(super) struct ConfigLoader;

impl ConfigLoader {
    /// TODO: doc
    #[allow(clippy::unused_async)]
    pub(crate) async fn infer_from_package_json(_workspace_path: &Path) -> Option<I18nSettings> {
        // TODO
        None
    }
}
