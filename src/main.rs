// #![allow(warnings)]
pub mod abi;
pub mod ast;
pub mod csa;
pub mod diagnostics;
pub mod lexer;
pub mod lower;
pub mod parser;
pub mod tests;
use std::{ffi::CString, fs, process::Command};

use crate::{
    csa::perform_csa,
    diagnostics::{Diagnostic, print_diagnostics},
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

    if let Err(diagnostics) = compile_and_link(object_file, exec_file, &source) {
        let mut lines = vec![0];
        lines.extend(
            source
                .char_indices()
                .filter(|(_, c)| *c == '\n')
                .map(|(i, _)| i + 1),
        );
        lines.push(source.len());
        print_diagnostics(&diagnostics, &lines, &source);
    }
}

pub fn compile_and_link(
    object_file: &str,
    exec_file: &str,
    source: &str,
) -> Result<(), Vec<Diagnostic>> {
    compile(object_file, source)?;
    Command::new("gcc")
        .args([object_file, "-o", exec_file])
        .status()
        .expect("Couldn't link the executable.");
    Ok(())
}

pub fn compile(object_file: &str, source: &str) -> Result<(), Vec<Diagnostic>> {
    let lex = Token::lexer(&source).spanned();

    let mut tokens = Vec::new();
    let mut diagnostics = Vec::new();
    for (token, range) in lex {
        if let Ok(token) = token {
            tokens.push((token, range));
        } else {
            diagnostics.push(Diagnostic::InvalidToken { range });
        }
    }

    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    let parsed_file = parse_tokens(tokens, &source);
    if !parsed_file.diagnostics.is_empty() {
        return Err(parsed_file.diagnostics);
    }

    let typed_ast = perform_csa(parsed_file, &source);

    if !typed_ast.diagnostics.is_empty() {
        return Err(typed_ast.diagnostics);
    }

    // println!("No errors detected.");

    let (context, machine, module) = lower_to_llvm(typed_ast, &source);

    unsafe {
        // llvm_sys::core::LLVMDumpModule(module);

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

    Ok(())
}
