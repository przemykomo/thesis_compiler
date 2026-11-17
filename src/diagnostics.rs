use crate::{
    ast::{FieldDecl, Type},
    lexer::Token,
};
use std::ops::Range;

#[derive(Debug)]
pub enum Diagnostic {
    InvalidToken {
        range: Range<usize>,
    },
    UnexpectedToken {
        unexpected: Token,
        range: Range<usize>,
        expected: &'static [Token],
    },
    UnexpectedToken2 {
        unexpected: Token,
        range: Range<usize>,
        expected: Token,
    },
    ExpectedType {
        unexpected: Token,
        range: Range<usize>,
    },
    ExpectedExpression {
        unexpected: Token,
        range: Range<usize>,
    },
    EOF,
    DuplicateDefinition {
        range: Range<usize>,
        old: Range<usize>,
    },
    EllipsisNotLast {
        range: Range<usize>,
    },
    VariadicFnDef {
        range: Range<usize>,
    },

    NoTypeNamed {
        range: Range<usize>,
    },
    RecursiveStruct {
        range: Range<usize>,
    },

    FunctionDoesntReturn {
        range: Range<usize>,
    },

    MissingVariableType {
        range: Range<usize>,
    },

    CantCoerceType {
        range: Range<usize>,
        from: Type,
        to: Type,
    },
    VoidReturn {
        range: Range<usize>,
        expected: Type,
    },
    NotALoop {
        range: Range<usize>,
    },
    NoVariableOrFunction {
        range: Range<usize>,
    },
    NoFieldNamed {
        range: Range<usize>,
    },
    ExpectedField {
        expected: Range<usize>,
        got: Range<usize>,
    },
    MissingFields {
        range: Range<usize>,
        fields: Vec<FieldDecl>,
    },

    ExpectedNumericType {
        range: Range<usize>,
        got: Type,
    },
    DerefNonPointer {
        range: Range<usize>,
        got: Type,
    },
    CantCoerceBinary {
        range: Range<usize>,
        lhs: Type,
        rhs: Type,
    },
    CantCast {
        range: Range<usize>,
        from: Type,
        to: Type,
    },
    NotAStruct {
        range: Range<usize>,
    },
    NotAnLValue {
        range: Range<usize>,
    },
    NotAFunction {
        range: Range<usize>,
    },
    MissingArgs {
        range: Range<usize>,
        expected: usize,
        got: usize,
    },
    UnexpectedArgs {
        range: Range<usize>,
        expected: usize,
        got: usize,
    },
    NonASCIIChar {
        range: Range<usize>,
    },
    IndexNonPtr {
        range: Range<usize>,
    },
}

pub fn print_diagnostics(diagnostics: &[Diagnostic], lines: &[usize], source: &str) {
    use Diagnostic::*;
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
            CantCoerceType { range, from, to } => {
                let line = lines.partition_point(|&l| l < range.start);
                print!("Can't coerce type from ");
                print_type(from, source);
                print!(" to ");
                print_type(to, source);
                println!(
                    " line {}: `{}`\n{}",
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

pub fn print_type(ty: &Type, source: &str) {
    match ty {
        Type::UnsizedInt => print!("{{int}}"),
        Type::I8 => print!("i8"),
        Type::U8 => print!("u8"),
        Type::I16 => print!("i16"),
        Type::U16 => print!("u16"),
        Type::I32 => print!("i32"),
        Type::U32 => print!("u32"),
        Type::I64 => print!("i64"),
        Type::U64 => print!("u64"),
        Type::UnsizedFloat => print!("{{float}}"),
        Type::F32 => print!("f32"),
        Type::F64 => print!("f64"),
        Type::Char => print!("char"),
        Type::Pointer(ty) => {
            print!("*");
            print_type(ty, source);
        }
        Type::Fn {
            args: _,
            variadic: _,
            return_ty: _,
        } => todo!(),
        Type::Boolean => print!("bool"),
        Type::Void => print!("void"),
        Type::Struct(range) => print!("{}", &source[range.clone()]),
    }
}
