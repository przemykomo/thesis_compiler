use std::range::Range;

#[derive(Debug, Clone, PartialEq)]
pub struct Span<T> {
    pub val: T,
    pub span: Range<usize>,
}

#[derive(Debug)]
pub struct ParsedFile<'a> {
    pub functions: Vec<FunDecl<'a>>,
    pub structs: Vec<StructDecl<'a>>,
    // pub diagnostics: Vec<Diagnostic>
}

#[derive(Debug)]
pub struct StructDecl<'a> {
    pub ident: Span<&'a str>,
    pub fields: Vec<FieldDecl<'a>>,
}

#[derive(Debug)]
pub struct FunDecl<'a> {
    pub ident: Span<&'a str>,
    pub return_ty: Type<'a>,
    pub args: Vec<FieldDecl<'a>>,
    pub body: Option<Vec<Statement<'a>>>,
}

#[derive(Debug)]
pub struct FieldDecl<'a> {
    pub ident: Span<&'a str>,
    pub ty: Type<'a>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type<'a> {
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
    Pointer(Box<Type<'a>>),
    Boolean,
    Void,
    Struct(Span<&'a str>),
}

#[derive(Debug)]
pub enum Statement<'a> {
    Let {
        ident: Span<&'a str>,
        expr: Option<Span<Expr<'a>>>,
        ty: Type<'a>,
    },
    If {
        expr: Span<Expr<'a>>,
        body: Vec<Statement<'a>>,
        else_body: Option<Vec<Statement<'a>>>,
    },
    While {
        expr: Span<Expr<'a>>,
        body: Vec<Statement<'a>>,
    },
    Return(Span<Expr<'a>>),
    Expr(Span<Expr<'a>>),
    Break,
    Continue,
}

// TODO: FieldExpr moved into BinaryExpr, update grammar
#[derive(Debug)]
pub enum Expr<'a> {
    IntLiteral(i128),
    CharLiteral(char),
    BoolLiteral(bool),
    FloatLiteral(f64),
    StringLiteral(&'a str),
    Call(Span<FunctionCall<'a>>),
    Binary {
        lhs: Box<Span<Expr<'a>>>,
        rhs: Box<Span<Expr<'a>>>,
        operator: BinaryOp,
    },
    Unary {
        expr: Box<Span<Expr<'a>>>,
        operator: UnaryOp,
    },
    Cast {
        expr: Box<Span<Expr<'a>>>,
        ty: Type<'a>,
    },
    StructLiteral {
        ident: Span<&'a str>,
        members: Vec<(Span<&'a str>, Option<Span<Expr<'a>>>)>,
    },
    Identifier(Span<&'a str>), //TODO add to grammar
    Index {
        expr: Box<Span<Expr<'a>>>,
        index: Box<Span<Expr<'a>>>,
    },
}

#[derive(Debug)]
pub struct FunctionCall<'a> {
    pub ident: Span<&'a str>,
    pub args: Vec<Span<Expr<'a>>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOp {
    Arithmetic(ArithmeticOp),
    Comparison(ComparisonOp),
    Assign,
    ArithmeticAssign(ArithmeticOp),
    MemberAccess,
}

//TODO remove bitwise operators from the language grammar?
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
