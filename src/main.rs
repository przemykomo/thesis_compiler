pub mod ast;
pub mod diagnostics;
pub mod lexer;
pub mod parser;
use crate::{diagnostics::Diagnostic::*, lexer::Token, parser::parse_tokens};
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

fn main() -> i32 {
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

    let mut lines = vec![0];
    lines.extend(
        source
            .char_indices()
            .filter(|(_, c)| *c == '\n')
            .map(|(i, _)| i + 1),
    );
    lines.push(source.len());

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

    for diagnostic in parsed_file.diagnostics {
        match diagnostic {
            UnexpectedToken {
                unexpected,
                range,
                expected,
            } => {
                let line = lines.partition_point(|&l| l < range.start);
                println!(
                    "Unexpected token: {}, expected {:#?}. line {}: `{}`\n{}",
                    unexpected,
                    expected,
                    line - 1,
                    &source[range],
                    &source[lines[line - 1]..lines[line]]
                );
            }
            UnexpectedToken2 {
                unexpected,
                range,
                expected,
            } => {
                let line = lines.partition_point(|&l| l < range.start);
                println!(
                    "Unexpected token: {}, expected {}. line {}: `{}`\n{}",
                    unexpected,
                    expected,
                    line - 1,
                    &source[range],
                    &source[lines[line - 1]..lines[line]]
                );
            }
            ExpectedType { unexpected, range } => {
                let line = lines.partition_point(|&l| l < range.start);
                println!(
                    "Unexpected token: {}, expected a type. line {}: `{}`\n{}",
                    unexpected,
                    line - 1,
                    &source[range],
                    &source[lines[line - 1]..lines[line]]
                );
            }
            EOF => println!(
                "Unexpected end of file on line {}\n{}",
                lines.len() - 2,
                &source[lines[lines.len() - 2]..lines[lines.len() - 1]]
            ),
        }
    }
}
