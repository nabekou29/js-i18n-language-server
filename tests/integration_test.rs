//! i18n analyzer統合テスト
//!
//! 実際のJavaScript/TypeScriptファイルを使用して
//! analyzer機能をテストします。

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::fs;
use std::path::PathBuf;

use js_i18n_language_server::analyzer::I18nAnalyzer;

fn fixture_path(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test").join("fixtures").join(path)
}

#[test]
fn test_simple_javascript_t_calls() {
    let mut analyzer = I18nAnalyzer::new().expect("Failed to create analyzer");
    let file_path = fixture_path("javascript/simple_t.js");
    let content = fs::read_to_string(&file_path).expect("Failed to read fixture file");

    let calls = analyzer.analyze_file(&content, "js").expect("Failed to analyze file");

    // 基本的な検証
    assert!(!calls.is_empty(), "No translation calls found");

    // 期待される翻訳キーが含まれているか確認
    let keys: Vec<String> = calls.iter().map(|c| c.key.key.clone()).collect();
    assert!(keys.contains(&"simple.key".to_string()), "simple.key not found");
    assert!(keys.contains(&"nested.path.to.key".to_string()), "nested.path.to.key not found");
    assert!(keys.contains(&"with.i18n.object".to_string()), "with.i18n.object not found");
    assert!(keys.contains(&"common.greeting".to_string()), "common.greeting not found");
    assert!(keys.contains(&"common.farewell".to_string()), "common.farewell not found");
    assert!(keys.contains(&"errors.notFound".to_string()), "errors.notFound not found");

    // 関数名の検証
    assert!(calls.iter().any(|c| c.function_name == "t"), "No t() calls found");
    assert!(calls.iter().any(|c| c.function_name == "i18n.t"), "No i18n.t() calls found");
}

#[test]
fn test_typescript_translations() {
    let mut analyzer = I18nAnalyzer::new().expect("Failed to create analyzer");
    let file_path = fixture_path("typescript/typed_translations.ts");
    let content = fs::read_to_string(&file_path).expect("Failed to read fixture file");

    let calls = analyzer.analyze_file(&content, "ts").expect("Failed to analyze file");

    // TypeScriptファイルでも翻訳が検出されることを確認
    assert!(!calls.is_empty(), "No translation calls found in TypeScript");

    // 期待される翻訳キーが含まれているか確認
    // 現在の実装では文字列リテラルのみ検出されるので、user.greetingのみが期待される
    assert!(calls.iter().any(|c| c.key.key == "user.greeting"), "user.greeting not found");
    // TODO: 将来的には変数経由の翻訳キーも検出できるようにする
}

#[test]
fn test_react_component_with_hook() {
    let mut analyzer = I18nAnalyzer::new().expect("Failed to create analyzer");
    let file_path = fixture_path("react/use_translation.jsx");
    let content = fs::read_to_string(&file_path).expect("Failed to read fixture file");

    let calls = analyzer.analyze_file(&content, "jsx").expect("Failed to analyze file");

    // React Hook使用時も翻訳が検出されることを確認
    assert!(!calls.is_empty(), "No translation calls found in React component");

    // 期待される翻訳キーが含まれているか確認
    let keys: Vec<String> = calls.iter().map(|c| c.key.key.clone()).collect();
    assert!(keys.contains(&"page.title".to_string()), "page.title not found");
    assert!(keys.contains(&"page.description".to_string()), "page.description not found");
    assert!(keys.contains(&"button.submit".to_string()), "button.submit not found");
    assert!(keys.contains(&"title".to_string()), "title not found");
    assert!(keys.contains(&"intro".to_string()), "intro not found");
}

#[test]
fn test_position_information() {
    let mut analyzer = I18nAnalyzer::new().expect("Failed to create analyzer");
    let content = r#"
import { t } from "i18next";

const msg = t("test.key");
"#;

    let calls = analyzer.analyze_file(content, "js").expect("Failed to analyze file");

    assert_eq!(calls.len(), 1);
    let call = &calls[0];

    // 位置情報が正しく設定されているか確認
    assert_eq!(call.key.key, "test.key");
    assert_eq!(call.start_line, 4); // 1-indexed
    assert!(call.start_column > 0);
    assert!(call.end_column > call.start_column);
}
