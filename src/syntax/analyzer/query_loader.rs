//! Load Tree-sitter queries from files.

use std::sync::OnceLock;

use tree_sitter::Query;

use crate::input::source::ProgrammingLanguage;

struct QueryFile {
    content: &'static str,
    name: &'static str,
}

// JS and TSX share the same query text (includes JSX patterns)
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

// TS queries omit JSX patterns
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

const SVELTE_I18N_QUERIES: &[QueryFile] =
    &[QueryFile { content: include_str!("../../../queries/svelte-i18n.scm"), name: "svelte-i18n" }];

static JS_QUERY_CACHE: OnceLock<Vec<Query>> = OnceLock::new();
static TS_QUERY_CACHE: OnceLock<Vec<Query>> = OnceLock::new();
static TSX_QUERY_CACHE: OnceLock<Vec<Query>> = OnceLock::new();
static SVELTE_QUERY_CACHE: OnceLock<Vec<Query>> = OnceLock::new();

fn parse_queries(language: ProgrammingLanguage) -> Vec<Query> {
    let tree_sitter_lang = language.tree_sitter_language();

    let base: &[QueryFile] = match language {
        ProgrammingLanguage::JavaScript | ProgrammingLanguage::Jsx | ProgrammingLanguage::Tsx => {
            JS_QUERIES
        }
        ProgrammingLanguage::TypeScript | ProgrammingLanguage::Svelte => TS_QUERIES,
    };

    let extra: &[QueryFile] = match language {
        ProgrammingLanguage::Svelte => SVELTE_I18N_QUERIES,
        _ => &[],
    };

    base.iter()
        .chain(extra)
        .filter_map(|qf| {
            Query::new(&tree_sitter_lang, qf.content)
                .map_err(|e| tracing::error!("Failed to parse {} query: {e:?}", qf.name))
                .ok()
        })
        .collect()
}

/// Loads cached queries for a language. Queries are parsed once per language.
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
        ProgrammingLanguage::Svelte => {
            SVELTE_QUERY_CACHE.get_or_init(|| parse_queries(ProgrammingLanguage::Svelte))
        }
    }
}
