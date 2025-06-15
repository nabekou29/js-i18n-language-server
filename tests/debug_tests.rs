//! デバッグ用統合テスト
//!
//! 統合テストの問題を調査するためのデバッグテストを提供します。

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::print_stdout)]

use std::{
    fs,
    path::PathBuf,
};

use js_i18n_language_server::{
    analyzer::{
        FileIdManager,
        analyze_file,
    },
    query::{
        QueryExecutor,
        TranslationQueries,
        language_javascript,
    },
};
use tree_sitter::Parser;

/// テスト用のファイルパスを取得するヘルパー関数
fn get_fixture_path(relative_path: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("fixtures");
    path.push(relative_path);
    path
}

/// テストファイルの内容を読み込むヘルパー関数
fn read_fixture_file(relative_path: &str) -> Result<String, std::io::Error> {
    let path = get_fixture_path(relative_path);
    fs::read_to_string(path)
}

/// ASTの構造を再帰的に出力するヘルパー関数
fn print_ast_structure(node: &tree_sitter::Node<'_>, source_code: &str, depth: usize) {
    let indent = "  ".repeat(depth);
    let node_text = if node.byte_range().len() < 50 {
        &source_code[node.byte_range()]
    } else {
        &source_code[node.byte_range()][..47]
    };

    println!(
        "{}{}[{}]: '{}'",
        indent,
        node.kind(),
        node.byte_range().len(),
        node_text.replace('\n', "\\n")
    );

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            print_ast_structure(&child, source_code, depth + 1);
        }
    }
}

#[cfg(test)]
mod debug_tests {
    use super::*;

    #[test]
    fn debug_basic_query_execution() {
        // 基本的なクエリ実行をデバッグ
        let mut parser = Parser::new();
        let language = language_javascript();
        parser.set_language(language).expect("Failed to set language");

        let source_code = r"
        const message = t('hello.world');
        const greeting = i18n.t('greeting.message', { name: 'User' });
        ";

        let tree = parser.parse(source_code, None).expect("Failed to parse code");

        // クエリを直接実行
        let queries = TranslationQueries::new(language).expect("Failed to create queries");
        let mut executor = QueryExecutor::new();

        let references = executor
            .execute_query(queries.i18next_trans_f_call(), &tree, source_code)
            .expect("Failed to execute query");

        println!("Debug: Found {} references", references.len());
        for (i, ref_item) in references.iter().enumerate() {
            println!(
                "Reference {}: key='{}', function='{}', captures={:?}",
                i, ref_item.key, ref_item.function_name, ref_item.captures
            );
        }

        // ASTの構造を確認
        let root = tree.root_node();
        print_ast_structure(&root, source_code, 0);
    }

    #[test]
    fn debug_analyze_file_function() {
        // analyze_file関数の動作をデバッグ
        let file_manager = FileIdManager::new();
        let file_path = PathBuf::from("test.js");
        let file_id = file_manager.get_or_create_file_id(file_path.clone());

        let source_code = r"
        const message = t('hello.world');
        const greeting = i18n.t('greeting.message');
        ";

        println!("Debug: Analyzing simple JavaScript code");
        let result =
            analyze_file(file_id, &file_path, source_code).expect("Failed to analyze file");

        println!("Debug: Analysis result:");
        println!("  - File ID: {:?}", result.file_id);
        println!("  - References count: {}", result.references.len());
        println!("  - Scopes count: {}", result.scopes.len());
        println!("  - Errors count: {}", result.errors.len());

        for error in &result.errors {
            println!("  - Error: {error}");
        }

        for (i, reference) in result.references.iter().enumerate() {
            println!(
                "  - Reference {}: key='{}', function='{}', line={}, column={}",
                i, reference.key, reference.function_name, reference.line, reference.column
            );
        }

        for (i, scope) in result.scopes.iter().enumerate() {
            println!(
                "  - Scope {}: lines {}-{}, functions={:?}",
                i, scope.start_line, scope.end_line, scope.imported_functions
            );
        }
    }

    #[test]
    fn debug_fixture_file_analysis() {
        // フィクスチャファイルの解析をデバッグ
        let file_manager = FileIdManager::new();
        let file_path = get_fixture_path("javascript/basic_i18next.js");
        let file_id = file_manager.get_or_create_file_id(file_path.clone());

        let source_code = read_fixture_file("javascript/basic_i18next.js")
            .expect("Failed to read basic_i18next.js");

        println!("Debug: Source code length: {} characters", source_code.len());
        println!("Debug: First 200 characters: {}", &source_code[..source_code.len().min(200)]);

        let result = analyze_file(file_id, &file_path, &source_code)
            .expect("Failed to analyze basic_i18next.js");

        println!("Debug: Analysis result for basic_i18next.js:");
        println!("  - References count: {}", result.references.len());
        println!("  - Scopes count: {}", result.scopes.len());
        println!("  - Errors count: {}", result.errors.len());

        for error in &result.errors {
            println!("  - Error: {error}");
        }

        println!("Debug: All found references:");
        for (i, reference) in result.references.iter().enumerate() {
            println!(
                "  - Reference {}: key='{}', function='{}', line={}, column={}",
                i, reference.key, reference.function_name, reference.line, reference.column
            );
        }
    }

