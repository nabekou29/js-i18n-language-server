//! LSPサーバーのエントリーポイント
//!
//! このバイナリはLanguage Server Protocolを実装したサーバーを起動します。
//! 標準入出力を通じてクライアントと通信します。

use rust_lsp_tutorial::Backend;
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
