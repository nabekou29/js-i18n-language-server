use thiserror::Error;

/// Defines errors that may occur during the analysis process
#[derive(Error, Debug)]
pub enum AnalyzerError {
    /// Error when failing to set the language for the parser
    #[error("Failed to set language for parser: {0}")]
    LanguageSetup(#[from] tree_sitter::LanguageError),
    /// Error when failing to parse source code
    #[error("Failed to parse source code")]
    ParseFailed,
    /// Error when failing to execute a query
    #[error("Query execution failed: {0}")]
    QueryExecution(String),
}
