use std::{fmt::Display, fs, path::Path, rc::Rc};

use crate::{
    ast::*,
    error::{CompileError, CompileResult, IoError},
    lex::*,
    num::Num,
    op::*,
};

pub fn parse<P>(input: &str, file: P) -> CompileResult<Vec<Item>>
where
    P: AsRef<Path>,
{
    let tokens = lex(input, &file)?;
    let mut parser = Parser { tokens, curr: 0 };
    parser.skip_whitespace();
    let items = parser.items()?;
    if let Some(token) = parser.next() {
        return Err(
            CompileError::ExpectedFound("item".into(), token.span.as_string()).at(token.span),
        );
    }
    // Write back to file
    let formatted: String = items.iter().map(|item| item.to_string()).collect();
    if formatted != input {
        return parse(&formatted, file);
    }
    if let Err(error) = fs::write(&file, &formatted) {
        return Err(CompileError::IO(IoError {
            message: format!("Unable to format `{}`", file.as_ref().to_string_lossy()),
            error,
        })
        .at(Span::dud()));
    }
    Ok(items)
}

struct Parser {
    tokens: Vec<Token>,
    curr: usize,
}

impl Parser {
    fn skip_whitespace(&mut self) {
        while let Some(Token {
            tt: TT::Whitespace, ..
        }) = self.peek()
        {
            self.curr += 1;
        }
    }
    fn increment(&mut self) {
        self.curr += 1;
        self.skip_whitespace();
    }
    fn match_to<F, T>(&mut self, f: F) -> Option<Sp<T>>
    where
        F: FnOnce(&TT) -> Option<T>,
    {
        let token = self.tokens.get(self.curr)?;
        let val = f(&token.tt)?;
        let span = token.span.clone();
        self.increment();
        Some(span.sp(val))
    }
    fn match_if<F>(&mut self, f: F) -> Option<Token>
    where
        F: Fn(&TT) -> bool,
    {
        let token = self.tokens.get(self.curr)?;
        if !f(&token.tt) {
            return None;
        }
        let token = token.clone();
        self.increment();
        Some(token)
    }
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.curr)
    }
    fn next(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.curr).cloned()?;
        self.increment();
        Some(token)
    }
    fn match_token(&mut self, token_type: TT) -> Option<Token> {
        self.match_if(|tt| &token_type == tt)
    }
    fn expect<S, T>(&self, expectation: S, op: Option<T>) -> CompileResult<T>
    where
        S: Display,
    {
        op.ok_or_else(|| {
            let token = self.peek().or_else(|| self.tokens.last()).unwrap();
            let span = token.span.clone();
            CompileError::ExpectedFound(expectation.to_string(), format!("`{:?}`", token.tt))
                .at(span)
        })
    }
    fn expect_with<S, F, T>(&mut self, expectation: S, f: F) -> CompileResult<T>
    where
        S: Display,
        F: Fn(&mut Self) -> CompileResult<Option<T>>,
    {
        let val = f(self)?;
        self.expect(expectation, val)
    }
    fn expect_token(&mut self, tt: TT) -> CompileResult<Token> {
        let expectation = format!("`{:?}`", tt);
        let token = self.match_token(tt);
        self.expect(&expectation, token)
    }
    fn expect_token_or<S>(&mut self, tt: TT, or: S) -> CompileResult<Token>
    where
        S: Display,
    {
        let expectation = format!("{} or `{:?}`", or, tt);
        let token = self.match_token(tt);
        self.expect(&expectation, token)
    }
    fn items(&mut self) -> CompileResult<Vec<Item>> {
        let mut items = Vec::new();
        while let Some(item) = self.item()? {
            items.push(item);
        }
        Ok(items)
    }
    fn item(&mut self) -> CompileResult<Option<Item>> {
        let comment = self.comment();
        Ok(Some(if let Some(expr) = self.top_expr()? {
            self.match_token(TT::Newline);
            Item::Expr(ExprItem { expr, comment })
        } else if let Some(comment) = comment {
            self.match_token(TT::Newline);
            Item::Comment(comment)
        } else if self.newline() {
            Item::Newline
        } else {
            return Ok(None);
        }))
    }
    fn newline(&mut self) -> bool {
        let mut newline = false;
        while self.match_token(TT::Newline).is_some() {
            newline = true;
        }
        newline
    }
    fn comment(&mut self) -> Option<Comment> {
        self.match_to(comment).map(|comment| comment.data)
    }
    fn top_expr(&mut self) -> CompileResult<Option<Expr>> {
        Ok(Some(if let Some(expr) = self.expr()? {
            expr
        } else {
            return Ok(None);
        }))
    }
    fn assign(&mut self, name: Ident, op: AssignOp, span: Span) -> CompileResult<AssignExpr> {
        let body = self.expect_with("expression", Self::top_expr)?;
        Ok(AssignExpr {
            name,
            op,
            body,
            span,
        })
    }
    fn expr(&mut self) -> CompileResult<Option<Expr>> {
        Ok(Some(if let Some(expr) = self.function_or_value_expr()? {
            expr
        } else if let Some(expr) = self.un_mod_expr()? {
            expr
        } else if let Some(expr) = self.bin_mod_expr()? {
            expr
        } else {
            return Ok(None);
        }))
    }
    fn un_mod_expr(&mut self) -> CompileResult<Option<Expr>> {
        if let Some(m) = self.match_to(un_mod) {
            Ok(Some(Expr::UnMod(m)))
        } else {
            self.role_term(Role::UnModifier)
        }
    }
    fn bin_mod_expr(&mut self) -> CompileResult<Option<Expr>> {
        if let Some(m) = self.match_to(bin_mod) {
            Ok(Some(Expr::BinMod(m)))
        } else {
            self.role_term(Role::BinModifier)
        }
    }
    fn function_or_value_expr(&mut self) -> CompileResult<Option<Expr>> {
        Ok(Some(if let Some(first) = self.function_term()? {
            // unary, train, or function
            if let Some(second) = self.function_term()? {
                // double unary or train
                if let Some(right) = self.function_or_value_expr()? {
                    // double unary or fork
                    if let Role::Value = right.role() {
                        // double unary
                        Expr::un(first, Expr::un(second, right))
                    } else {
                        // fork
                        Expr::bin(second, first, right, BinKind::Fork)
                    }
                } else {
                    // atop
                    Expr::un(first, second)
                }
            } else if let Some(inner) = self.function_or_value_expr()? {
                // unary
                Expr::un(first, inner)
            } else {
                // function
                first
            }
        } else if let Some(left) = self.value_term()? {
            // binary, fork, or value
            if let Some(op) = self.function_term()? {
                // binary or fork
                let right = self.expect_with("expression", Self::function_or_value_expr)?;
                let kind = if let Role::Value = right.role() {
                    BinKind::Function
                } else {
                    BinKind::Fork
                };
                Expr::bin(op, left, right, kind)
            } else if let Some(other) = self.expr()? {
                // error: double subjects
                return Err(
                    CompileError::InvalidRole(other.role(), vec![Role::Function])
                        .at(other.span().clone()),
                );
            } else {
                // value
                left
            }
        } else {
            return Ok(None);
        }))
    }
    fn function_or_value_term(&mut self) -> CompileResult<Option<Expr>> {
        Ok(Some(if let Some(function) = self.function_term()? {
            function
        } else if let Some(value) = self.value_term()? {
            value
        } else {
            return Ok(None);
        }))
    }
    fn function_term(&mut self) -> CompileResult<Option<Expr>> {
        let start = self.curr;
        if let Some(op) = self.un_mod_expr()? {
            if let Some(inner) = self.function_or_value_term()? {
                return Ok(Some(Expr::un(op, inner)));
            }
        } else if let Some(op) = self.bin_mod_expr()? {
            if let Some(left) = self.function_or_value_term()? {
                let right = self.expect_with(
                    format!("{}'s second argument", op),
                    Self::function_or_value_term,
                )?;
                return Ok(Some(Expr::bin(op, left, right, BinKind::Modifier)));
            }
        } else if let Some(op) = self.match_to(op) {
            return Ok(Some(Expr::Op(op)));
        } else if let Some(op) = self.role_term(Role::Function)? {
            return Ok(Some(op));
        }
        self.curr = start;
        Ok(None)
    }
    fn value_term(&mut self) -> CompileResult<Option<Expr>> {
        Ok(Some(if let Some(expr) = self.constant()? {
            expr
        } else if let Some(expr) = self.role_term(Role::Value)? {
            expr
        } else {
            return Ok(None);
        }))
    }
    fn constant(&mut self) -> CompileResult<Option<Expr>> {
        Ok(Some(if let Some(num) = self.match_to(num) {
            Expr::Num(num)
        } else if let Some(char) = self.match_to(char) {
            Expr::Char(char)
        } else if let Some(string) = self.match_to(string) {
            Expr::String(string)
        } else if let Some(expr) = self.array()? {
            expr
        } else {
            return Ok(None);
        }))
    }
    fn role_term(&mut self, role: Role) -> CompileResult<Option<Expr>> {
        let start = self.curr;
        let expr = if let Some(expr) = self.parened()? {
            expr
        } else if let Some(expr) = self.function_literal()? {
            expr
        } else if let Some(ident) = self.match_to(ident) {
            if let Some(op) = self.match_to(assign_op) {
                let body = self.expect_with("expression", Self::expr)?;
                Expr::Assign(
                    AssignExpr {
                        name: ident.data,
                        op: op.data,
                        body,
                        span: ident.span,
                    }
                    .into(),
                )
            } else {
                Expr::Ident(ident)
            }
        } else if let Some(param) = self.match_to(param) {
            Expr::Param(param)
        } else {
            return Ok(None);
        };
        Ok(if expr.role() == role {
            Some(expr)
        } else {
            self.curr = start;
            None
        })
    }
    fn parened(&mut self) -> CompileResult<Option<Expr>> {
        if self.match_token(TT::OpenParen).is_none() {
            return Ok(None);
        }
        let expr = self.expect_with("expression", Self::expr)?;
        self.expect_token(TT::CloseParen)?;
        Ok(Some(Expr::Parened(expr.into())))
    }
    fn function_literal(&mut self) -> CompileResult<Option<Expr>> {
        if self.match_token(TT::OpenAngleDot).is_none() {
            return Ok(None);
        }
        let expr = self.expect_with("function body", Self::expr)?;
        self.expect_token(TT::CloseAngleDot)?;
        Ok(Some(Expr::Function(expr.into())))
    }
    fn array(&mut self) -> CompileResult<Option<Expr>> {
        let open = if let Some(token) = self.match_token(TT::OpenAngle) {
            token
        } else {
            return Ok(None);
        };
        let mut items = Vec::new();
        loop {
            let start = self.curr;
            let expr = if let Ok(Some(expr)) = self.expr() {
                expr
            } else {
                self.curr = start;
                if let Some(expr) = self.function_or_value_term()? {
                    expr
                } else {
                    break;
                }
            };
            let comma = self.match_token(TT::Comma).is_some();
            items.push((expr, comma))
        }
        let close = self.expect_token(TT::CloseAngle)?;
        let span = open.span.join(&close.span);
        Ok(Some(Expr::Array(ArrayExpr { items, span })))
    }
}

