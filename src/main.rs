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
    log_level: Option<String>,
}

fn parse_args() -> Args {
    let mut args = Args { log_file: None, log_level: None };
    let mut args_iter = std::env::args().skip(1);

    while let Some(arg) = args_iter.next() {
        match arg.as_str() {
            "--log-file" => {
                args.log_file = args_iter.next().map(PathBuf::from);
            }
            "--log-level" => {
                args.log_level = args_iter.next();
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
      --log-file <PATH>    Log to the specified file instead of stderr
      --log-level <LEVEL>  Log level (e.g., info, debug, or js_i18n_language_server=info)
  -h, --help               Print help
  -V, --version            Print version

Environment Variables:
  JS_I18N_LOG           Log level filter (--log-level takes priority)
  RUST_LOG              Fallback log level filter
  JS_I18N_LOG_FILE      Log file path (takes priority over --log-file)"
    );
}

#[allow(clippy::print_stdout)]
fn print_version() {
    println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
}

/// Resolves the tracing env filter.
///
/// Priority: `--log-level` arg > `JS_I18N_LOG` env > `RUST_LOG` env > default (`warn`).
/// Simple values (e.g., `info`) are auto-scoped to `js_i18n_language_server={value}`.
fn resolve_env_filter(log_level_arg: Option<&str>) -> tracing_subscriber::EnvFilter {
    let raw = log_level_arg.map(String::from).or_else(|| std::env::var("JS_I18N_LOG").ok());

    raw.map_or_else(
        || {
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "js_i18n_language_server=warn".into())
        },
        |value| {
            let filter = if value.contains('=') {
                value
            } else {
                format!("js_i18n_language_server={value}")
            };
            filter.parse().unwrap_or_else(|_| "js_i18n_language_server=warn".into())
        },
    )
}

/// Initializes the logging system.
///
/// Log file priority: `JS_I18N_LOG_FILE` env var > `--log-file` arg > stderr (default).
fn init_logging(args: &Args) {
    use std::fs::File;

    let env_filter = resolve_env_filter(args.log_level.as_deref());

    let log_file_path =
        std::env::var("JS_I18N_LOG_FILE").ok().map(PathBuf::from).or_else(|| args.log_file.clone());
    let file_writer = log_file_path.and_then(|path| File::create(&path).ok());

    if let Some(file) = file_writer {
        tracing_subscriber::fmt()
            .with_env_filter(env_filter)
            .with_writer(file)
            .with_ansi(false)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(env_filter)
            .with_writer(std::io::stderr)
            .with_ansi(false)
            .init();
    }
}

#[tokio::main]
async fn main() {
    let args = parse_args();
    init_logging(&args);

    let config_manager = Arc::new(Mutex::new(ConfigManager::new()));
    let workspace_indexer = Arc::new(WorkspaceIndexer::new());
    let state = ServerState::new(I18nDatabaseImpl::default());

    let (stdin, stdout) = (tokio::io::stdin(), tokio::io::stdout());
    let (service, socket) =
        LspService::new(|client| Backend { client, config_manager, workspace_indexer, state });
    Server::new(stdin, stdout, socket).serve(service).await;
}
