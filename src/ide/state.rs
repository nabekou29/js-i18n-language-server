//! Shared state for the LSP server.

use std::collections::{
    HashMap,
    HashSet,
};
use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::{
    Mutex,
    MutexGuard,
};
use tower_lsp::lsp_types::WorkspaceFolder;

use crate::db::I18nDatabaseImpl;
use crate::input::source::SourceFile;
use crate::input::translation::Translation;

pub type PendingUpdate = (tower_lsp::lsp_types::Url, String, bool);

/// Shared state for the LSP server.
///
/// # Lock Ordering
///
/// When acquiring multiple locks, always follow this order:
/// 1. `db`
/// 2. `source_files` / `translations` / `opened_files`
#[derive(Clone)]
pub struct ServerState {
    pub db: Arc<Mutex<I18nDatabaseImpl>>,
    pub source_files: Arc<Mutex<HashMap<PathBuf, SourceFile>>>,
    pub translations: Arc<Mutex<Vec<Translation>>>,
    pub opened_files: Arc<Mutex<HashSet<tower_lsp::lsp_types::Url>>>,
    /// Current language for Virtual Text, completion, and Code Actions.
    /// Changeable via `i18n.setCurrentLanguage` command.
    pub current_language: Arc<Mutex<Option<String>>>,
    /// Updates skipped during indexing; processed after indexing completes.
    pub pending_updates: Arc<Mutex<Vec<PendingUpdate>>>,
    /// Whether the client supports edit translation code actions (from `experimental.i18nEditTranslationCodeAction`).
    pub code_actions_enabled: Arc<Mutex<bool>>,
    /// Workspace folders from `initialize` params (not from runtime LSP request).
    /// Ensures each server only indexes its assigned folders in multi-server setups.
    pub workspace_folders: Arc<Mutex<Vec<WorkspaceFolder>>>,
}

impl ServerState {
    pub fn new(db: I18nDatabaseImpl) -> Self {
        Self {
            db: Arc::new(Mutex::new(db)),
            source_files: Arc::new(Mutex::new(HashMap::new())),
            translations: Arc::new(Mutex::new(Vec::new())),
            opened_files: Arc::new(Mutex::new(HashSet::new())),
            current_language: Arc::new(Mutex::new(None)),
            pending_updates: Arc::new(Mutex::new(Vec::new())),
            code_actions_enabled: Arc::new(Mutex::new(false)),
            workspace_folders: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Acquires locks on `db` and `translations` in correct order.
    pub async fn lock_db_and_translations(
        &self,
    ) -> (MutexGuard<'_, I18nDatabaseImpl>, MutexGuard<'_, Vec<Translation>>) {
        let db = self.db.lock().await;
        let translations = self.translations.lock().await;
        (db, translations)
    }

    /// Acquires locks on `db` and `source_files` in correct order.
    pub async fn lock_db_and_source_files(
        &self,
    ) -> (MutexGuard<'_, I18nDatabaseImpl>, MutexGuard<'_, HashMap<PathBuf, SourceFile>>) {
        let db = self.db.lock().await;
        let source_files = self.source_files.lock().await;
        (db, source_files)
    }

    /// Acquires all locks in correct order.
    pub async fn lock_all(
        &self,
    ) -> (
        MutexGuard<'_, I18nDatabaseImpl>,
        MutexGuard<'_, HashMap<PathBuf, SourceFile>>,
        MutexGuard<'_, Vec<Translation>>,
    ) {
        let db = self.db.lock().await;
        let source_files = self.source_files.lock().await;
        let translations = self.translations.lock().await;
        (db, source_files, translations)
    }
}

impl std::fmt::Debug for ServerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServerState")
            .field("db", &"<I18nDatabaseImpl>")
            .field("source_files", &"<HashMap<PathBuf, SourceFile>>")
            .field("translations", &"<Vec<Translation>>")
            .field("opened_files", &"<HashSet<Url>>")
            .field("current_language", &"<Option<String>>")
            .field("pending_updates", &"<Vec<PendingUpdate>>")
            .field("code_actions_enabled", &"<bool>")
            .field("workspace_folders", &"<Vec<WorkspaceFolder>>")
            .finish()
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::significant_drop_tightening,
    clippy::field_reassign_with_default
)]
mod tests {
    use std::path::PathBuf;

    use googletest::prelude::*;
    use rstest::*;

    use super::*;

    #[rstest]
    fn new_creates_empty_state() {
        let db = I18nDatabaseImpl::default();
        let state = ServerState::new(db);

        assert_that!(Arc::strong_count(&state.db), eq(1));
        assert_that!(Arc::strong_count(&state.source_files), eq(1));
        assert_that!(Arc::strong_count(&state.translations), eq(1));
        assert_that!(Arc::strong_count(&state.opened_files), eq(1));
        assert_that!(Arc::strong_count(&state.current_language), eq(1));
    }

    #[rstest]
    fn clone_shares_state() {
        let db = I18nDatabaseImpl::default();
        let state1 = ServerState::new(db);
        let state2 = state1.clone();

        assert_that!(Arc::strong_count(&state1.db), eq(2));
        assert_that!(Arc::strong_count(&state1.source_files), eq(2));
        assert_that!(Arc::strong_count(&state1.translations), eq(2));
        assert_that!(Arc::strong_count(&state1.opened_files), eq(2));
        assert_that!(Arc::strong_count(&state1.current_language), eq(2));

        assert_that!(Arc::ptr_eq(&state1.db, &state2.db), eq(true));
        assert_that!(Arc::ptr_eq(&state1.source_files, &state2.source_files), eq(true));
    }

    #[rstest]
    fn debug_impl_works() {
        let db = I18nDatabaseImpl::default();
        let state = ServerState::new(db);

        let debug_str = format!("{state:?}");

        assert_that!(debug_str, contains_substring("ServerState"));
        assert_that!(debug_str, contains_substring("db"));
        assert_that!(debug_str, contains_substring("source_files"));
        assert_that!(debug_str, contains_substring("translations"));
        assert_that!(debug_str, contains_substring("opened_files"));
        assert_that!(debug_str, contains_substring("current_language"));
    }

    #[tokio::test]
    async fn state_can_be_modified_through_locks() {
        let db = I18nDatabaseImpl::default();
        let state = ServerState::new(db);

        {
            let mut source_files = state.source_files.lock().await;
            let dummy_source = SourceFile::new(
                &*state.db.lock().await,
                "file:///test.ts".to_string(),
                "const x = 1;".to_string(),
                crate::input::source::ProgrammingLanguage::TypeScript,
            );
            source_files.insert(PathBuf::from("/test.ts"), dummy_source);
        }

        let source_files = state.source_files.lock().await;
        assert_eq!(source_files.len(), 1);
        assert!(source_files.contains_key(&PathBuf::from("/test.ts")));
    }

    #[tokio::test]
    async fn cloned_state_shares_modifications() {
        let db = I18nDatabaseImpl::default();
        let state1 = ServerState::new(db);
        let state2 = state1.clone();

        {
            let mut opened_files = state1.opened_files.lock().await;
            let uri = tower_lsp::lsp_types::Url::parse("file:///test.ts").unwrap();
            opened_files.insert(uri);
        }

        let opened_files = state2.opened_files.lock().await;
        assert_eq!(opened_files.len(), 1);
    }
}
