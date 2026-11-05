use std::{iter::Peekable, ops::Range, slice::Iter};

use crate::{ast::*, diagnostics::Diagnostic, lexer::Token};

type TokenIter<'t, 'a> = &'t mut Peekable<Iter<'a, (Token, Range<usize>)>>;
const ITEMS_FIRST: [Token; 2] = [Token::Fn, Token::Struct];
const FN_BODY_FIRST: [Token; 2] = [Token::Semicolon, Token::LBrace];

pub fn parse_tokens<'a>(tokens: Vec<(Token, Range<usize>)>, source: &'a str) -> ParsedFile<'a> {
    let mut ast = ParsedFile {
        functions: Vec::new(),
        structs: Vec::new(),
        source,
        diagnostics: Vec::new(),
    };

    let mut tokens = tokens.iter().peekable();
    while tokens.peek().is_some() {
        if parse_item(&mut ast, &mut tokens).is_none() {
            if tokens.peek().is_none() {
                //TODO idk if this won't produce some false EOF diagnostics sometimes
                ast.diagnostics.push(Diagnostic::EOF);
            }
            skip_to(&mut tokens, &ITEMS_FIRST);
        }
    }

    ast
}

fn parse_item(ast: &mut ParsedFile<'_>, tokens: TokenIter) -> Option<()> {
    let token = tokens.next().unwrap();
    match token.0 {
        Token::Fn => parse_fn(ast, tokens)?,
        Token::Struct => parse_struct(ast, tokens)?,
        _ => {
            ast.diagnostics.push(Diagnostic::UnexpectedToken {
                unexpected: token.0,
                range: token.1.clone(),
                expected: &ITEMS_FIRST,
            });
            return None;
        }
    }
    Some(())
}

// Simple panic mode error recovery
fn skip_to(
    tokens: &mut Peekable<Iter<'_, (Token, Range<usize>)>>,
    expected: &[Token],
) -> Option<()> {
    //TODO possibly match parenthesis
    //maybe instead of treating parenthesis as a normal token, it could be a separate argument with
    //a nesting integer added so that I can skip more nested parentheses
    //actually I don't need a param, I can just start with a counter = 0 and just stop when it
    //becomes negative, even if a closing parenthesis wasn't in the expected tokens
    //but then my expectation that it's either ; or } wouldn't hold in parse_scope
    while !expected.contains(&tokens.peek()?.0) {
        tokens.next();
    }
    Some(())
}

fn expect(
    expected: Token,
    ast: &mut ParsedFile<'_>,
    tokens: TokenIter,
) -> Option<(Token, Range<usize>)> {
    let token = tokens.next()?;
    if token.0 == expected {
        return Some(token.clone());
    }

    ast.diagnostics.push(Diagnostic::UnexpectedToken2 {
        unexpected: token.0,
        range: token.1.clone(),
        expected: expected,
    });
    None
}

fn parse_struct(ast: &mut ParsedFile<'_>, tokens: TokenIter) -> Option<()> {
    let ident = expect(Token::Ident, ast, tokens)?;
    expect(Token::LBrace, ast, tokens)?;
    let fields = parse_field_decl_list(ast, tokens, Token::RBrace)?;

    ast.structs.push(StructDecl {
        ident: ident.1,
        fields,
    });
    Some(())
}

/// Parse comma separated `ident: type` declarations
fn parse_field_decl_list(
    ast: &mut ParsedFile<'_>,
    tokens: TokenIter,
    end: Token,
) -> Option<Vec<FieldDecl>> {
    let mut fields = Vec::new();
    //TODO This could be just a loop {} I think
    while tokens.peek()?.0 != end {
        if let Some(field) = parse_field_decl(ast, tokens) {
            fields.push(field);
        } else {
            skip_to(tokens, &[Token::Comma, end])?;
        }

        if tokens.peek()?.0 == end {
            break;
        }

        expect(Token::Comma, ast, tokens);
    }
    tokens.next();

    Some(fields)
}

fn parse_field_decl(ast: &mut ParsedFile<'_>, tokens: TokenIter) -> Option<FieldDecl> {
    let ident = expect(Token::Ident, ast, tokens)?;
    expect(Token::Colon, ast, tokens)?;
    let ty = parse_type(ast, tokens)?.0;

    Some(FieldDecl { ident: ident.1, ty })
}

