//! Key usage intermediate representation.

use crate::interned::TransKey;
use crate::types::SourceRange;

/// A key usage location in source code.
#[salsa::interned]
pub struct KeyUsage {
    pub key: TransKey<'db>,
    pub range: SourceRange,
    pub namespace: Option<String>,
    pub namespaces: Option<Vec<String>>,
}
