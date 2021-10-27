use std::{fs::OpenOptions, io::Write, path::Path};

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
    match OpenOptions::new().write(true).truncate(true).open(&file) {
        Ok(mut file) => {
            let mut formatter = Formatter::new(file);
            for item in &items {
                let _ = item.format(&mut formatter);
                let _ = writeln!(formatter);
            }
        }
        Err(error) => {
            return Err(CompileErrorKind::IO(IoError {
                message: format!("Unable to format `{}`", file.as_ref().to_string_lossy()),
                error,
            })
            .at(Span::dud()))
        }
    }
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
        Ok(Some(if let Some(expr) = self.terminal()? {
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
        Ok(Some(if let Some((num, span)) = self.match_to(num) {
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
}
