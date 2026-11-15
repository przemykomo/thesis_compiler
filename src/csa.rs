use std::{collections::HashMap, ops::Range};

use crate::{ast::*, diagnostics::Diagnostic};

#[derive(Debug)]
pub struct TypedAST<'a> {
    pub functions: HashMap<&'a str, TypedFn>,
    pub structs: HashMap<&'a str, StructDecl>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug)]
pub struct TypedFn {
    pub ident: Range<usize>,
    pub return_ty: Type,
    pub args: Vec<FieldDecl>,
    pub body: Option<TypedBlock>,
    pub scope: Scope,
}

#[derive(Debug)]
pub struct TypedBlock {
    pub statements: Vec<TypedStatement>,
    pub return_actuality: BlockReturnActuality,
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum BlockReturnActuality {
    NeverReturns,
    SometimesReturns,
    AlwaysReturns,
}

impl BlockReturnActuality {
    #[must_use]
    pub const fn join(a: Self, b: Self) -> Self {
        use BlockReturnActuality::*;
        match (a, b) {
            (NeverReturns, NeverReturns) => NeverReturns,

            (NeverReturns | SometimesReturns, SometimesReturns | AlwaysReturns)
            | (SometimesReturns | AlwaysReturns, NeverReturns | SometimesReturns) => {
                SometimesReturns
            }

            (AlwaysReturns, AlwaysReturns) => AlwaysReturns,
        }
    }
}

#[derive(Debug)]
pub enum TypedStatement {
    // TODO this will probably only get checked in case of the if statement during codegen, maybe
    // shouldn't put it here for all statements
    // pub return_actuality: BlockReturnActuality,
    // Making TypedStatement an enum instead of a struct with a field for the kind
    Let {
        ident: Range<usize>,
        expr: Option<TypedExpr>,
    },
    If {
        expr: TypedExpr,
        body: TypedBlock,
        else_body: Option<TypedBlock>,
    },
    While {
        expr: TypedExpr,
        body: TypedBlock,
    },
    Return(Option<TypedExpr>),

    //TODO maybe Break and Continue should contain loop label aquired during csa
    Break,
    Continue,
    Expr(TypedExpr),
}

#[derive(Debug)]
pub struct TypedExpr {
    pub kind: TypedExprKind,
    pub ty: Type,
}

impl TypedExpr {
    fn coerce_to(&mut self, ty: &Type) -> bool {
        use Type::*;
        if self.ty == *ty {
            return true;
        }

        if self.ty == UnsizedInt {
            match ty {
                UnsizedInt => todo!(),
                I8 | U8 | I16 | U16 | I32 | U32 | I64 | U64 | F32 | F64 => {
                    self.ty = ty.clone();
                    return true;
                }
                _ => return false,
            }
        }

        if self.ty == UnsizedFloat {
            match ty {
                UnsizedFloat => todo!(),
                F32 | F64 => {
                    self.ty = ty.clone();
                    return true;
                }
                _ => return false,
            }
        }

        false
    }

    fn can_cast(&self, ty: &Type) -> bool {
        use Type::*;
        if self.ty.is_int_char_bool() && ty.is_int_char_bool() {
            return true;
        }

        if self.ty.is_numeric() && ty.is_numeric() {
            return true;
        }

        if let (Pointer(_), Pointer(_)) = (&self.ty, ty) {
            return true;
        }

        false
    }
}

#[derive(Debug)]
pub enum TypedExprKind {
    IntLiteral(i128),
    CharLiteral(u8),
    BoolLiteral(bool),
    FloatLiteral(f64),
    //TODO parse string literals
    StringLiteral(Range<usize>),
    Call {
        lhs: Box<TypedExpr>,
        args: Vec<TypedExpr>,
    },

