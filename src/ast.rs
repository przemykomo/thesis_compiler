use std::{collections::HashMap, ops::Range};

use crate::diagnostics::Diagnostic;

#[derive(Debug, Clone, PartialEq)]
pub struct Span<T> {
    pub val: T,
    pub span: Range<usize>,
}

#[derive(Debug)]
pub struct ParsedFile<'a> {
    pub functions: HashMap<&'a str, FnDecl>,
    pub structs: HashMap<&'a str, StructDecl>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug)]
pub struct StructDecl {
    pub ident: Range<usize>,
    pub fields: Vec<FieldDecl>,
}

#[derive(Debug)]
pub struct FnDecl {
    pub ident: Range<usize>,
    pub return_ty: Type,
    pub args: Vec<FieldDecl>,
    pub body: Option<Vec<Statement>>,
}

#[derive(Debug, Clone)]
pub struct FieldDecl {
    pub ident: Range<usize>,
    pub ty: Type,
}
impl PartialEq for FieldDecl {
    fn eq(&self, other: &Self) -> bool {
        self.ty == other.ty
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    UnsizedInt,
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    I64,
    U64,

    UnsizedFloat,
    F32,
    F64,

    Char,
    Pointer(Box<Type>),
    //TODO Function pointers
    Fn {
        args: Vec<FieldDecl>,
        return_ty: Box<Type>,
    },
    Boolean,
    Void,
    Struct(Range<usize>),
}

impl Type {
    pub fn is_numeric(&self) -> bool {
        use Type::*;
        matches!(
            self,
            UnsizedInt | I8 | U8 | I16 | U16 | I32 | U32 | I64 | U64 | UnsizedFloat | F32 | F64
        )
    }

    pub fn is_int_char_bool(&self) -> bool {
        use Type::*;
        matches!(
            self,
            UnsizedInt | I8 | U8 | I16 | U16 | I32 | U32 | I64 | U64 | Char | Boolean
        )
    }

    pub fn is_int(&self) -> bool {
        use Type::*;
        matches!(
            self,
            UnsizedInt | I8 | U8 | I16 | U16 | I32 | U32 | I64 | U64
        )
    }

    pub fn is_float(&self) -> bool {
        use Type::*;
        // UnsizedInt can be promoted to a float TODO make it clearer
        matches!(self, UnsizedInt | UnsizedFloat | F32 | F64)
    }
}

#[derive(Debug)]
pub enum Statement {
    Let {
        ident: Range<usize>,
        expr: Option<Expr>,
        ty: Option<Type>,
    },
    If {
        expr: Expr,
        body: Vec<Statement>,
        else_body: Option<Vec<Statement>>,
    },
    While {
        expr: Expr,
        body: Vec<Statement>,
    },
    Return {
        span: Range<usize>,
        expr: Option<Expr>,
    },
    Break(Range<usize>),
    Continue(Range<usize>),
    Expr(Expr),
}

#[derive(Debug)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Range<usize>,
}

// FieldExpr can't be moved into BinaryExpr, since the rhs must be an identifier of a member, not a
// generic expression
// TODO Maybe just remove ranges from here as Expr has a range already?
#[derive(Debug)]
pub enum ExprKind {
    IntLiteral(Range<usize>),
    CharLiteral(Range<usize>),
    BoolLiteral(bool),
    FloatLiteral(Range<usize>),
    StringLiteral(Range<usize>),
    Call {
        lhs: Box<Expr>,
        args: Vec<Expr>,
    },
    Binary {
        lhs: Box<Expr>,
        rhs: Box<Expr>,
        op: BinaryOp,
    },
    Unary {
        expr: Box<Expr>,
        op: UnaryOp,
    },
    Cast {
        expr: Box<Expr>,
        ty: Type,
    },
    StructLiteral {
        ident: Range<usize>,
        fields: Vec<(Range<usize>, Expr)>,
    },
    Identifier(Range<usize>),
    Field {
        expr: Box<Expr>,
        member: Range<usize>,
    },
    //TODO: This might be just merged into the Binary expression
    Index {
        expr: Box<Expr>,
        index: Box<Expr>,
    },
}

// No bitwise operator for now
#[derive(Debug, Clone, PartialEq, Copy)]
pub enum BinaryOp {
    Arithmetic(ArithmeticOp),
    Comparison(ComparisonOp),
    Boolean(BooleanOp),
    Assign,
    ArithmeticAssign(ArithmeticOp),

    // Special operators to make expr parser simpler
    Index,
    Call,
    FieldAccess,
    Cast,
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum ArithmeticOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum ComparisonOp {
    Eq,
    Greater,
    Less,
    GreaterEq,
    LessEq,
    NotEq,
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum BooleanOp {
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum UnaryOp {
    Minus,
    Not,
    AddressOf,
    Deref,
}
