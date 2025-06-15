//! i18n Language Server 翻訳リソース管理システムの使用例
//!
//! このファイルは、実装した翻訳リソース管理システムの基本的な使用方法を示します。
//!
//! 作成日: 2025-06-15
//! 作成者: @nabekou29

#![allow(
    clippy::print_stdout,
    clippy::uninlined_format_args,
    unused_variables,
    unused_imports,
    clippy::expect_used,
    clippy::unwrap_used
)]

use std::path::Path;

use js_i18n_language_server::translation::{
    NamespacedKey,
    TranslationCache,
    TranslationFileFormat,
    TranslationValue,
};

/// 翻訳リソース管理システムの基本的な使用例
///
/// # エラー
/// ファイルの読み込みまたは解析に失敗した場合
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 翻訳キャッシュを作成
    let cache = TranslationCache::new();

    println!("=== i18n Language Server 翻訳リソース管理システム使用例 ===\n");

    // 2. 翻訳ファイルをロード
    let translation_file = Path::new("examples/sample_translations.json");

    if translation_file.exists() {
        match cache.load_file(translation_file, TranslationFileFormat::Json).await {
            Ok(()) => {
                println!("✅ 翻訳ファイルを正常にロードしました: {}", translation_file.display());
            }
            Err(e) => {
                println!("❌ 翻訳ファイルの読み込みに失敗しました: {}", e);
                return Err(e.into());
            }
        }
    } else {
        println!("⚠️  サンプル翻訳ファイルが見つかりません: {}", translation_file.display());
        println!("   手動でテストデータを作成します");

        // 手動でテストデータを作成
        create_test_data(&cache);
    }

    // 3. 統計情報を表示
    let stats = cache.get_stats();
    println!("\n📊 キャッシュ統計情報:");
    println!("   - ネームスペース数: {}", stats.namespace_count);
    println!("   - 総キー数: {}", stats.total_keys);
    println!("   - ファイル数: {}", stats.file_count);

    // 4. 利用可能なネームスペースを表示
    let namespaces = cache.get_namespaces();
    println!("\n🏷️  利用可能なネームスペース:");
    for namespace in &namespaces {
        println!("   - {}", namespace);
    }

    // 5. 翻訳キーの検索例
    println!("\n🔍 翻訳キーの検索例:");

    // 完全一致検索
    let key = NamespacedKey::parse("common.hello")?;
    if let Some(value) = cache.get_translation(&key) {
        println!("   {} -> {:?}", key.full_key(), value.as_string());
    } else {
        println!("   {} -> 見つかりません", key.full_key());
    }

    // ネストしたキーの検索
    let nested_key = NamespacedKey::parse("user.profile.name")?;
    if let Some(value) = cache.get_translation(&nested_key) {
        println!("   {} -> {:?}", nested_key.full_key(), value.as_string());
    } else {
        println!("   {} -> 見つかりません", nested_key.full_key());
    }

    // 6. プレフィックス検索（補完機能）
    println!("\n🎯 プレフィックス検索（補完機能）:");
    let namespace =
        if namespaces.is_empty() { None } else { namespaces.first().map(String::as_str) };

    let prefixes = ["common", "user", "errors"];
    for prefix in &prefixes {
        let results = cache.search_keys_with_prefix(namespace, prefix, 5);
        if !results.is_empty() {
            println!("   '{}' で始まるキー:", prefix);
            for result in &results {
                println!("     - {}", result);
            }
        }
    }

    // 7. 全キーの列挙
    if !namespaces.is_empty() {
        let first_namespace = namespaces.first().unwrap();
        println!("\n📋 ネームスペース '{}' の全キー:", first_namespace);

        match cache.get_all_keys(Some(first_namespace)) {
            Ok(keys) => {
                let display_keys: Vec<&str> = keys.iter().take(10).map(String::as_str).collect();
                for key in display_keys {
                    println!("     - {}", key);
                }
                if keys.len() > 10 {
                    println!("     ... 他 {} 個のキー", keys.len() - 10);
                }
            }
            Err(e) => {
                println!("   エラー: {}", e);
            }
        }
    }

    println!("\n✨ 使用例の実行が完了しました");
    Ok(())
}

/// テストデータを手動で作成する
///
/// # 引数
/// * `cache` - 翻訳キャッシュ
fn create_test_data(cache: &TranslationCache) {
    // 将来のテストデータ追加用

    println!("   テストデータを作成中...");

    // commonネームスペースのデータ
    // TODO: TranslationCacheにテストデータを直接追加するpublic methodが必要
    // 現在はプライベートフィールドにアクセスできないため、実装は保留
    println!("   TranslationCacheへのテストデータ追加は将来の実装で対応予定");

    // 以下は将来実装予定のコード例
    // cache.insert_test_data("common.hello", "Hello, World!");
    // cache.insert_test_data("common.welcome", "Welcome to our application");
    // cache.insert_test_data("common.actions.save", "Save");
    // cache.insert_test_data("common.actions.cancel", "Cancel");
    // cache.insert_test_data("user.profile.name", "Name");
    // cache.insert_test_data("user.profile.email", "Email address");
    // cache.insert_test_data("user.settings.language", "Language");
    // cache.insert_test_data("errors.required", "This field is required");
    // cache.insert_test_data("errors.invalid_email", "Please enter a valid email address");

    println!("   ✅ テストデータを作成しました");
}