    #[test]
    fn debug_raw_query_on_fixture() {
        // フィクスチャファイルに対する生のクエリ実行をデバッグ
        let mut parser = Parser::new();
        let language = language_javascript();
        parser.set_language(language).expect("Failed to set language");

        let source_code = read_fixture_file("javascript/basic_i18next.js")
            .expect("Failed to read basic_i18next.js");

        let tree = parser.parse(&source_code, None).expect("Failed to parse code");

        // クエリを直接実行
        let queries = TranslationQueries::new(language).expect("Failed to create queries");
        let mut executor = QueryExecutor::new();

        println!("Debug: Executing i18next_trans_f_call query");
        let call_references = executor
            .execute_query(queries.i18next_trans_f_call(), &tree, &source_code)
            .expect("Failed to execute call query");

        println!("Debug: Found {} call references", call_references.len());
        for (i, ref_item) in call_references.iter().enumerate() {
            println!(
                "Call Reference {}: key='{}', function='{}', captures={:?}",
                i, ref_item.key, ref_item.function_name, ref_item.captures
            );
        }

        println!("Debug: Executing i18next_trans_f query");
        let hook_references = executor
            .execute_query(queries.i18next_trans_f(), &tree, &source_code)
            .expect("Failed to execute hook query");

        println!("Debug: Found {} hook references", hook_references.len());
        for (i, ref_item) in hook_references.iter().enumerate() {
            println!(
                "Hook Reference {}: key='{}', function='{}', captures={:?}",
                i, ref_item.key, ref_item.function_name, ref_item.captures
            );
        }

        println!("Debug: Executing import_statements query");
        let import_references = executor
            .execute_query(queries.import_statements(), &tree, &source_code)
            .expect("Failed to execute import query");

        println!("Debug: Found {} import references", import_references.len());
        for (i, ref_item) in import_references.iter().enumerate() {
            println!(
                "Import Reference {}: key='{}', function='{}', captures={:?}",
                i, ref_item.key, ref_item.function_name, ref_item.captures
            );
        }
    }

    #[test]
    fn debug_typescript_vs_javascript() {
        // TypeScriptとJavaScriptパーサーの違いを確認
        use js_i18n_language_server::query::language_typescript;

        let source_code = r"
        import { useTranslation } from 'react-i18next';
        const { t } = useTranslation();
        const message = t('test.key');
        ";

        // JavaScript パーサー
        let mut js_parser = Parser::new();
        let js_language = language_javascript();
        js_parser.set_language(js_language).expect("Failed to set JS language");
        let js_tree = js_parser.parse(source_code, None).expect("Failed to parse with JS");

        // TypeScript パーサー
        let mut ts_parser = Parser::new();
        let ts_language = language_typescript();
        ts_parser.set_language(ts_language).expect("Failed to set TS language");
        let ts_tree = ts_parser.parse(source_code, None).expect("Failed to parse with TS");

        println!("Debug: JavaScript parser root: {}", js_tree.root_node().kind());
        println!("Debug: TypeScript parser root: {}", ts_tree.root_node().kind());

        // クエリ実行の比較
        let js_queries = TranslationQueries::new(js_language).expect("Failed to create JS queries");
        let ts_queries = TranslationQueries::new(ts_language).expect("Failed to create TS queries");

        let mut executor = QueryExecutor::new();

        let js_refs = executor
            .execute_query(js_queries.i18next_trans_f_call(), &js_tree, source_code)
            .expect("Failed to execute JS query");

        let ts_refs = executor
            .execute_query(ts_queries.i18next_trans_f_call(), &ts_tree, source_code)
            .expect("Failed to execute TS query");

        println!("Debug: JavaScript parser found {} references", js_refs.len());
        println!("Debug: TypeScript parser found {} references", ts_refs.len());

        for (i, ref_item) in js_refs.iter().enumerate() {
            println!(
                "JS Reference {}: key='{}', function='{}'",
                i, ref_item.key, ref_item.function_name
            );
        }

        for (i, ref_item) in ts_refs.iter().enumerate() {
            println!(
                "TS Reference {}: key='{}', function='{}'",
                i, ref_item.key, ref_item.function_name
            );
        }
    }
}
