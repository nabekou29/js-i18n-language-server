//! Entry point for the Language Server Protocol implementation.

use std::path::PathBuf;
use std::sync::Arc;

use js_i18n_language_server::{
    Backend,
    ServerState,
    config::ConfigManager,
    db::I18nDatabaseImpl,
    indexer::workspace::WorkspaceIndexer,
};
use tokio::sync::Mutex;
use tower_lsp::{
    LspService,
    Server,
};

/// Command-line arguments.
struct Args {
    log_file: Option<PathBuf>,
}

fn parse_args() -> Args {
    let mut args = Args { log_file: None };
    let mut args_iter = std::env::args().skip(1);

    while let Some(arg) = args_iter.next() {
        match arg.as_str() {
            "--log-file" => {
                args.log_file = args_iter.next().map(PathBuf::from);
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            "--version" | "-V" => {
                print_version();
                std::process::exit(0);
            }
            _ => {
                // Ignore unknown args (LSP clients may pass additional arguments)
            }
        }
    }

    args
}

#[allow(clippy::print_stdout)]
fn print_help() {
    println!(
        r"Language Server Protocol implementation for JavaScript/TypeScript i18n

Usage: js-i18n-language-server [OPTIONS]

Options:
      --log-file <PATH>  Log to the specified file instead of stderr
  -h, --help             Print help
  -V, --version          Print version

Environment Variables:
  RUST_LOG              Log level (e.g., js_i18n_language_server=debug)
  JS_I18N_LOG_FILE      Log file path (takes priority over --log-file)"
    );
}

#[allow(clippy::print_stdout)]
fn print_version() {
    println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
}

/// Initializes the logging system.
///
/// Priority: `JS_I18N_LOG_FILE` env var > `--log-file` arg > stderr (default).
fn init_logging(log_file_arg: Option<PathBuf>) {
    use std::fs::File;

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "js_i18n_language_server=warn".into());

    let log_file_path = std::env::var("JS_I18N_LOG_FILE").ok().map(PathBuf::from).or(log_file_arg);
    let file_writer = log_file_path.and_then(|path| File::create(&path).ok());

    if let Some(file) = file_writer {
        tracing_subscriber::fmt()
            .with_env_filter(env_filter)
            .with_writer(file)
            .with_ansi(false)
            .init();
    } else {
        tracing_subscriber::fmt().with_env_filter(env_filter).with_writer(std::io::stderr).init();
    }
}

#[tokio::main]
async fn main() {
    let args = parse_args();
    init_logging(args.log_file);

    let config_manager = Arc::new(Mutex::new(ConfigManager::new()));
    let workspace_indexer = Arc::new(WorkspaceIndexer::new());
    let state = ServerState::new(I18nDatabaseImpl::default());

    let (stdin, stdout) = (tokio::io::stdin(), tokio::io::stdout());
    let (service, socket) =
        LspService::new(|client| Backend { client, config_manager, workspace_indexer, state });
    Server::new(stdin, stdout, socket).serve(service).await;
}