    // Binary expr is split up since Assign requires a Place,
    // and Index, Call, FieldAccess and Cast were defined in BinaryOp just to make parsing easier
    Arithmetic {
        lhs: Box<TypedExpr>,
        rhs: Box<TypedExpr>,
        op: ArithmeticOp,
    },
    Comparison {
        lhs: Box<TypedExpr>,
        rhs: Box<TypedExpr>,
        op: ComparisonOp,
    },
    Boolean {
        lhs: Box<TypedExpr>,
        rhs: Box<TypedExpr>,
        op: BooleanOp,
    },
    Assign {
        lhs: Box<Place>,
        rhs: Box<TypedExpr>,
    },
    //TODO Figure out if I can somehow desugar this without having lhs potentialy run code twice
    ArithmeticAssign {
        lhs: Box<Place>,
        rhs: Box<TypedExpr>,
        op: ArithmeticOp,
    },

    // Unary is split up since AddressOf requires a Place
    Minus(Box<TypedExpr>),
    Not(Box<TypedExpr>),
    AddressOf(Box<Place>),
    Deref(Box<TypedExpr>),

    Cast {
        expr: Box<TypedExpr>,
        ty: Type, // TODO remove?
    },
    StructLiteral {
        // ident: Range<usize>,
        fields: Vec<TypedExpr>,
    },

    //TODO split up to variable and function access?
    // Identifier(Range<usize>),
    Variable(usize),
    Function(Range<usize>),
    Field {
        place: Box<Place>,
        field: Range<usize>,
    },
}

#[derive(Debug)]
pub struct Place {
    pub kind: PlaceKind,
    pub ty: Type,
}

#[derive(Debug)]
pub enum PlaceKind {
    Deref(Box<TypedExpr>),
    Variable(usize),
    Index(Box<TypedExpr>, Box<TypedExpr>),
    Field(Box<Place>, Range<usize>),
}

#[derive(Debug)]
pub struct Variable {
    pub ident: Range<usize>,
    pub ty: Option<Type>,
}

#[derive(Debug)]
pub struct Scope {
    pub vars: Vec<Variable>,
    pub visible_vars: Vec<usize>,
}

impl Scope {
    pub fn insert(&mut self, var: Variable) {
        let id = self.visible_vars.len();
        self.vars.push(var);
        self.visible_vars.push(id);
    }

