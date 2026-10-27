use logos::Logos;

#[derive(Logos, Debug, PartialEq)]
#[logos(skip r"[ \t\n\f]+")]
#[logos(skip(r"//[^\n]*", allow_greedy = true))] // comments
#[logos(skip(r"/\*(?:[^*]|\*[^/])*\*/", allow_greedy = true))] /* comments */
pub enum Token {
    #[token("i8")]
    I8,
    #[token("i16")]
    I16,
    #[token("i32")]
    I32,
    #[token("i64")]
    I64,
    #[token("u8")]
    U8,
    #[token("u16")]
    U16,
    #[token("u32")]
    U32,
    #[token("u64")]
    U64,
    #[token("f32")]
    F32,
    #[token("f64")]
    F64,
    #[token("char")]
    Char,
    #[token("bool")]
    Bool,

    #[token("if")]
    If,
    #[token("else")]
    Else,
    #[token("while")]
    While,
    #[token("return")]
    Return,
    #[token("fn")]
    Fn,
    #[token("let")]
    Let,
    #[token("break")]
    Break,
    #[token("continue")]
    Continue,
    #[token("struct")]
    Struct,
    #[token("as")]
    As,

    #[token(";")]
    Semicolon,

    #[token(",")]
    Comma,

    #[regex(r"'(\.|[^'\\])*'")]
    CharLit,
    #[regex(r"(\d*[.])?\d+")]
    FloatLit,
    #[regex(r"(\d+)|(0x[0-9a-fA-F]+)|(0b[01]+)", priority = 3)]
    IntLit,
    #[regex("true|false")]
    BoolLiteral,
    #[regex(r#""(\.|[^"\\])*""#)]
    StringLiteral,
    // https://www.unicode.org/reports/tr31/
    #[regex(r"(\p{XID_Start}\p{XID_Continue}*)|(_\p{XID_Continue}+)", priority = 2)]
    Ident,

    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Asterisk,
    #[token("/")]
    Slash,
    #[token("==")]
    EqEq,
    #[token("!=")]
    NotEq,
    #[token(">=")]
    Gte,
    #[token(">")]
    Gt,
    #[token("<=")]
    Lte,
    #[token("<")]
    Lt,
    #[token("&&")]
    And,
    #[token("||")]
    Or,
    #[token("!")]
    Bang,
    #[token("&")]
    Ampersand,
    #[token("=")]
    Eq,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token(":")]
    Colon,
}
