//! ソースファイル入力定義

use std::path::Path;

/// ソースファイルの内容
#[salsa::input]
pub struct SourceFile {
    /// ファイルのURI
    #[returns(ref)]
    pub uri: String,

    /// ファイルの内容
    #[returns(ref)]
    pub text: String,

    /// 言語
    pub language: ProgrammingLanguage,
}

/// サポートされるプログラミング言語
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProgrammingLanguage {
    JavaScript,
    Jsx,
    TypeScript,
    Tsx,
}

impl ProgrammingLanguage {
    /// ファイル URI から言語を推論
    #[must_use]
    pub fn from_uri(uri: &str) -> Self {
        let file_path = Path::new(uri);
        match file_path.extension().and_then(|ext| ext.to_str()) {
            Some("tsx") => Self::Tsx,
            Some("ts") => Self::TypeScript,
            Some("jsx") => Self::Jsx,
            _ => Self::JavaScript,
        }
    }

    /// Tree-sitter の Language を取得
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
    #[case::tsx("file.tsx", ProgrammingLanguage::Tsx)]
    #[case::ts("file.ts", ProgrammingLanguage::TypeScript)]
    #[case::jsx("file.jsx", ProgrammingLanguage::Jsx)]
    #[case::js("file.js", ProgrammingLanguage::JavaScript)]
    #[case::no_ext("file", ProgrammingLanguage::JavaScript)]
    #[case::multiple_dots("file.config.ts", ProgrammingLanguage::TypeScript)]
    fn test_from_uri(#[case] uri: &str, #[case] expected: ProgrammingLanguage) {
        let lang = ProgrammingLanguage::from_uri(uri);
        assert_eq!(lang, expected);
    }
}
