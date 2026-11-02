pub mod ast;
pub mod diagnostics;
pub mod lexer;
pub mod parser;
use crate::{lexer::Token, parser::parse_tokens};
use logos::Logos;

fn main() {
    let source = r"
fn extern_fn(arg: MyStruct, num: i32) -> bool;
fn other_fn();

struct MyStruct {
    a: bool,
    b: f32,
    c: i32
}

public fn main() -> i32 {
    let my_struct: MyStruct = MyStruct {
        a: true,
        b: 3.14,
        c: 8
    };

    let ret: bool = extern_fn(my_struct, 5);
    if ret {
        other_fn();
    }

    return 0;
}
";
    let lex = Token::lexer(source).spanned();

    let mut tokens = Vec::new();
    let mut err = false;
    for (token, span) in lex {
        if let Ok(token) = token {
            tokens.push((token, span));
        } else {
            err = true;
            println!("Error, unrecognized token: `{}`", &source[span]);
        }
    }

    if err {
        return;
    }

    let parsed_file = parse_tokens(tokens, source);
}