    pub fn find(&self, ident: Range<usize>, source: &str) -> Option<usize> {
        for visible in &self.visible_vars {
            if &source[self.vars[*visible].ident.clone()] == &source[ident.clone()] {
                return Some(*visible);
            }
        }

        None
    }
}

pub fn perform_csa<'a>(ast: ParsedFile<'a>, source: &'a str) -> TypedAST<'a> {
    let mut diagnostics = Vec::new();
    // Ensure that there is no recursive structs, as these have infinite size
    for (_, decl) in &ast.structs {
        check_recursive_structs(&ast, decl, &mut Vec::new(), source, &mut diagnostics);
    }

    let mut functions = HashMap::new();
    for (_, decl) in &ast.functions {
        let typed_fn = type_fn(decl, &ast, &mut diagnostics, source);
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
        let mut tmp = &field.ty;
        while let Type::Pointer(ty) = tmp {
            if let Type::Struct(ref ident) = **ty {
                if !ast.structs.contains_key(&source[ident.clone()]) {
                    diagnostics.push(Diagnostic::NoTypeNamed {
                        range: ident.clone(),
                    });
                }
                break;
            };
            tmp = ty;
        }

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

fn type_fn(
    decl: &FnDecl,
    ast: &ParsedFile,
    diagnostics: &mut Vec<Diagnostic>,
    source: &str,
) -> TypedFn {
    let mut scope = Scope {
        vars: Vec::new(),
        visible_vars: Vec::new(),
    };

    let Some(body) = &decl.body else {
        return TypedFn {
            ident: decl.ident.clone(),
            return_ty: decl.return_ty.clone(),
            args: decl.args.clone(),
            body: None,
            scope,
        };
    };

    for arg in &decl.args {
        scope.insert(Variable {
            ident: arg.ident.clone(),
            ty: Some(arg.ty.clone()),
        });
    }

    let mut block = type_block(
        ast,
        &mut scope,
        body,
        false,
        decl.return_ty.clone(),
        diagnostics,
        source,
    );

    if block.return_actuality != BlockReturnActuality::AlwaysReturns {
        if decl.return_ty == Type::Void {
            block.statements.push(TypedStatement::Return(None));
            block.return_actuality = BlockReturnActuality::AlwaysReturns;
        } else {
            diagnostics.push(Diagnostic::FunctionDoesntReturn {
                range: decl.ident.clone(),
            });
        }
    }

    TypedFn {
        ident: decl.ident.clone(),
        return_ty: decl.return_ty.clone(),
        args: decl.args.clone(),
        body: Some(block),
        scope,
    }
}

fn type_block(
    ast: &ParsedFile,
    scope: &mut Scope,
    block: &Vec<Statement>,
    is_loop: bool,
    return_ty: Type,
    diagnostics: &mut Vec<Diagnostic>,
    source: &str,
) -> TypedBlock {
    let mut statements = Vec::new();
    let mut sometimes_returns = false;
    let mut always_returns = false;

    for statement in block {
        match statement {
            Statement::Let { ident, expr, ty } => {
                let Some(expr) = expr else {
                    let ty = match ty {
                        Some(ty) => ty,
                        None => {
                            diagnostics.push(Diagnostic::MissingVariableType {
                                range: ident.clone(),
                            });
                            scope.insert(Variable {
                                ident: ident.clone(),
                                ty: None,
                            });
                            continue;
                        }
                    };

                    scope.insert(Variable {
                        ident: ident.clone(),
                        ty: Some(ty.clone()),
                    });
                    statements.push(TypedStatement::Let {
                        ident: ident.clone(),
                        expr: None,
                    });
                    continue;
                };

                let Some(mut expr) = type_expr(ast, scope, expr, diagnostics, source) else {
                    scope.insert(Variable {
                        ident: ident.clone(),
                        ty: ty.clone(),
                    });
                    continue;
                };

                if let Some(ty) = ty {
                    if !expr.coerce_to(ty) {
                        diagnostics.push(Diagnostic::CantCoerceType {
                            range: ident.clone(),
                            from: expr.ty.clone(),
                            to: ty.clone(),
                        });
                    }
                } else if expr.ty == Type::UnsizedInt {
                    expr.ty = Type::I32;
                } else if expr.ty == Type::UnsizedFloat {
                    expr.ty = Type::F32;
                }

                scope.insert(Variable {
                    ident: ident.clone(),
                    ty: Some(expr.ty.clone()),
                });

                statements.push(TypedStatement::Let {
                    ident: ident.clone(),
                    expr: Some(expr),
                });
            }
            Statement::If {
                expr,
                body,
                else_body,
            } => {
                let expr_span = expr.span.clone();
                let mut expr = type_expr(ast, scope, expr, diagnostics, source);

                if let Some(ref mut expr) = expr
                    && expr.ty != Type::Boolean
                {
                    diagnostics.push(Diagnostic::CantCoerceType {
                        range: expr_span,
                        from: expr.ty.clone(),
                        to: Type::Boolean,
                    });
                }

                let body = type_block(
                    ast,
                    scope,
                    body,
                    is_loop,
                    return_ty.clone(),
                    diagnostics,
                    source,
                );

                let else_body = else_body.as_ref().map(|else_body| {
                    type_block(
                        ast,
                        scope,
                        else_body,
                        is_loop,
                        return_ty.clone(),
                        diagnostics,
                        source,
                    )
                });

                let else_returned = else_body
                    .as_ref()
                    .map_or(BlockReturnActuality::NeverReturns, |block| {
                        block.return_actuality
                    });

                match BlockReturnActuality::join(body.return_actuality, else_returned) {
                    BlockReturnActuality::NeverReturns => {}
                    BlockReturnActuality::SometimesReturns => sometimes_returns = true,
                    BlockReturnActuality::AlwaysReturns => always_returns = true,
                }

                if let Some(expr) = expr {
                    statements.push(TypedStatement::If {
                        expr,
                        body,
                        else_body,
                    });
                }
            }
            Statement::While { expr, body } => {
                let expr_span = expr.span.clone();
                let mut expr = type_expr(ast, scope, expr, diagnostics, source);

                if let Some(ref mut expr) = expr
                    && expr.ty != Type::Boolean
                {
                    diagnostics.push(Diagnostic::CantCoerceType {
                        range: expr_span,
                        from: expr.ty.clone(),
                        to: Type::Boolean,
                    });
                }

                let body = type_block(
                    ast,
                    scope,
                    body,
                    true,
                    return_ty.clone(),
                    diagnostics,
                    source,
                );

                if let Some(expr) = expr {
                    statements.push(TypedStatement::While { expr, body });
                }
            }
            Statement::Return { span, expr } => {
                always_returns = true;
                if expr.is_none() && return_ty != Type::Void {
                    diagnostics.push(Diagnostic::VoidReturn {
                        range: span.clone(),
                        expected: return_ty.clone(),
                    });
                    continue;
                }

                let expr = expr.as_ref().map(|expr| {
                    (
                        expr.span.clone(),
                        type_expr(ast, scope, expr, diagnostics, source),
                    )
                });

                if let Some((range, Some(mut expr))) = expr {
                    if !expr.coerce_to(&return_ty) {
                        diagnostics.push(Diagnostic::CantCoerceType {
                            range,
                            from: expr.ty.clone(),
                            to: return_ty.clone(),
                        });
                    }
                    statements.push(TypedStatement::Return(Some(expr)));
                } else {
                    statements.push(TypedStatement::Return(None));
                }
            }
            Statement::Break(range) => {
                if !is_loop {
                    diagnostics.push(Diagnostic::NotALoop {
                        range: range.clone(),
                    });
                }
                statements.push(TypedStatement::Break);
            }
            Statement::Continue(range) => {
                if !is_loop {
                    diagnostics.push(Diagnostic::NotALoop {
                        range: range.clone(),
                    });
                }
                statements.push(TypedStatement::Continue);
            }
            Statement::Expr(expr) => {
                let Some(expr) = type_expr(ast, scope, expr, diagnostics, source) else {
                    continue;
                };

                statements.push(TypedStatement::Expr(expr));
            }
        }
    }

    TypedBlock {
        statements,
        return_actuality: if always_returns {
            BlockReturnActuality::AlwaysReturns
        } else if sometimes_returns {
            BlockReturnActuality::SometimesReturns
        } else {
            BlockReturnActuality::NeverReturns
        },
    }
}

fn type_expr(
    ast: &ParsedFile,
    scope: &mut Scope,
    expr: &Expr,
    diagnostics: &mut Vec<Diagnostic>,
    source: &str,
) -> Option<TypedExpr> {
    Some(match &expr.kind {
        ExprKind::IntLiteral(range) => TypedExpr {
            kind: TypedExprKind::IntLiteral(source[range.clone()].parse().unwrap()),
            ty: Type::UnsizedInt,
        },
        ExprKind::CharLiteral(range) => {
            let c = source[range.clone()].chars().next().unwrap();
            if !c.is_ascii() {
                diagnostics.push(Diagnostic::NonASCIIChar {
                    range: range.clone(),
                });
                return None;
            }
            TypedExpr {
                kind: TypedExprKind::CharLiteral(c as u8),
                ty: Type::Char,
            }
        }
        ExprKind::BoolLiteral(val) => TypedExpr {
            kind: TypedExprKind::BoolLiteral(*val),
            ty: Type::Boolean,
        },
        ExprKind::FloatLiteral(range) => TypedExpr {
            kind: TypedExprKind::FloatLiteral(source[range.clone()].parse().unwrap()),
            ty: Type::UnsizedFloat,
        },
        ExprKind::StringLiteral(range) => TypedExpr {
            kind: TypedExprKind::StringLiteral(range.clone()),
            ty: Type::Pointer(Box::new(Type::Char)),
        },
        ExprKind::Call { lhs, args } => {
            let range = expr.span.clone();
            let lspan = lhs.span.clone();
            let lhs = type_expr(ast, scope, lhs, diagnostics, source)?;

            let Type::Fn {
                args: fn_args,
                return_ty,
            } = &lhs.ty
            else {
                diagnostics.push(Diagnostic::NotAFunction { range: lspan });
                return None;
            };

            let mut typed_args = Vec::new();
            for (i, expr) in args.iter().enumerate() {
                let arg = fn_args.get(i)?;
                let range = expr.span.clone();
                let mut expr = type_expr(ast, scope, expr, diagnostics, source)?;
                if !expr.coerce_to(&arg.ty) {
                    diagnostics.push(Diagnostic::CantCoerceType {
                        range,
                        from: expr.ty,
                        to: arg.ty.clone(),
                    });
                    continue;
                }
                typed_args.push(expr);
            }

            if fn_args.len() != args.len() {
                diagnostics.push(Diagnostic::MissingArgs {
                    range,
                    expected: fn_args.len(),
                    got: args.len(),
                });
            }

            let ty = *return_ty.clone();
            TypedExpr {
                kind: TypedExprKind::Call {
                    lhs: Box::new(lhs),
                    args: typed_args,
                },
                ty,
            }
        }
        ExprKind::Binary { lhs, rhs, op } => {
            type_binary_expr(ast, scope, diagnostics, source, lhs, rhs, *op)?
        }
        ExprKind::Unary { expr, op } => {
            let range = expr.span.clone();
            let expr = type_expr(ast, scope, expr, diagnostics, source)?;

            match op {
                UnaryOp::Minus => {
                    if !expr.ty.is_numeric() {
                        diagnostics.push(Diagnostic::ExpectedNumericType {
                            range,
                            got: expr.ty.clone(),
                        });
                        return None;
                    }
                    let ty = expr.ty.clone();
                    TypedExpr {
                        kind: TypedExprKind::Minus(Box::new(expr)),
                        ty,
                    }
                }
                UnaryOp::Not => {
                    if expr.ty != Type::Boolean {
                        diagnostics.push(Diagnostic::CantCoerceType {
                            range,
                            from: expr.ty.clone(),
                            to: Type::Boolean,
                        });
                        return None;
                    }

                    TypedExpr {
                        kind: TypedExprKind::Not(Box::new(expr)),
                        ty: Type::Boolean,
                    }
                }
                UnaryOp::AddressOf => {
                    let ty = expr.ty.clone();
                    TypedExpr {
                        kind: TypedExprKind::AddressOf(Box::new(expr_to_place(
                            range,
                            expr,
                            diagnostics,
                        )?)),
                        ty: Type::Pointer(Box::new(ty)),
                    }
                }
                UnaryOp::Deref => {
                    let Type::Pointer(ty) = expr.ty.clone() else {
                        diagnostics.push(Diagnostic::DerefNonPointer {
                            range,
                            got: expr.ty.clone(),
                        });
                        return None;
                    };

                    TypedExpr {
                        kind: TypedExprKind::Deref(Box::new(expr)),
                        ty: *ty,
                    }
                }
            }
        }
        ExprKind::Cast { expr, ty } => {
            let range = expr.span.clone();
            let mut expr = type_expr(ast, scope, expr, diagnostics, source)?;

            if expr.coerce_to(ty) {
                expr
            } else {
                if !expr.can_cast(ty) {
                    diagnostics.push(Diagnostic::CantCast {
                        range,
                        from: expr.ty,
                        to: ty.clone(),
                    });
                    return None;
                }

                TypedExpr {
                    kind: TypedExprKind::Cast {
                        expr: Box::new(expr),
                        ty: ty.clone(),
                    },
                    ty: ty.clone(),
                }
            }
        }
        ExprKind::StructLiteral { ident, fields } => {
            let Some(decl) = ast.structs.get(&source[ident.clone()]) else {
                diagnostics.push(Diagnostic::NoTypeNamed {
                    range: ident.clone(),
                });
                return None;
            };

            let mut typed_fields = Vec::new();
            let mut err = false;
            for (i, (ident, expr)) in fields.iter().enumerate() {
                let Some(field) = decl.fields.get(i) else {
                    diagnostics.push(Diagnostic::NoFieldNamed {
                        range: ident.clone(),
                    });
                    err = true;
                    continue;
                };

                if &source[field.ident.clone()] != &source[ident.clone()] {
                    diagnostics.push(Diagnostic::ExpectedField {
                        expected: field.ident.clone(),
                        got: ident.clone(),
                    });
                    err = true;
                    continue;
                }

                let range = expr.span.clone();
                if let Some(mut expr) = type_expr(ast, scope, expr, diagnostics, source) {
                    if !expr.coerce_to(&field.ty) {
                        diagnostics.push(Diagnostic::CantCoerceType {
                            range,
                            from: expr.ty.clone(),
                            to: field.ty.clone(),
                        });
                    } else {
                        typed_fields.push(expr);
                    }
                }
            }

            if !err && fields.len() < decl.fields.len() {
                let missing: Vec<FieldDecl> = decl.fields[fields.len()..].iter().cloned().collect();
                diagnostics.push(Diagnostic::MissingFields {
                    range: expr.span.clone(),
                    fields: missing,
                });
            }

            TypedExpr {
                kind: TypedExprKind::StructLiteral {
                    fields: typed_fields,
                },
                ty: Type::Struct(ident.clone()),
            }
        }
        ExprKind::Identifier(range) => {
            if let Some(var) = scope.find(range.clone(), source) {
                TypedExpr {
                    kind: TypedExprKind::Variable(var),
                    ty: scope.vars[var].ty.clone()?,
                }
            } else if let Some(function) = ast.functions.get(&source[range.clone()]) {
                TypedExpr {
                    kind: TypedExprKind::Function(range.clone()),
                    ty: Type::Fn {
                        args: function.args.clone(),
                        return_ty: Box::new(function.return_ty.clone()),
                    },
                }
            } else {
                diagnostics.push(Diagnostic::NoVariableOrFunction {
                    range: range.clone(),
                });
                return None;
            }
        }
        ExprKind::Field { expr, member } => {
            let range = expr.span.clone();
            let expr = type_expr(ast, scope, expr, diagnostics, source)?;

            let Type::Struct(ident) = &expr.ty else {
                diagnostics.push(Diagnostic::NotAStruct { range });
                return None;
            };

            let decl = ast.structs.get(&source[ident.clone()])?;
            let Some(field) = decl
                .fields
                .iter()
                .find(|f| &source[f.ident.clone()] == &source[member.clone()])
            else {
                diagnostics.push(Diagnostic::NoFieldNamed {
                    range: member.clone(),
                });
                return None;
            };

            let place = expr_to_place(range, expr, diagnostics)?;

            TypedExpr {
                kind: TypedExprKind::Field {
                    place: Box::new(place),
                    field: member.clone(),
                },
                ty: field.ty.clone(),
            }
        }
        ExprKind::Index { expr, index } => todo!(),
    })
}

fn type_binary_expr(
    ast: &ParsedFile,
    scope: &mut Scope,
    diagnostics: &mut Vec<Diagnostic>,
    source: &str,
    lhs: &Box<Expr>,
    rhs: &Box<Expr>,
    op: BinaryOp,
) -> Option<TypedExpr> {
    let lspan = lhs.span.clone();
    let rspan = rhs.span.clone();
    let lhs = type_expr(ast, scope, lhs, diagnostics, source);
    let rhs = type_expr(ast, scope, rhs, diagnostics, source);
    let mut lhs = lhs?;
    let mut rhs = rhs?;
    Some(match op {
        BinaryOp::Arithmetic(op) => {
            if let Some(consts) = check_unsized_consts(&mut lhs, &mut rhs) {
                match consts {
                    UnsizedVals::Int(l, r) => {
                        let res = match op {
                            ArithmeticOp::Add => l + r,
                            ArithmeticOp::Sub => l - r,
                            ArithmeticOp::Mul => l * r,
                            ArithmeticOp::Div => l / r,
                            ArithmeticOp::Rem => l % r,
                        };
                        TypedExpr {
                            kind: TypedExprKind::IntLiteral(res),
                            ty: Type::UnsizedInt,
                        }
                    }
                    UnsizedVals::Float(l, r) => {
                        let res = match op {
                            ArithmeticOp::Add => l + r,
                            ArithmeticOp::Sub => l - r,
                            ArithmeticOp::Mul => l * r,
                            ArithmeticOp::Div => l / r,
                            ArithmeticOp::Rem => l % r,
                        };
                        TypedExpr {
                            kind: TypedExprKind::FloatLiteral(res),
                            ty: Type::UnsizedFloat,
                        }
                    }
                }
            } else {
                coerce_binary_nums(diagnostics, &mut lhs, &mut rhs, lspan, rspan)?;
                let ty = lhs.ty.clone();
                TypedExpr {
                    kind: TypedExprKind::Arithmetic {
                        lhs: Box::new(lhs),
                        rhs: Box::new(rhs),
                        op,
                    },
                    ty,
                }
            }
        }
        BinaryOp::Comparison(op) => {
            if let Some(consts) = check_unsized_consts(&mut lhs, &mut rhs) {
                match consts {
                    UnsizedVals::Int(l, r) => {
                        let res = match op {
                            ComparisonOp::Eq => l == r,
                            ComparisonOp::Greater => l > r,
                            ComparisonOp::Less => l < r,
                            ComparisonOp::GreaterEq => l >= r,
                            ComparisonOp::LessEq => l <= r,
                            ComparisonOp::NotEq => l != r,
                        };
                        TypedExpr {
                            kind: TypedExprKind::BoolLiteral(res),
                            ty: Type::Boolean,
                        }
                    }
                    UnsizedVals::Float(l, r) => {
                        let res = match op {
                            ComparisonOp::Eq => l == r,
                            ComparisonOp::Greater => l > r,
                            ComparisonOp::Less => l < r,
                            ComparisonOp::GreaterEq => l >= r,
                            ComparisonOp::LessEq => l <= r,
                            ComparisonOp::NotEq => l != r,
                        };
                        TypedExpr {
                            kind: TypedExprKind::BoolLiteral(res),
                            ty: Type::Boolean,
                        }
                    }
                }
            } else {
                coerce_binary_nums(diagnostics, &mut lhs, &mut rhs, lspan, rspan)?;
                TypedExpr {
                    kind: TypedExprKind::Comparison {
                        lhs: Box::new(lhs),
                        rhs: Box::new(rhs),
                        op,
                    },
                    ty: Type::Boolean,
                }
            }
        }
        BinaryOp::Boolean(op) => {
            if lhs.ty != Type::Boolean {
                diagnostics.push(Diagnostic::CantCoerceType {
                    range: lspan,
                    from: lhs.ty.clone(),
                    to: Type::Boolean,
                });
                return None;
            }

            if rhs.ty != Type::Boolean {
                diagnostics.push(Diagnostic::CantCoerceType {
                    range: rspan,
                    from: rhs.ty.clone(),
                    to: Type::Boolean,
                });
                return None;
            }

            TypedExpr {
                kind: TypedExprKind::Boolean {
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                    op,
                },
                ty: Type::Boolean,
            }
        }
        BinaryOp::Assign => {
            let lhs = expr_to_place(lspan, lhs, diagnostics)?;

            if !rhs.coerce_to(&lhs.ty) {
                diagnostics.push(Diagnostic::CantCoerceType {
                    range: rspan,
                    from: rhs.ty,
                    to: lhs.ty,
                });
                return None;
            }

            let ty = lhs.ty.clone();
            TypedExpr {
                kind: TypedExprKind::Assign {
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                },
                ty,
            }
        }
        BinaryOp::ArithmeticAssign(op) => {
            if !rhs.coerce_to(&lhs.ty) {
                diagnostics.push(Diagnostic::CantCoerceType {
                    range: rspan,
                    from: rhs.ty,
                    to: lhs.ty,
                });
                return None;
            }

            let lhs = expr_to_place(lspan, lhs, diagnostics)?;
            let ty = lhs.ty.clone();
            TypedExpr {
                kind: TypedExprKind::ArithmeticAssign {
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                    op,
                },
                ty: ty.clone(),
            }
        }
        _ => unreachable!(),
    })
}

enum UnsizedVals {
    Int(i128, i128),
    Float(f64, f64),
}

fn check_unsized_consts(lhs: &TypedExpr, rhs: &TypedExpr) -> Option<UnsizedVals> {
    match (&lhs.kind, &rhs.kind) {
        (TypedExprKind::IntLiteral(l), TypedExprKind::IntLiteral(r)) => {
            Some(UnsizedVals::Int(*l, *r))
        }
        (TypedExprKind::IntLiteral(l), TypedExprKind::FloatLiteral(r)) => {
            Some(UnsizedVals::Float(*l as f64, *r))
        }
        (TypedExprKind::FloatLiteral(l), TypedExprKind::IntLiteral(r)) => {
            Some(UnsizedVals::Float(*l, *r as f64))
        }
        (TypedExprKind::FloatLiteral(l), TypedExprKind::FloatLiteral(r)) => {
            Some(UnsizedVals::Float(*l, *r))
        }
        _ => None,
    }
}

fn coerce_binary_nums(
    diagnostics: &mut Vec<Diagnostic>,
    lhs: &mut TypedExpr,
    rhs: &mut TypedExpr,
    lspan: Range<usize>,
    rspan: Range<usize>,
) -> Option<()> {
    if lhs.ty.is_int() && rhs.ty.is_int() {
        coerce_binary(
            lspan,
            rspan,
            lhs,
            rhs,
            Type::UnsizedInt,
            Type::I32,
            diagnostics,
        )?;
    } else if lhs.ty.is_float() && rhs.ty.is_float() {
        coerce_binary(
            lspan,
            rspan,
            lhs,
            rhs,
            Type::UnsizedFloat,
            Type::F32,
            diagnostics,
        )?;
    } else {
        diagnostics.push(Diagnostic::CantCoerceBinary {
            range: lspan.start..rspan.end,
            lhs: lhs.ty.clone(),
            rhs: rhs.ty.clone(),
        });
        return None;
    }

    Some(())
}

fn coerce_binary(
    lspan: Range<usize>,
    rspan: Range<usize>,
    lhs: &mut TypedExpr,
    rhs: &mut TypedExpr,
    unsized_ty: Type,
    default_ty: Type,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<()> {
    use Type::*;
    if lhs.ty == rhs.ty {
        if lhs.ty == unsized_ty {
            //TODO some back propagating inference or constant propagation?
            //It's probably where I'd do bounds checking as well
            //I have implemented constant folding using check_unsized_consts,
            //but I'm not too happy about that, should return to it later
            lhs.ty = default_ty.clone();
            rhs.ty = default_ty;
        }
    } else if lhs.coerce_to(&rhs.ty) {
    } else if rhs.coerce_to(&lhs.ty) {
    } else if (lhs.ty == UnsizedFloat && rhs.ty == UnsizedInt)
        || (rhs.ty == UnsizedFloat && lhs.ty == UnsizedInt)
    {
        //TODO: I'm not too sure if this check should be here
        lhs.ty = F32;
        rhs.ty = F32;
    } else {
        diagnostics.push(Diagnostic::CantCoerceBinary {
            range: lspan.start..rspan.end,
            lhs: lhs.ty.clone(),
            rhs: rhs.ty.clone(),
        });
        return None;
    }
    Some(())
}

fn expr_to_place(
    range: Range<usize>,
    expr: TypedExpr,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Place> {
    Some(match expr.kind {
        TypedExprKind::Deref(typed_expr) => Place {
            kind: PlaceKind::Deref(typed_expr),
            ty: expr.ty.clone(),
        },
        TypedExprKind::Variable(var) => Place {
            kind: PlaceKind::Variable(var),
            ty: expr.ty.clone(),
        },
        TypedExprKind::Field { place, field } => Place {
            kind: PlaceKind::Field(place, field),
            ty: expr.ty.clone(),
        },
        _ => {
            diagnostics.push(Diagnostic::NotAnLValue { range });
            return None;
        }
    })
}
