pub mod ast;
pub mod diagnostics;
pub mod lexer;
pub mod parser;
use std::fs;

use crate::{diagnostics::Diagnostic::*, lexer::Token, parser::parse_tokens};
use logos::Logos;

fn main() {
    //TODO cli args

    let source_file = "test/sample.tc";
    let source =
        fs::read_to_string(&source_file).expect("Should have been able to read the source file.");

    let mut lines = vec![0];
    lines.extend(
        source
            .char_indices()
            .filter(|(_, c)| *c == '\n')
            .map(|(i, _)| i + 1),
    );
    lines.push(source.len());

    let lex = Token::lexer(&source).spanned();

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

    let parsed_file = parse_tokens(tokens, &source);
    if parsed_file.diagnostics.is_empty() {
        dbg!(parsed_file.functions);
    }

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
            ExpectedExpression { unexpected, range } => {
                let line = lines.partition_point(|&l| l < range.start);
                println!(
                    "Unexpected token: {}, expected an expression. line {}: `{}`\n{}",
                    unexpected,
                    line - 1,
                    &source[range],
                    &source[lines[line - 1]..lines[line]]
                );
            }
        }
    }
}
