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
        let a = if let Some(term) = self.term()? {
            term
        } else {
            return Ok(None);
        };
        let b = if let Some(expr) = self.expr()? {
            expr
        } else {
            return Ok(Some(a));
        };
        Ok(Some(match (a.role(), b) {
            (Role::Value, Expr::Un(b)) if b.op.role() == Role::Function => {
                Expr::bin(b.op, a, b.inner)
            }
            (_, b) => Expr::un(a, b),
        }))
    }
    fn term(&mut self) -> CompileResult<Option<Expr>> {
        Ok(Some(if let Some(num) = self.match_to(num) {
            Expr::Num(num)
        } else if let Some(char) = self.match_to(char) {
            Expr::Char(char)
        } else if let Some(string) = self.match_to(string) {
            Expr::String(string)
        } else if let Some(op) = self.match_to(op) {
            Expr::Op(op)
        } else if let Some(un_mod) = self.match_to(un_mod) {
            Expr::UnMod(un_mod)
        } else if let Some(bin_mod) = self.match_to(bin_mod) {
            Expr::BinMod(bin_mod)
        } else if let Some(ident) = self.match_to(ident) {
            Expr::Ident(ident)
        } else if let Some(expr) = self.parened()? {
            expr
        } else if let Some(expr) = self.array()? {
            expr
        } else {
            return Ok(None);
        }))
    }
    fn parened(&mut self) -> CompileResult<Option<Expr>> {
        if self.match_token(TT::OpenParen).is_none() {
            return Ok(None);
        }
        let expr = self.expect_with("expression", Self::expr)?;
        self.expect_token(TT::CloseParen)?;
        Ok(Some(Expr::Parened(expr.into())))
    }
    fn array(&mut self) -> CompileResult<Option<Expr>> {
        let open = if let Some(token) = self.match_token(TT::OpenAngle) {
            token
        } else {
            return Ok(None);
        };
        let mut items = Vec::new();
        fn add_item(items: &mut Vec<(Expr, bool)>, expr: Expr, comma: bool) {
            match expr {
                Expr::Un(un) if [un.op.role(), un.inner.role()] == [Role::Value; 2] => {
                    items.push((un.op, false));
                    add_item(items, un.inner, comma);
                }
                expr => items.push((expr, comma)),
            }
        }
        while let Some(expr) = self.expr()? {
            let comma = self.match_token(TT::Comma).is_some();
            add_item(&mut items, expr, comma);
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

fn ident_if<F>(f: F) -> impl Fn(&TT) -> Option<Ident>
where
    F: Fn(&Ident) -> bool,
{
    move |tt: &TT| {
        if let TT::Ident(ident) = tt {
            if f(ident) {
                Some(ident.clone())
            } else {
                None
            }
        } else {
            None
        }
    }
}
