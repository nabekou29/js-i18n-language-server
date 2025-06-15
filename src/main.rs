//! LSPサーバーのエントリーポイント
//!
//! このバイナリはLanguage Server Protocolを実装したサーバーを起動します。
//! 標準入出力を通じてクライアントと通信します。

use js_i18n_language_server::Backend;
use tower_lsp::{
    LspService,
    Server,
};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();

    let (stdin, stdout) = (tokio::io::stdin(), tokio::io::stdout());
    let (service, socket) = LspService::new(Backend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}
