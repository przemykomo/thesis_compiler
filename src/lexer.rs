use std::fmt::Display;

use logos::Logos;

#[derive(Logos, Debug, PartialEq, Copy, Clone)]
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
    #[token("void")]
    Void,

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
    #[token("true")]
    True,
    #[token("false")]
    False,
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
    #[token("%")]
    Percent,

    #[token("+=")]
    PlusEq,
    #[token("-=")]
    MinusEq,
    #[token("*=")]
    AsteriskEq,
    #[token("/=")]
    SlashEq,
    #[token("%=")]
    PercentEq,

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
    #[token("->")]
    Arrow,
    #[token(".")]
    Period,
}

impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Token::I8 => "i8",
            Token::I16 => "i16",
            Token::I32 => "i32",
            Token::I64 => "i64",
            Token::U8 => "u8",
            Token::U16 => "u16",
            Token::U32 => "u32",
            Token::U64 => "u64",
            Token::F32 => "f32",
            Token::F64 => "f64",
            Token::Char => "char",
            Token::Bool => "bool",
            Token::Void => "void",
            Token::If => "if",
            Token::Else => "else",
            Token::While => "while",
            Token::Return => "return",
            Token::Fn => "fn",
            Token::Let => "let",
            Token::Break => "break",
            Token::Continue => "continue",
            Token::Struct => "struct",
            Token::As => "as",
            Token::Semicolon => ";",
            Token::Comma => ",",
            Token::CharLit => "a char literal",
            Token::FloatLit => "a float literal",
            Token::IntLit => "an integer literal",
            Token::True => "true",
            Token::False => "false",
            Token::StringLiteral => "a string literal",
            Token::Ident => "an indentifier",
            Token::Plus => "+",
            Token::Minus => "-",
            Token::Asterisk => "*",
            Token::Slash => "/",
            Token::Percent => "%",
            Token::PlusEq => "+=",
            Token::MinusEq => "-=",
            Token::AsteriskEq => "*=",
            Token::SlashEq => "/=",
            Token::PercentEq => "%=",
            Token::EqEq => "==",
            Token::NotEq => "!=",
            Token::Gte => ">=",
            Token::Gt => ">",
            Token::Lte => "<=",
            Token::Lt => "<",
            Token::And => "&&",
            Token::Or => "||",
            Token::Bang => "!",
            Token::Ampersand => "&",
            Token::Eq => "=",
            Token::LParen => "(",
            Token::RParen => ")",
            Token::LBrace => "{",
            Token::RBrace => "}",
            Token::LBracket => "[",
            Token::RBracket => "]",
            Token::Colon => ":",
            Token::Arrow => "->",
            Token::Period => ".",
        };

        write!(f, "{s}")
    }
}
