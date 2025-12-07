//! Entry point for the Language Server Protocol implementation.

use std::sync::Arc;

use js_i18n_language_server::{
    Backend,
    config::ConfigManager,
    db::I18nDatabaseImpl,
    indexer::workspace::WorkspaceIndexer,
};
use tokio::sync::Mutex;
use tower_lsp::{
    LspService,
    Server,
};
use tracing_appender::rolling::{
    RollingFileAppender,
    Rotation,
};

#[tokio::main]
async fn main() {
    // TODO: リリース前にログ設計を見直す
    let file_appender = RollingFileAppender::new(Rotation::DAILY, "/tmp/js_i18n_lsp", "lsp.log");
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "error".into()),
        )
        .with_writer(file_appender)
        .init();

    let config_manager = Arc::new(Mutex::new(ConfigManager::new()));
    let workspace_indexer = Arc::new(WorkspaceIndexer::new());
    let db = Arc::new(Mutex::new(I18nDatabaseImpl::default()));
    let source_files = Arc::new(Mutex::new(std::collections::HashMap::new()));
    let translations = Arc::new(Mutex::new(Vec::new()));
    let opened_files = Arc::new(Mutex::new(std::collections::HashSet::new()));

    let (stdin, stdout) = (tokio::io::stdin(), tokio::io::stdout());
    let (service, socket) = LspService::new(|client| Backend {
        client,
        config_manager,
        workspace_indexer,
        db,
        source_files,
        translations,
        opened_files,
    });
    Server::new(stdin, stdout, socket).serve(service).await;
}
