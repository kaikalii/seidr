use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

use crate::{
    ast::*,
    error::{CompileErrorKind, CompileResult, IoError},
    lex::*,
    num::Num,
    op::Op,
};

pub fn parse<P>(input: &str, file: P) -> CompileResult<Vec<Item>>
where
    P: AsRef<Path>,
{
    let tokens = lex(input, &file)?;
    let mut parser = Parser { tokens, curr: 0 };
    let items = parser.items()?;
    if let Some(token) = parser.next() {
        return Err(
            CompileErrorKind::ExpectedFound("item".into(), token.span.as_string()).at(token.span),
        );
    }
    // Write back to file
    let formatted: String = items.iter().map(|item| format!("{}\n", item)).collect();
    if let Err(error) = fs::write(&file, &formatted) {
        return Err(CompileErrorKind::IO(IoError {
            message: format!("Unable to format `{}`", file.as_ref().to_string_lossy()),
            error,
        })
        .at(Span::dud()));
    }
    println!("items:");
    for item in &items {
        println!("    {:?}", item);
    }
    println!();
    Ok(items)
}

struct Parser {
    tokens: Vec<Token>,
    curr: usize,
}

type Sp<T> = (T, Span);

impl Parser {
    fn skip_whitespace(&mut self) {
        while let Some(Token {
            tt: TT::Whitespace | TT::Newline,
            ..
        }) = self.tokens.get(self.curr)
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
        Some((val, span))
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
    fn expect<T>(&self, expectation: &str, op: Option<T>) -> CompileResult<T> {
        op.ok_or_else(|| {
            let span = self
                .peek()
                .or_else(|| self.tokens.last())
                .unwrap()
                .span
                .clone();
            CompileErrorKind::ExpectedFound(expectation.into(), span.as_string()).at(span)
        })
    }
    fn expect_token(&mut self, tt: TT) -> CompileResult<Token> {
        let expectation = format!("`{}`", tt);
        let token = self.match_token(tt);
        self.expect(&expectation, token)
    }
    fn expect_token_or(&mut self, tt: TT, or: &str) -> CompileResult<Token> {
        let expectation = format!("{} or `{}`", or, tt);
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
        Ok(Some(if let Some(expr) = self.expression(false)? {
            Item::Expr(expr)
        } else {
            return Ok(None);
        }))
    }
    fn expect_expression(&mut self, parened: bool) -> CompileResult<Expr> {
        let expr = self.expression(parened)?;
        self.expect("expression", expr)
    }
    fn expression(&mut self, parened: bool) -> CompileResult<Option<Expr>> {
        fn op(tt: &TT) -> Option<Op> {
            if let TT::Op(op) = tt {
                Some(*op)
            } else {
                None
            }
        }
        Ok(Some(if let Some(expr) = self.tied_array()? {
            if let Some((op, op_span)) = self.match_to(op) {
                let left = expr;
                let right = self.expect_expression(false)?;
                let span = left.span().join(right.span());
                Expr::Bin(
                    BinExpr {
                        op,
                        left,
                        right,
                        op_span,
                        span,
                        parened,
                    }
                    .into(),
                )
            } else {
                expr
            }
        } else if let Some((op, op_span)) = self.match_to(op) {
            let inner = self.expect_expression(false)?;
            let span = op_span.join(inner.span());
            Expr::Un(
                UnExpr {
                    op,
                    inner,
                    span,
                    op_span,
                    parened,
                }
                .into(),
            )
        } else {
            return Ok(None);
        }))
    }
    fn terminal(&mut self) -> CompileResult<Option<Expr>> {
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
        Ok(Some(if let Some(array) = self.bracketed_array()? {
            array
        } else if let Some((num, span)) = self.match_to(num) {
            Expr::Num(num, span)
        } else if let Some((c, span)) = self.match_to(char) {
            Expr::Char(c, span)
        } else if let Some((ident, span)) = self.ident() {
            Expr::Ident(ident, span)
        } else if self.match_token(TT::OpenParen).is_some() {
            let expr = self.expect_expression(true)?;
            self.expect_token(TT::CloseParen)?;
            expr
        } else {
            return Ok(None);
        }))
    }
    fn ident(&mut self) -> Option<(Ident, Span)> {
        fn ident(tt: &TT) -> Option<Ident> {
            if let TT::Ident(ident) = tt {
                Some(ident.clone())
            } else {
                None
            }
        }
        self.match_to(ident)
    }
    fn array(&mut self) -> CompileResult<Option<Expr>> {
        Ok(Some(if let Some(expr) = self.tied_array()? {
            expr
        } else if let Some(expr) = self.bracketed_array()? {
            expr
        } else {
            return Ok(None);
        }))
    }
    fn tied_array(&mut self) -> CompileResult<Option<Expr>> {
        let start = self.curr;
        let first = if let Some(expr) = self.terminal()? {
            expr
        } else {
            return Ok(None);
        };
        let mut items = vec![first];
        while self.match_token(TT::Undertie).is_some() {
            let item = self.terminal()?;
            let item = self.expect("item", item)?;
            items.push(item);
        }
        if items.len() == 1 {
            return Ok(Some(items.swap_remove(0)));
        }
        let span = items[0].span().join(items.last().unwrap().span());
        Ok(Some(Expr::Array(ArrayExpr {
            items,
            tied: true,
            span,
        })))
    }
    fn bracketed_array(&mut self) -> CompileResult<Option<Expr>> {
        let open = if let Some(token) = self.match_token(TT::OpenAngle) {
            token
        } else {
            return Ok(None);
        };
        let first = self.expression(false)?;
        let mut items = Vec::from_iter(first);
        while self.match_token(TT::Comma).is_some() {
            let item = self.terminal()?;
            let item = self.expect("item", item)?;
            items.push(item);
        }
        self.match_token(TT::Comma);
        let close = self.expect_token(TT::CloseAngle)?;
        let span = open.span.join(&close.span);
        Ok(Some(Expr::Array(ArrayExpr {
            items,
            tied: false,
            span,
        })))
    }
}