fn parse_type(ast: &mut ParsedFile<'_>, tokens: TokenIter) -> Option<(Type, Range<usize>)> {
    let token = tokens.next()?;
    let ty = match token.0 {
        Token::Ident => Type::Struct(token.1.clone()),
        Token::Void => Type::Void,
        Token::I8 => Type::I8,
        Token::U8 => Type::U8,
        Token::I16 => Type::I16,
        Token::U16 => Type::U16,
        Token::I32 => Type::I32,
        Token::U32 => Type::U32,
        Token::I64 => Type::I64,
        Token::U64 => Type::U64,
        Token::F32 => Type::F32,
        Token::F64 => Type::F64,
        Token::Bool => Type::Boolean,
        Token::Char => Type::Char,
        Token::Asterisk => {
            let ty = parse_type(ast, tokens)?;
            return Some((Type::Pointer(Box::new(ty.0)), token.1.start..ty.1.end));
        }
        _ => {
            ast.diagnostics.push(Diagnostic::ExpectedType {
                unexpected: token.0,
                range: token.1.clone(),
            });
            return None;
        }
    };

    Some((ty, token.1.clone()))
}

fn parse_fn(ast: &mut ParsedFile<'_>, tokens: TokenIter) -> Option<()> {
    let ident = expect(Token::Ident, ast, tokens)?;
    expect(Token::LParen, ast, tokens)?;
    let args = parse_field_decl_list(ast, tokens, Token::RParen)?;
    let return_ty = match parse_fn_return_type(ast, tokens) {
        Some(r) => r,
        None => {
            skip_to(tokens, &FN_BODY_FIRST)?;
            Type::Void
        }
    };

    let token = tokens.next()?;
    let body = match token.0 {
        Token::Semicolon => None,
        Token::LBrace => parse_scope(ast, tokens),
        _ => {
            ast.diagnostics.push(Diagnostic::UnexpectedToken {
                unexpected: token.0,
                range: token.1.clone(),
                expected: &FN_BODY_FIRST,
            });
            // Skip this function entirely
            skip_to(tokens, &ITEMS_FIRST)?;
            return Some(());
        }
    };

    ast.functions.push(FnDecl {
        ident: ident.1,
        return_ty,
        args,
        body,
    });
    Some(())
}

fn parse_fn_return_type(ast: &mut ParsedFile<'_>, tokens: TokenIter) -> Option<Type> {
    if tokens.peek()?.0 != Token::Arrow {
        return Some(Type::Void);
    }

    tokens.next();
    Some(parse_type(ast, tokens)?.0)
}

fn parse_scope(ast: &mut ParsedFile<'_>, tokens: TokenIter) -> Option<Vec<Statement>> {
    let mut statements: Vec<Statement> = Vec::new();
    while tokens.peek()?.0 != Token::RBrace {
        let mut skip_semi = false;
        if let Some((stmt, skip)) = parse_statement(ast, tokens) {
            statements.push(stmt);
            skip_semi = skip;
        } else {
            skip_to(tokens, &[Token::Semicolon, Token::RBrace])?;
        }

        if tokens.peek().unwrap().0 == Token::RBrace {
            break;
        }

        if !skip_semi {
            expect(Token::Semicolon, ast, tokens);
        }
    }
    tokens.next();

    Some(statements)
}

fn parse_statement(ast: &mut ParsedFile<'_>, tokens: TokenIter) -> Option<(Statement, bool)> {
    let mut skip_semi = false;
    let token = tokens.peek()?;
    let stmt = match token.0 {
        Token::Let => {
            tokens.next();
            let ident = expect(Token::Ident, ast, tokens)?;
            let ty = if tokens.peek()?.0 == Token::Colon {
                tokens.next();
                Some(parse_type(ast, tokens)?.0)
            } else {
                None
            };

            let expr = if tokens.peek()?.0 == Token::Eq {
                tokens.next();
                Some(parse_expr(ast, tokens, false)?)
            } else {
                None
            };

            Statement::Let {
                ident: ident.1,
                expr,
                ty,
            }
        }
        Token::If => {
            tokens.next();
            let expr = parse_expr(ast, tokens, true)?;
            expect(Token::LBrace, ast, tokens)?;
            let body = parse_scope(ast, tokens)?;
            let else_body = if tokens.peek()?.0 == Token::Else {
                tokens.next();
                expect(Token::LBrace, ast, tokens)?;
                let else_body = parse_scope(ast, tokens)?;
                // expect(Token::RBrace, ast, tokens)?;
                Some(else_body)
            } else {
                None
            };

            skip_semi = true;
            Statement::If {
                expr,
                body,
                else_body,
            }
        }
        Token::While => {
            tokens.next();
            let expr = parse_expr(ast, tokens, true)?;
            expect(Token::LBrace, ast, tokens)?;
            let body = parse_scope(ast, tokens)?;

            skip_semi = true;
            Statement::While { expr, body }
        }
        Token::Return => {
            tokens.next();
            let expr = if tokens.peek()?.0 == Token::Semicolon {
                None
            } else {
                Some(parse_expr(ast, tokens, false)?)
            };
            Statement::Return(expr)
        }
        Token::Break => {
            tokens.next();
            Statement::Break
        }
        Token::Continue => {
            tokens.next();
            Statement::Continue
        }
        _ => Statement::Expr(parse_expr(ast, tokens, false)?),
    };

    Some((stmt, skip_semi))
}

