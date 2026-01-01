//! LSP サーバーの共有状態

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

use crate::db::I18nDatabaseImpl;
use crate::input::source::SourceFile;
use crate::input::translation::Translation;

/// LSP サーバーの共有状態
///
/// `Backend` から状態管理の責務を分離し、ハンドラー間で共有可能にします。
///
/// # ロック順序
///
/// 複数のロックを同時に取得する場合は、以下の順序を厳守してください：
/// 1. `db`
/// 2. `source_files` / `translations` / `opened_files`
#[derive(Clone)]
pub struct ServerState {
    /// Salsa データベース
    pub db: Arc<Mutex<I18nDatabaseImpl>>,
    /// `SourceFile` 管理（ファイルパス → `SourceFile`）
    pub source_files: Arc<Mutex<HashMap<PathBuf, SourceFile>>>,
    /// 翻訳データ
    pub translations: Arc<Mutex<Vec<Translation>>>,
    /// 現在開いているファイルの URI
    pub opened_files: Arc<Mutex<HashSet<tower_lsp::lsp_types::Url>>>,
}

impl ServerState {
    /// 新しい `ServerState` を作成
    pub fn new(db: I18nDatabaseImpl) -> Self {
        Self {
            db: Arc::new(Mutex::new(db)),
            source_files: Arc::new(Mutex::new(HashMap::new())),
            translations: Arc::new(Mutex::new(Vec::new())),
            opened_files: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    /// `db` と `translations` のロックを一括取得
    ///
    /// ロック順序（`db` → `translations`）を保証します。
    pub async fn lock_db_and_translations(
        &self,
    ) -> (MutexGuard<'_, I18nDatabaseImpl>, MutexGuard<'_, Vec<Translation>>) {
        let db = self.db.lock().await;
        let translations = self.translations.lock().await;
        (db, translations)
    }

    /// `db` と `source_files` のロックを一括取得
    ///
    /// ロック順序（`db` → `source_files`）を保証します。
    pub async fn lock_db_and_source_files(
        &self,
    ) -> (MutexGuard<'_, I18nDatabaseImpl>, MutexGuard<'_, HashMap<PathBuf, SourceFile>>) {
        let db = self.db.lock().await;
        let source_files = self.source_files.lock().await;
        (db, source_files)
    }

    /// `db`, `source_files`, `translations` のロックを一括取得
    ///
    /// ロック順序（`db` → `source_files` → `translations`）を保証します。
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
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use googletest::prelude::*;

    use super::*;

    #[googletest::test]
    fn new_creates_empty_state() {
        let db = I18nDatabaseImpl::default();
        let state = ServerState::new(db);

        // tokio::test を使わずに同期的に確認
        // Arc のポインタが存在することを確認
        expect_that!(Arc::strong_count(&state.db), eq(1));
        expect_that!(Arc::strong_count(&state.source_files), eq(1));
        expect_that!(Arc::strong_count(&state.translations), eq(1));
        expect_that!(Arc::strong_count(&state.opened_files), eq(1));
    }

    #[googletest::test]
    fn clone_shares_state() {
        let db = I18nDatabaseImpl::default();
        let state1 = ServerState::new(db);
        let state2 = state1.clone();

        // Clone 後は Arc の参照カウントが 2 になる
        expect_that!(Arc::strong_count(&state1.db), eq(2));
        expect_that!(Arc::strong_count(&state1.source_files), eq(2));
        expect_that!(Arc::strong_count(&state1.translations), eq(2));
        expect_that!(Arc::strong_count(&state1.opened_files), eq(2));

        // 同じポインタを指していることを確認
        expect_that!(Arc::ptr_eq(&state1.db, &state2.db), eq(true));
        expect_that!(Arc::ptr_eq(&state1.source_files, &state2.source_files), eq(true));
    }

    #[googletest::test]
    fn debug_impl_works() {
        let db = I18nDatabaseImpl::default();
        let state = ServerState::new(db);

        let debug_str = format!("{:?}", state);

        // Debug 出力に主要なフィールド名が含まれていることを確認
        expect_that!(debug_str, contains_substring("ServerState"));
        expect_that!(debug_str, contains_substring("db"));
        expect_that!(debug_str, contains_substring("source_files"));
        expect_that!(debug_str, contains_substring("translations"));
        expect_that!(debug_str, contains_substring("opened_files"));
    }

    #[tokio::test]
    async fn state_can_be_modified_through_locks() {
        let db = I18nDatabaseImpl::default();
        let state = ServerState::new(db);

        // source_files に要素を追加
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

        // 追加した要素が取得できることを確認
        let source_files = state.source_files.lock().await;
        assert_eq!(source_files.len(), 1);
        assert!(source_files.contains_key(&PathBuf::from("/test.ts")));
    }

    #[tokio::test]
    async fn cloned_state_shares_modifications() {
        let db = I18nDatabaseImpl::default();
        let state1 = ServerState::new(db);
        let state2 = state1.clone();

        // state1 経由で opened_files に要素を追加
        {
            let mut opened_files = state1.opened_files.lock().await;
            let uri = tower_lsp::lsp_types::Url::parse("file:///test.ts").unwrap();
            opened_files.insert(uri);
        }

        // state2 経由でも変更が見えることを確認
        let opened_files = state2.opened_files.lock().await;
        assert_eq!(opened_files.len(), 1);
    }
}
