//! ロケールファイル入力定義

#[salsa::input]
pub struct LocaleFile {
    /// ファイルのURI
    #[returns(ref)]
    pub uri: String,

    /// ファイルの内容
    #[returns(ref)]
    pub text: String,
}
