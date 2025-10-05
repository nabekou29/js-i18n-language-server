//! Entry point for the Language Server Protocol implementation.

use std::sync::Arc;

use js_i18n_language_server::{
    Backend,
    config::ConfigManager,
    indexer::workspace::WorkspaceIndexer,
};
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
    let file_appender = RollingFileAppender::new(Rotation::DAILY, "/tmp/js_i18n_lsp", "lsp.log");
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "error".into()),
        )
        .with_writer(file_appender)
        .init();

    let config_manager = Arc::new(ConfigManager::new());
    let workspace_indexer = Arc::new(WorkspaceIndexer::new());

    let (stdin, stdout) = (tokio::io::stdin(), tokio::io::stdout());
    let (service, socket) =
        LspService::new(|client| Backend { client, config_manager, workspace_indexer });
    Server::new(stdin, stdout, socket).serve(service).await;
}
