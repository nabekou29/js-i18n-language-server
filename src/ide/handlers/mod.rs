//! LSP ハンドラーモジュール
//!
//! `LanguageServer` trait の各メソッドの実装を機能別に分割しています。
//!
//! このモジュールは `ide::backend` 内部でのみ使用され、外部には公開されません。

#![allow(unreachable_pub)]

pub mod code_action;
pub mod document_sync;
pub mod execute_command;
pub mod features;
pub mod lifecycle;
pub mod workspace;
