//! Source file input definitions.

use std::path::Path;

#[salsa::input]
pub struct SourceFile {
    #[returns(ref)]
    pub uri: String,

    #[returns(ref)]
    pub text: String,

    pub language: ProgrammingLanguage,
}

/// Supported programming languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProgrammingLanguage {
    JavaScript,
    Jsx,
    TypeScript,
    Tsx,
}

impl ProgrammingLanguage {
    /// Infers the programming language from file extension.
    #[must_use]
    pub fn from_uri(uri: &str) -> Option<Self> {
        let file_path = Path::new(uri);
        match file_path.extension().and_then(|ext| ext.to_str()) {
            Some("tsx") => Some(Self::Tsx),
            Some("ts") => Some(Self::TypeScript),
            Some("jsx") => Some(Self::Jsx),
            Some("js") => Some(Self::JavaScript),
            _ => None,
        }
    }

    #[must_use]
    pub fn tree_sitter_language(&self) -> tree_sitter::Language {
        match self {
            Self::JavaScript | Self::Jsx => tree_sitter_javascript::LANGUAGE.into(),
            Self::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Self::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::expect_used, clippy::panic)]
mod tests {
    use rstest::*;

    use super::*;

    #[rstest]
    #[case::tsx("file.tsx", Some(ProgrammingLanguage::Tsx))]
    #[case::ts("file.ts", Some(ProgrammingLanguage::TypeScript))]
    #[case::jsx("file.jsx", Some(ProgrammingLanguage::Jsx))]
    #[case::js("file.js", Some(ProgrammingLanguage::JavaScript))]
    #[case::multiple_dots("file.config.ts", Some(ProgrammingLanguage::TypeScript))]
    #[case::json("file.json", None)]
    #[case::no_ext("file", None)]
    #[case::unknown_ext("file.txt", None)]
    fn test_from_uri(#[case] uri: &str, #[case] expected: Option<ProgrammingLanguage>) {
        let lang = ProgrammingLanguage::from_uri(uri);
        assert_eq!(lang, expected);
    }
}
