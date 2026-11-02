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
    while tokens.peek()?.0 != end {
        if let Some(field) = parse_field_decl(ast, tokens) {
            fields.push(field);
        } else {
            skip_to(tokens, &[Token::Comma, end])?;
        }

        if tokens.peek().unwrap().0 == end {
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
    let ty = parse_type(ast, tokens)?;

    Some(FieldDecl { ident: ident.1, ty })
}

fn parse_type(ast: &mut ParsedFile<'_>, tokens: TokenIter) -> Option<Type> {
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
        Token::Asterisk => Type::Pointer(Box::new(parse_type(ast, tokens)?)),
        _ => {
            ast.diagnostics.push(Diagnostic::ExpectedType {
                unexpected: token.0,
                range: token.1.clone(),
            });
            return None;
        }
    };

    Some(ty)
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
    parse_type(ast, tokens)
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
                Some(parse_type(ast, tokens)?)
            } else {
                None
            };

            let expr = if tokens.peek()?.0 == Token::Colon {
                tokens.next();
                Some(parse_expr(ast, tokens)?)
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
            let expr = parse_expr(ast, tokens)?;
            expect(Token::LBrace, ast, tokens)?;
            let body = parse_scope(ast, tokens)?;
            let else_body = if tokens.peek()?.0 == Token::Else {
                tokens.next();
                expect(Token::LBrace, ast, tokens)?;
                let else_body = parse_scope(ast, tokens)?;
                expect(Token::RBrace, ast, tokens)?;
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
            let expr = parse_expr(ast, tokens)?;
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
                Some(parse_expr(ast, tokens)?)
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
        _ => Statement::Expr(parse_expr(ast, tokens)?),
    };

    Some((stmt, skip_semi))
}

fn parse_expr(ast: &mut ParsedFile<'_>, tokens: TokenIter) -> Option<Expr> {
    parse_expr_int(ast, tokens, 0)
}

fn parse_expr_int(ast: &mut ParsedFile<'_>, tokens: TokenIter, min_bp: u8) -> Option<Expr> {
    let unary_op_token = tokens.peek()?;
    let unary_op = match unary_op_token.0 {
        Token::Minus => Some(UnaryOp::Minus),
        Token::Plus => Some(UnaryOp::Plus),
        Token::Bang => Some(UnaryOp::Not),
        Token::Ampersand => Some(UnaryOp::AddressOf),
        Token::Asterisk => Some(UnaryOp::Deref),
        _ => None,
    };

    if unary_op.is_some() {
        tokens.next();
    }

    None
}

fn parse_literals_or_member_access(ast: &mut ParsedFile<'_>, tokens: TokenIter) -> Option<Expr> {
    let literal_token = tokens.peek()?;
    let expr = match literal_token.0 {
        Token::IntLit => Some(ExprKind::IntLiteral(literal_token.1.clone())),
        Token::CharLit => Some(ExprKind::CharLiteral(literal_token.1.clone())),
        Token::True => Some(ExprKind::BoolLiteral(true)),
        Token::False => Some(ExprKind::BoolLiteral(false)),
        Token::FloatLit => Some(ExprKind::FloatLiteral(literal_token.1.clone())),
        Token::StringLiteral => Some(ExprKind::StringLiteral(literal_token.1.clone())),
        _ => None,
    };

    if let Some(kind) = expr {
        let span = literal_token.1.clone();
        tokens.next();
        Some(Expr { kind, span })
    } else {
        parse_member_access(ast, tokens)
    }
}

fn parse_member_access(ast: &mut ParsedFile<'_>, tokens: TokenIter) -> Option<Expr> {
    let mut lhs = parse_expr_atom(false)?;

    while tokens.peek()?.0 == Token::Period {
        tokens.next();
        let rhs = parse_expr_atom(true)?;
        lhs = Expr { kind: (), span: () }
    }

    Some(lhs)
}
