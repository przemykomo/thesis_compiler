use std::{collections::HashMap, ops::Range};

use crate::{
    ast::{FieldDecl, FnDecl, ParsedFile, StructDecl, Type},
    diagnostics::Diagnostic,
};

pub struct TypedAST<'a> {
    pub functions: HashMap<&'a str, TypedFn>,
    pub structs: HashMap<&'a str, StructDecl>,
    pub diagnostics: Vec<Diagnostic>,
}

pub struct TypedFn {
    pub ident: Range<usize>,
    pub return_ty: Type,
    pub args: Vec<FieldDecl>,
    // pub body: Option<Vec<Statement>>,
}

pub fn perform_csa<'a>(ast: ParsedFile<'a>, source: &'a str) -> TypedAST<'a> {
    let mut diagnostics = Vec::new();
    // Ensure that there is no recursive structs, as these have infinite size
    for (_, decl) in &ast.structs {
        check_recursive_structs(&ast, decl, &mut Vec::new(), source, &mut diagnostics);
    }

    let mut functions = HashMap::new();
    for (_, decl) in &ast.functions {
        let typed_fn = type_fn(decl);
        functions.insert(&source[typed_fn.ident.clone()], typed_fn);
    }

    let typed = TypedAST {
        functions,
        structs: ast.structs,
        diagnostics,
    };

    typed
}

fn check_recursive_structs<'a>(
    ast: &'a ParsedFile,
    decl: &'a StructDecl,
    stack: &mut Vec<&'a StructDecl>,
    source: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    stack.push(decl);
    for field in &decl.fields {
        let Type::Struct(ident) = &field.ty else {
            continue;
        };

        let Some(field_decl) = ast.structs.get(&source[ident.clone()]) else {
            diagnostics.push(Diagnostic::NoTypeNamed {
                range: ident.clone(),
            });
            continue;
        };

        if stack
            .iter()
            .find(|&&s| s as *const StructDecl == field_decl as *const _)
            .is_some()
        {
            diagnostics.push(Diagnostic::RecursiveStruct {
                range: ident.clone(),
            });
        } else {
            check_recursive_structs(ast, field_decl, stack, source, diagnostics);
        }
    }
    stack.pop();
}

fn type_fn(decl: &FnDecl) -> TypedFn {
    todo!()
}
