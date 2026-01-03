//! Load Tree-sitter queries from files.
//!
//! クエリのパースはコストが高いため、言語ごとに1回だけパースし、
//! 以降はキャッシュを使用する。

use std::sync::OnceLock;

use tree_sitter::Query;

use crate::input::source::ProgrammingLanguage;

/// クエリファイルの定義
struct QueryFile {
    /// クエリファイルの内容
    content: &'static str,
    /// クエリの説明（デバッグ用）
    name: &'static str,
}

/// JavaScript/JSX 用のクエリファイル
const JS_QUERIES: &[QueryFile] = &[
    QueryFile {
        content: include_str!("../../../queries/javascript/react-i18next.scm"),
        name: "react-i18next",
    },
    QueryFile { content: include_str!("../../../queries/javascript/i18next.scm"), name: "i18next" },
    QueryFile {
        content: include_str!("../../../queries/javascript/next-intl.scm"),
        name: "next-intl",
    },
];

/// TypeScript 用のクエリファイル
const TS_QUERIES: &[QueryFile] = &[
    QueryFile {
        content: include_str!("../../../queries/typescript/react-i18next.scm"),
        name: "react-i18next",
    },
    QueryFile { content: include_str!("../../../queries/typescript/i18next.scm"), name: "i18next" },
    QueryFile {
        content: include_str!("../../../queries/typescript/next-intl.scm"),
        name: "next-intl",
    },
];

/// TSX 用のクエリファイル
const TSX_QUERIES: &[QueryFile] = &[
    QueryFile {
        content: include_str!("../../../queries/tsx/react-i18next.scm"),
        name: "react-i18next",
    },
    QueryFile { content: include_str!("../../../queries/tsx/i18next.scm"), name: "i18next" },
    QueryFile { content: include_str!("../../../queries/tsx/next-intl.scm"), name: "next-intl" },
];

// === 言語別クエリキャッシュ ===
// Query は Sync + Send なので OnceLock で安全にキャッシュできる
static JS_QUERY_CACHE: OnceLock<Vec<Query>> = OnceLock::new();
static TS_QUERY_CACHE: OnceLock<Vec<Query>> = OnceLock::new();
static TSX_QUERY_CACHE: OnceLock<Vec<Query>> = OnceLock::new();

/// 指定した言語用のクエリファイル群をパースする（内部関数）
fn parse_queries(language: ProgrammingLanguage) -> Vec<Query> {
    let tree_sitter_lang = language.tree_sitter_language();

    let query_files = match language {
        ProgrammingLanguage::JavaScript | ProgrammingLanguage::Jsx => JS_QUERIES,
        ProgrammingLanguage::TypeScript => TS_QUERIES,
        ProgrammingLanguage::Tsx => TSX_QUERIES,
    };

    query_files
        .iter()
        .filter_map(|qf| {
            Query::new(&tree_sitter_lang, qf.content)
                .map_err(|e| tracing::error!("Failed to parse {} query: {e:?}", qf.name))
                .ok()
        })
        .collect()
}

/// クエリをロード（キャッシュ付き）
///
/// クエリのパースは言語ごとに1回だけ行われ、以降はキャッシュを使用する。
/// これにより、大量のファイルを処理する際のパフォーマンスが大幅に向上する。
///
/// # 注意
/// キャッシュされた `Query` への参照を返すため、寿命は `'static` となる。
#[must_use]
pub fn load_queries(language: ProgrammingLanguage) -> &'static [Query] {
    match language {
        ProgrammingLanguage::JavaScript | ProgrammingLanguage::Jsx => {
            JS_QUERY_CACHE.get_or_init(|| parse_queries(ProgrammingLanguage::JavaScript))
        }
        ProgrammingLanguage::TypeScript => {
            TS_QUERY_CACHE.get_or_init(|| parse_queries(ProgrammingLanguage::TypeScript))
        }
        ProgrammingLanguage::Tsx => {
            TSX_QUERY_CACHE.get_or_init(|| parse_queries(ProgrammingLanguage::Tsx))
        }
    }
}
