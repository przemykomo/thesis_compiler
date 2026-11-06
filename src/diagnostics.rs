use crate::lexer::Token;
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
}
