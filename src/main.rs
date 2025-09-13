//! Entry point for the Language Server Protocol implementation.

use std::sync::Arc;

use js_i18n_language_server::{
    Backend,
    config::ConfigManager,
};
use tower_lsp::{
    LspService,
    Server,
};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();
    let config_manager = Arc::new(ConfigManager::new());

    let (stdin, stdout) = (tokio::io::stdin(), tokio::io::stdout());
    let (service, socket) = LspService::new(|client| Backend { client, config_manager });
    Server::new(stdin, stdout, socket).serve(service).await;
}
