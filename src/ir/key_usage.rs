//! キー使用箇所の中間表現

use crate::interned::TransKey;
use crate::types::SourceRange;

/// ソースコード内でのキー使用箇所
#[salsa::interned]
pub struct KeyUsage {
    /// キー名（インターン化）
    pub key: TransKey<'db>,

    /// ソースコード上の範囲
    pub range: SourceRange,
}
