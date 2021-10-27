use std::path::Path;

use crate::{
    ast::*,
    error::{CompileErrorKind, CompileResult},
    lex::*,
};

pub fn parse<P>(input: &str, file: P) -> CompileResult<Vec<Item>>
where
    P: AsRef<Path>,
{
    let tokens = lex(input, file)?;
    let mut parser = Parser { tokens, curr: 0 };
    let items = parser.items()?;
    if let Some(token) = parser.next() {
        return Err(
            CompileErrorKind::ExpectedFound("item".into(), token.span.as_string()).at(token.span),
        );
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
            tt: TT::Whitespace, ..
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
        let token = self.tokens.get(self.curr - 1).cloned()?;
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
        Ok(Some(if let Some(expr) = self.expression()? {
            Item::Expr(expr)
        } else {
            return Ok(None);
        }))
    }
    fn expect_expression(&mut self) -> CompileResult<Expr> {
        let expr = self.expression()?;
        self.expect("expression", expr)
    }
    fn expression(&mut self) -> CompileResult<Option<Expr>> {
        todo!()
    }
}
