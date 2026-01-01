//! IDE 機能を提供するモジュール

pub mod backend;
pub mod code_actions;
pub mod completion;
pub mod diagnostics;
pub mod goto_definition;
mod handlers;
pub mod hover;
pub mod references;
pub mod state;
pub mod virtual_text;
