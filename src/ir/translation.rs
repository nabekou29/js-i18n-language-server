//! 翻訳データの中間表現

use std::collections::HashMap;

use crate::interned::TransKey;

#[derive(Clone, PartialEq, Eq)]
pub struct Translation<'db> {
    /// キー名
    pub key: TransKey<'db>,

    /// ロケール → 翻訳値のマッピング
    pub values: HashMap<String, String>,
}
