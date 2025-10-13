/// インターン化された翻訳キー
#[salsa::interned]
pub struct TransKey {
    #[returns(ref)]
    pub text: String,
}
