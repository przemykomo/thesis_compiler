use std::ops::Range;

use crate::diagnostics::Diagnostic;

#[derive(Debug, Clone, PartialEq)]
pub struct Span<T> {
    pub val: T,
    pub span: Range<usize>,
}

#[derive(Debug)]
pub struct ParsedFile<'a> {
    pub functions: Vec<FnDecl>,
    pub structs: Vec<StructDecl>,
    pub source: &'a str,
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

#[derive(Debug)]
pub struct FieldDecl {
    pub ident: Range<usize>,
    pub ty: Type,
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
    Boolean,
    Void,
    Struct(Range<usize>),
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
    Return(Some(Expr)),
    Break,
    Continue,
    Expr(Expr),
}

#[derive(Debug)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Range<usize>,
}

// FieldExpr can't be moved into BinaryExpr, since the rhs must be an identifier of a member, not a
// generic expression
#[derive(Debug)]
pub enum ExprKind {
    IntLiteral(Range<usize>),
    CharLiteral(Range<usize>),
    BoolLiteral(bool),
    FloatLiteral(Range<usize>),
    StringLiteral(Range<usize>),
    Call(Span<FunctionCall>),
    Binary {
        lhs: Box<Expr>,
        rhs: Box<Expr>,
        operator: BinaryOp,
    },
    Unary {
        expr: Box<Expr>,
        operator: UnaryOp,
    },
    Cast {
        expr: Box<Expr>,
        ty: Type,
    },
    StructLiteral {
        ident: Range<usize>,
        members: Vec<(Range<usize>, Option<Expr>)>,
    },
    Identifier(Range<usize>),
    Field {
        expr: Box<Expr>,
        member: Range<usize>,
    },
    Index {
        expr: Box<Expr>,
        index: Box<Expr>,
    },
}

#[derive(Debug)]
pub struct FunctionCall {
    pub ident: Range<usize>,
    pub args: Vec<Expr>,
}

// No bitwise operator for now
#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOp {
    Arithmetic(ArithmeticOp),
    Comparison(ComparisonOp),
    Assign,
    ArithmeticAssign(ArithmeticOp),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ArithmeticOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ComparisonOp {
    Eq,
    Larger,
    Smaller,
    LargerEq,
    SmallerEq,
    NotEq,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Minus,
    Plus,
    Not,
    AddressOf,
    Deref,
}
