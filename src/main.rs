// #![allow(warnings)]
pub mod abi;
pub mod ast;
pub mod csa;
pub mod diagnostics;
pub mod lexer;
pub mod lower;
pub mod parser;
use std::{ffi::CString, fs, process::Command};

use crate::{
    csa::perform_csa,
    diagnostics::Diagnostic::{self, *},
    lexer::Token,
    lower::lower_to_llvm,
    parser::parse_tokens,
};
use llvm_sys::{
    core::LLVMContextDispose,
    target_machine::{LLVMCodeGenFileType, LLVMTargetMachineEmitToFile},
};
use logos::Logos;

fn main() {
    //TODO cli args

    let source_file = "test/sample.tc";
    let object_file = "out.o";
    let exec_file = "out";
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
    if !parsed_file.diagnostics.is_empty() {
        print_diagnostics(&parsed_file.diagnostics, &lines, &source);
        return;
    }

    let typed_ast = perform_csa(parsed_file, &source);

    if !typed_ast.diagnostics.is_empty() {
        print_diagnostics(&typed_ast.diagnostics, &lines, &source);
        return;
    }

    println!("No errors detected.");

    let (context, machine, module) = lower_to_llvm(typed_ast, &source);

    unsafe {
        // LLVMDumpModule(module);

        let mut err_string = ::core::ptr::null_mut();
        let ret = LLVMTargetMachineEmitToFile(
            machine,
            module,
            CString::new(object_file).unwrap().as_ptr(),
            LLVMCodeGenFileType::LLVMObjectFile,
            &mut err_string,
        );

        if ret == 1 {
            panic!("emit object file");
            // panic!("{}", LLVMString::new(err_string));
        }

        LLVMContextDispose(context);
    }

    Command::new("gcc")
        .arg(object_file)
        .arg("-o")
        .arg(exec_file)
        .status()
        .expect("Couldn't link the executable.");
}

fn print_diagnostics(diagnostics: &[Diagnostic], lines: &[usize], source: &str) {
    for diagnostic in diagnostics {
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
                    &source[range.clone()],
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
                    &source[range.clone()],
                    &source[lines[line - 1]..lines[line]]
                );
            }
            ExpectedType { unexpected, range } => {
                let line = lines.partition_point(|&l| l < range.start);
                println!(
                    "Unexpected token: {}, expected a type. line {}: `{}`\n{}",
                    unexpected,
                    line - 1,
                    &source[range.clone()],
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
                    &source[range.clone()],
                    &source[lines[line - 1]..lines[line]]
                );
            }
            // NoVariableNamed { range } => {
            //     let line = lines.partition_point(|&l| l < range.start);
            //     println!(
            //         "No variable named {}, line {}:\n{}",
            //         &source[range.clone()],
            //         line - 1,
            //         &source[lines[line - 1]..lines[line]]
            //     );
            // }
            _ => {
                dbg!(diagnostic);
            }
        }
    }
}