// Struct literals cause ambiguity in cases such as `if identifier {` so they are disabled in
// conditions
fn parse_expr(ast: &mut ParsedFile<'_>, tokens: TokenIter, condition: bool) -> Option<Expr> {
    parse_expr_int(ast, tokens, 0, condition)
}

fn parse_expr_int(
    ast: &mut ParsedFile<'_>,
    tokens: TokenIter,
    min_bp: u8,
    condition: bool,
) -> Option<Expr> {
    let unary_op_token = tokens.peek()?;
    let unary_op = match unary_op_token.0 {
        Token::Minus => Some(UnaryOp::Minus),
        Token::Plus => Some(UnaryOp::Plus),
        Token::Bang => Some(UnaryOp::Not),
        Token::Ampersand => Some(UnaryOp::AddressOf),
        Token::Asterisk => Some(UnaryOp::Deref),
        _ => None,
    };

    // Handle unary op binding power so that !a.b works correctly
    let mut lhs = if let Some(op) = unary_op {
        let token = tokens.next().unwrap();
        let expr = parse_expr_int(ast, tokens, UNARY_RBP, condition)?;
        let span = token.1.start..expr.span.end;
        Expr {
            kind: ExprKind::Unary {
                expr: Box::new(expr),
                op,
            },
            span,
        }
    } else {
        parse_expr_atom(ast, tokens, condition)?
    };

    while let Some(op) = parse_binary_op(tokens) {
        let (lbp, rbp) = binding_power(op);
        if lbp < min_bp {
            break;
        }
        tokens.next();

        match op {
            BinaryOp::Index => {
                let index = parse_expr_int(ast, tokens, rbp, condition);
                let Some(index) = index else {
                    skip_to(tokens, &[Token::RBracket]);
                    continue;
                };
                let token = expect(Token::RBracket, ast, tokens)?;

                let span = lhs.span.start..token.1.end;
                lhs = Expr {
                    kind: ExprKind::Index {
                        expr: Box::new(lhs),
                        index: Box::new(index),
                    },
                    span,
                }
            }
            BinaryOp::Call => {
                let mut args = Vec::new();

                //TODO This could be just a loop {} I think
                while tokens.peek()?.0 != Token::RParen {
                    if let Some(expr) = parse_expr(ast, tokens, condition) {
                        args.push(expr);
                    } else {
                        skip_to(tokens, &[Token::Comma, Token::RParen])?;
                    }

                    if tokens.peek()?.0 == Token::RParen {
                        break;
                    }

                    expect(Token::Comma, ast, tokens);
                }
                let token = tokens.next()?;
                let span = lhs.span.start..token.1.end;

                lhs = Expr {
                    kind: ExprKind::Call {
                        lhs: Box::new(lhs),
                        args,
                    },
                    span,
                }
            }
            BinaryOp::FieldAccess => {
                let token = expect(Token::Ident, ast, tokens)?;
                let span = lhs.span.start..token.1.end;
                lhs = Expr {
                    kind: ExprKind::Field {
                        expr: Box::new(lhs),
                        member: token.1.clone(),
                    },
                    span,
                }
            }
            BinaryOp::Cast => {
                let ty = parse_type(ast, tokens)?;
                let span = lhs.span.start..ty.1.end;
                lhs = Expr {
                    kind: ExprKind::Cast {
                        expr: Box::new(lhs),
                        ty: ty.0,
                    },
                    span,
                }
            }
            _ => {
                let rhs = parse_expr_int(ast, tokens, rbp, condition)?;

                let span = lhs.span.start..rhs.span.end;
                lhs = Expr {
                    kind: ExprKind::Binary {
                        lhs: Box::new(lhs),
                        rhs: Box::new(rhs),
                        op,
                    },
                    span,
                }
            }
        }
    }

    Some(lhs)
}

const UNARY_RBP: u8 = 13;

