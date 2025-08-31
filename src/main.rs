//! Entry point for the Language Server Protocol implementation.

use js_i18n_language_server::Backend;
use tower_lsp::{
    LspService,
    Server,
};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();

    let (stdin, stdout) = (tokio::io::stdin(), tokio::io::stdout());
    let (service, socket) = LspService::new(|client| Backend { client });
    Server::new(stdin, stdout, socket).serve(service).await;
}