fn num(tt: &TT) -> Option<Num> {
    if let TT::Num(num, _) = tt {
        Some(*num)
    } else {
        None
    }
}

fn char(tt: &TT) -> Option<char> {
    if let TT::Char(c) = tt {
        Some(*c)
    } else {
        None
    }
}

fn string(tt: &TT) -> Option<Rc<str>> {
    if let TT::String(s) = tt {
        Some(s.clone())
    } else {
        None
    }
}

fn op(tt: &TT) -> Option<Op> {
    if let TT::Op(op) = tt {
        Some(*op)
    } else {
        None
    }
}

fn un_mod(tt: &TT) -> Option<RuneUnMod> {
    if let TT::UnMod(m) = tt {
        Some(*m)
    } else {
        None
    }
}

fn bin_mod(tt: &TT) -> Option<RuneBinMod> {
    if let TT::BinMod(m) = tt {
        Some(*m)
    } else {
        None
    }
}

fn comment(tt: &TT) -> Option<Comment> {
    if let TT::Comment(comment) = tt {
        Some(comment.clone())
    } else {
        None
    }
}

fn assign_op(tt: &TT) -> Option<AssignOp> {
    if let TT::Assign(op) = tt {
        Some(*op)
    } else {
        None
    }
}

fn ident(tt: &TT) -> Option<Ident> {
    if let TT::Ident(ident) = tt {
        Some(ident.clone())
    } else {
        None
    }
}

fn param(tt: &TT) -> Option<Param> {
    if let TT::Param(param) = tt {
        Some(*param)
    } else {
        None
    }
}