fn binding_power(op: BinaryOp) -> (u8, u8) {
    use ArithmeticOp::*;
    use BinaryOp::*;
    match op {
        FieldAccess => (16, 15),
        Index | Call => (14, 0),
        // Unary is above cast, but below indexing
        Cast => (11, 10),
        Arithmetic(Mul | Div | Rem) => (9, 8),
        Arithmetic(Add | Sub) => (7, 6),
        Comparison(_) => (5, 4),
        Boolean(_) => (3, 2),
        Assign | ArithmeticAssign(_) => (0, 1),
    }
}

fn parse_binary_op(tokens: TokenIter) -> Option<BinaryOp> {
    use ArithmeticOp::*;
    use BinaryOp::*;
    use BooleanOp::*;
    use ComparisonOp::*;
    match tokens.peek()?.0 {
        Token::Plus => Some(Arithmetic(Add)),
        Token::Minus => Some(Arithmetic(Sub)),
        Token::Asterisk => Some(Arithmetic(Mul)),
        Token::Slash => Some(Arithmetic(Div)),
        Token::Percent => Some(Arithmetic(Rem)),

        Token::EqEq => Some(Comparison(Eq)),
        Token::Gt => Some(Comparison(Greater)),
        Token::Lt => Some(Comparison(Less)),
        Token::Gte => Some(Comparison(GreaterEq)),
        Token::Lte => Some(Comparison(LessEq)),
        Token::NotEq => Some(Comparison(NotEq)),

        Token::And => Some(Boolean(And)),
        Token::Or => Some(Boolean(Or)),

        Token::Eq => Some(Assign),

        Token::PlusEq => Some(ArithmeticAssign(Add)),
        Token::MinusEq => Some(ArithmeticAssign(Sub)),
        Token::AsteriskEq => Some(ArithmeticAssign(Mul)),
        Token::SlashEq => Some(ArithmeticAssign(Div)),
        Token::PercentEq => Some(ArithmeticAssign(Rem)),

        Token::LBracket => Some(Index),
        Token::LParen => Some(Call),
        Token::Period => Some(FieldAccess),
        Token::As => Some(Cast),

        _ => None,
    }
}

fn parse_expr_atom(ast: &mut ParsedFile<'_>, tokens: TokenIter, condition: bool) -> Option<Expr> {
    let literal_token = tokens.peek()?;
    let expr = match literal_token.0 {
        Token::IntLit => Some(ExprKind::IntLiteral(literal_token.1.clone())),
        Token::CharLit => Some(ExprKind::CharLiteral(literal_token.1.clone())),
        Token::True => Some(ExprKind::BoolLiteral(true)),
        Token::False => Some(ExprKind::BoolLiteral(false)),
        Token::FloatLit => Some(ExprKind::FloatLiteral(literal_token.1.clone())),
        Token::StringLiteral => Some(ExprKind::StringLiteral(literal_token.1.clone())),
        Token::Ident => {
            let ident = literal_token.1.clone();

            tokens.next();
            if !condition && tokens.peek()?.0 == Token::LBrace {
                tokens.next();

                let mut fields = Vec::new();

                //TODO This could be just a loop {} I think
                while tokens.peek()?.0 != Token::RBrace {
                    if let Some(field) = parse_field_expr(ast, tokens) {
                        fields.push(field);
                    } else {
                        skip_to(tokens, &[Token::Comma, Token::RBrace])?;
                    }

                    if tokens.peek()?.0 == Token::RBrace {
                        break;
                    }

                    expect(Token::Comma, ast, tokens);
                }
                let token = tokens.next()?;

                let span = ident.start..token.1.end;
                return Some(Expr {
                    kind: ExprKind::StructLiteral { ident, fields },
                    span,
                });
            }
            return Some(Expr {
                kind: ExprKind::Identifier(ident.clone()),
                span: ident.clone(),
            });
        }
        Token::LParen => {
            tokens.next();
            let expr = parse_expr(ast, tokens, false)?;
            expect(Token::RParen, ast, tokens)?;
            return Some(expr);
        }
        _ => {
            //TODO expected expression or a statement?
            ast.diagnostics.push(Diagnostic::ExpectedExpression {
                unexpected: literal_token.0,
                range: literal_token.1.clone(),
            });
            None
        }
    };

    if let Some(kind) = expr {
        let span = literal_token.1.clone();
        tokens.next();
        Some(Expr { kind, span })
    } else {
        None
    }
}

fn parse_field_expr(ast: &mut ParsedFile<'_>, tokens: TokenIter) -> Option<(Range<usize>, Expr)> {
    let ident = expect(Token::Ident, ast, tokens)?;
    expect(Token::Colon, ast, tokens)?;
    let expr = parse_expr(ast, tokens, false)?;

    Some((ident.1, expr))
}
