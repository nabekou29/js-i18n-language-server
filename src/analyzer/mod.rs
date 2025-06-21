//! i18n分析機能を提供するモジュール
//!
//! このモジュールは、JavaScript/TypeScriptコードから国際化関連の情報を抽出し、
//! 分析する機能を提供します。

pub mod extractor;
pub mod i18n_analyzer;
pub mod query_loader;
pub mod types;

pub use i18n_analyzer::I18nAnalyzer;
pub use query_loader::{
    QueryLoader,
    QuerySet,
};
pub use types::{
    TranslationCall,
    TranslationKey,
};
