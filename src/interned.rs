/// Interned translation key.
#[salsa::interned]
pub struct TransKey {
    #[returns(ref)]
    pub text: String,
}
