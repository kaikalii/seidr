use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
    rc::Rc,
};

use crate::{
    ast2::*,
    error::{CompileError, CompileResult, IoError},
    lex::*,
    num::Num,
    op::Op,
};

pub fn parse<P>(input: &str, file: P) -> CompileResult<Vec<OpTreeExpr>>
where
    P: AsRef<Path>,
{
    let tokens = lex(input, &file)?;
    let mut parser = Parser { tokens, curr: 0 };
    parser.skip_whitespace();
    let exprs = parser.exprs()?;
    if let Some(token) = parser.next() {
        return Err(
            CompileError::ExpectedFound("item".into(), token.span.as_string()).at(token.span),
        );
    }
    // Write back to file
    let formatted: String = exprs.iter().map(|item| format!("{}\n", item)).collect();
    if let Err(error) = fs::write(&file, &formatted) {
        return Err(CompileError::IO(IoError {
            message: format!("Unable to format `{}`", file.as_ref().to_string_lossy()),
            error,
        })
        .at(Span::dud()));
    }
    // println!("items:");
    // for item in &items {
    //     println!("    {:?}", item);
    // }
    // println!();
    Ok(exprs)
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
            let token = self.peek().or_else(|| self.tokens.last()).unwrap();
            let span = token.span.clone();
            CompileError::ExpectedFound(expectation.into(), format!("`{}`", token.tt)).at(span)
        })
    }
    fn expect_with<F, T>(&mut self, expectation: &str, f: F) -> CompileResult<T>
    where
        F: Fn(&mut Self) -> CompileResult<Option<T>>,
    {
        let val = f(self)?;
        self.expect(expectation, val)
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
    fn exprs(&mut self) -> CompileResult<Vec<OpTreeExpr>> {
        let mut exprs = Vec::new();
        while let Some(expr) = self.op_tree_expr()? {
            exprs.push(expr);
        }
        Ok(exprs)
    }
    fn op_tree_expr(&mut self) -> CompileResult<Option<OpTreeExpr>> {
        Ok(Some(if let Some(op) = self.op_expr()? {
            let x = self.expect_with("expression", Self::op_tree_expr)?;
            OpTreeExpr::Un(UnExpr { op, x }.into())
        } else if let Some(w) = self.val_expr()? {
            if let Some(op) = self.op_expr()? {
                let x = self.expect_with("expression", Self::op_tree_expr)?;
                OpTreeExpr::Bin(BinExpr { op, w, x }.into())
            } else {
                OpTreeExpr::Val(w)
            }
        } else {
            return Ok(None);
        }))
    }
    fn val_expr(&mut self) -> CompileResult<Option<ValExpr>> {
        let first = if let Some(expr) = self.single_val_expr()? {
            expr
        } else {
            return Ok(None);
        };
        let mut items = vec![first];
        while self.match_token(TT::Undertie).is_some() {
            let item = self.expect_with("expression", Self::single_val_expr)?;
            items.push(item);
        }
        Ok(Some(if items.len() == 1 {
            items.swap_remove(0)
        } else {
            let span = items[0].span().join(items.last().unwrap().span());
            ValExpr::Array(ArrayExpr {
                items: items.into_iter().map(OpTreeExpr::Val).collect(),
                tied: true,
                span,
            })
        }))
    }
    fn single_val_expr(&mut self) -> CompileResult<Option<ValExpr>> {
        Ok(Some(if let Some((num, span)) = self.match_to(num) {
            ValExpr::Num(num, span)
        } else if let Some((c, span)) = self.match_to(char) {
            ValExpr::Char(c, span)
        } else if let Some((s, span)) = self.match_to(string) {
            ValExpr::String(s, span)
        } else if self.match_token(TT::OpenParen).is_some() {
            let expr = self.expect_with("expression", Self::op_tree_expr)?;
            self.expect_token(TT::CloseParen);
            ValExpr::Parened(expr.into())
        } else if let Some(open) = self.match_token(TT::OpenAngle) {
            let mut items = Vec::new();
            while let Some(item) = self.op_tree_expr()? {
                items.push(item);
                if self.match_token(TT::Comma).is_none() {
                    break;
                }
            }
            let close = self.expect_token(TT::CloseAngle)?;
            let span = open.span.join(&close.span);
            ValExpr::Array(ArrayExpr {
                items,
                tied: false,
                span,
            })
        } else {
            return Ok(None);
        }))
    }
    fn op_expr(&mut self) -> CompileResult<Option<OpExpr>> {
        Ok(if let Some((op, span)) = self.match_to(op) {
            Some(OpExpr::Op(op, span))
        } else {
            None
        })
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
