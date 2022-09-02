pub type Token = (String, String);
pub type Tokens = Vec<Token>;
pub struct LineInfo {
    pub tokens: Tokens,
    pub is_complete: bool,
    pub unmatched: String
}
