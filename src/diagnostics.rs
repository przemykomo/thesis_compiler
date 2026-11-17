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

fn d(msg: &str, range: &Range<usize>, lines: &[usize], source: &str) {
    let line = lines.partition_point(|&l| l < range.start) - 1;
    let s = &source[range.clone()];
    let mut line_src = &source[lines[line]..lines[line + 1]];
    if line_src.ends_with('\n') {
        line_src = &line_src[..line_src.len() - 1];
    }
    println!("Line {line}: {msg}: `{s}`\n{line_src}");
    let col_start = range.start - lines[line];
    let col_end = (range.end - lines[line]).min(lines[line + 1]);
    print!("{: >width$}", "", width = col_start);
    println!("{:^>width$}", "", width = col_end - col_start);
}

pub fn print_diagnostics(diagnostics: &[Diagnostic], lines: &[usize], source: &str) {
    use Diagnostic::*;
    for diagnostic in diagnostics {
        match diagnostic {
            InvalidToken { range } => {
                d("Invalid token", range, lines, source);
            }
            UnexpectedToken {
                unexpected,
                range,
                expected,
            } => {
                d("Unexpected token", range, lines, source);
                println!("Expected {expected:#?}, got {unexpected}.");
            }
            UnexpectedToken2 {
                unexpected,
                range,
                expected,
            } => {
                d("Unexpected token", range, lines, source);
                println!("Expected {expected}, got {unexpected}.");
            }
            ExpectedType {
                unexpected: _,
                range,
            } => {
                d("Expected a type", range, lines, source);
            }
            EOF => println!(
                "Unexpected end of file on line {}\n{}",
                lines.len() - 2,
                &source[lines[lines.len() - 2]..lines[lines.len() - 1]]
            ),
            ExpectedExpression {
                unexpected: _,
                range,
            } => {
                d("Expected an expression", range, lines, source);
            }
            CantCoerceType { range, from, to } => {
                d("Can't coerce type", range, lines, source);
                print!("From ");
                print_type(from, source);
                print!(" to ");
                print_type(to, source);
                println!();
            }
            DuplicateDefinition { range, old } => {
                d("Duplicate definition", range, lines, source);
                d("Note: original definition:", old, lines, source);
            }
            EllipsisNotLast { range } => {
                d("Ellipsis must be a last argument", range, lines, source);
            }
            VariadicFnDef { range } => {
                d(
                    "Only external declarations are supported for variadic functions",
                    range,
                    lines,
                    source,
                );
            }
            NoTypeNamed { range } => {
                d("Invalid type", range, lines, source);
            }
            RecursiveStruct { range } => {
                d("Recursive struct definition", range, lines, source);
            }
            FunctionDoesntReturn { range } => {
                d("Function doesn't return", range, lines, source);
            }
            MissingVariableType { range } => {
                d("Missing variable type", range, lines, source);
            }
            VoidReturn { range, expected } => {
                d("Trying to return without a value", range, lines, source);
                print!("Expected a value of type: ");
                print_type(expected, source);
                println!();
            }
            NotALoop { range } => {
                d("Not a loop", range, lines, source);
            }
            NoVariableOrFunction { range } => {
                d("No variable or function", range, lines, source);
            }
            NoFieldNamed { range } => {
                d("No field named", range, lines, source);
            }
            ExpectedField { expected, got } => {
                d("Expected field", expected, lines, source);
                d("Instead got", got, lines, source);
            }
            MissingFields { range, fields } => {
                d("Missing fields from a struct literal", range, lines, source);
                print!("Missing fields: ");
                for f in fields {
                    print!("{}: ", &source[f.ident.clone()]);
                    print_type(&f.ty, source);
                    print!(", ");
                }
                println!();
            }
            ExpectedNumericType { range, got } => {
                d("Expected a numeric type", range, lines, source);
                print!("Instead got ");
                print_type(got, source);
                println!();
            }
            DerefNonPointer { range, got } => {
                d("Trying to dereference a non-pointer", range, lines, source);
                print!("Instead got ");
                print_type(got, source);
                println!();
            }
            CantCoerceBinary { range, lhs, rhs } => {
                d("Can't coerce types", range, lines, source);
                print!("Types: ");
                print_type(lhs, source);
                print!(" and ");
                print_type(rhs, source);
                println!();
            }
            CantCast { range, from, to } => {
                d("Can't cast", range, lines, source);
                print!("From ");
                print_type(from, source);
                print!(" to ");
                print_type(to, source);
                println!();
            }
            NotAStruct { range } => {
                d("Not a struct", range, lines, source);
            }
            NotAnLValue { range } => {
                d("Not an lvalue", range, lines, source);
            }
            NotAFunction { range } => {
                d("Not a function", range, lines, source);
            }
            MissingArgs {
                range,
                expected,
                got,
            } => {
                d("Missing function arguments", range, lines, source);
                println!("Expected {expected} arguments, got {got} instead.");
            }
            UnexpectedArgs {
                range,
                expected,
                got,
            } => {
                d(
                    "Additional function arguments provided",
                    range,
                    lines,
                    source,
                );
                println!("Expected {expected} arguments, got {got} instead.");
            }
            NonASCIIChar { range } => {
                d("Not an ascii character", range, lines, source);
            }
            IndexNonPtr { range } => {
                d("Trying to index a non-pointer", range, lines, source);
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
