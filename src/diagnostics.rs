use crate::{
    ast::{FieldDecl, Type},
    lexer::Token,
};
use std::ops::Range;

#[derive(Debug)]
pub enum Diagnostic {
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
}
