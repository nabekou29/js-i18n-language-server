//! ワークスペースインデックスのベンチマーク
//!
//! 使用方法:
//! ```
//! cargo run --release --example bench_indexing -- /path/to/workspace
//! ```

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use js_i18n_language_server::config::ConfigManager;
use js_i18n_language_server::db::I18nDatabaseImpl;
use js_i18n_language_server::indexer::workspace::WorkspaceIndexer;
use js_i18n_language_server::input::source::SourceFile;
use js_i18n_language_server::input::translation::Translation;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() {
    // tracing を初期化（INFO レベル）
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    let args: Vec<String> = std::env::args().collect();
    let workspace_path = args
        .get(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp/bench_indexing/test_project"));

    if !workspace_path.exists() {
        eprintln!("Error: Workspace path does not exist: {}", workspace_path.display());
        eprintln!("Generate test data first:");
        eprintln!("  /tmp/bench_indexing/generate_test_data.sh 500 10 100");
        std::process::exit(1);
    }

    println!("=== Workspace Indexing Benchmark ===");
    println!("Workspace: {}", workspace_path.display());
    println!();

    // 複数回実行して平均を取る
    let iterations = 3;
    let mut times = Vec::new();

    for i in 1..=iterations {
        println!("--- Iteration {}/{} ---", i, iterations);
        let elapsed = run_indexing(&workspace_path).await;
        times.push(elapsed);
        println!();
    }

    println!("=== Results ===");
    for (i, time) in times.iter().enumerate() {
        println!("  Run {}: {}ms", i + 1, time);
    }
    let avg = times.iter().sum::<u128>() / times.len() as u128;
    let min = *times.iter().min().unwrap_or(&0);
    let max = *times.iter().max().unwrap_or(&0);
    println!("  Average: {}ms", avg);
    println!("  Min: {}ms, Max: {}ms", min, max);
}

async fn run_indexing(workspace_path: &PathBuf) -> u128 {
    let start = std::time::Instant::now();

    // 必要なコンポーネントを初期化
    let db = I18nDatabaseImpl::default();
    let indexer = WorkspaceIndexer::new();
    let source_files: Arc<Mutex<HashMap<PathBuf, SourceFile>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let translations: Arc<Mutex<Vec<Translation>>> = Arc::new(Mutex::new(Vec::new()));

    // ConfigManager を作成して設定を読み込み
    let mut config_manager = ConfigManager::new();
    let _ = config_manager.load_settings(Some(workspace_path.clone()));

    // インデックスを実行
    let result = indexer
        .index_workspace(
            db,
            workspace_path,
            &config_manager,
            source_files.clone(),
            translations.clone(),
            None::<fn(u32, u32)>,
        )
        .await;

    let elapsed = start.elapsed().as_millis();

    match result {
        Ok(()) => {
            let source_count = source_files.lock().await.len();
            let translation_count = translations.lock().await.len();
            println!(
                "  Indexed {} source files, {} translation files in {}ms",
                source_count, translation_count, elapsed
            );
        }
        Err(e) => {
            eprintln!("  Error: {:?}", e);
        }
    }

    elapsed
}
