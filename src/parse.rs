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
    // for expr in &exprs {
    //     println!("    {:?}", expr);
    // }
    // println!();
    Ok(items)
}

struct Parser {
    tokens: Vec<Token>,
    curr: usize,
}

impl Parser {
    fn increment(&mut self) {
        self.curr += 1;
        while let Some(Token {
            tt: TT::Whitespace, ..
        }) = self.peek()
        {
            self.curr += 1;
        }
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
            CompileError::ExpectedFound(expectation.to_string(), format!("`{}`", token.tt)).at(span)
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
            if self.match_token(TT::Comma).is_none() {
                break;
            }
        }
        Ok(items)
    }
    fn item(&mut self) -> CompileResult<Option<Item>> {
        let comment = self.comment();
        Ok(Some(if let Some(expr) = self.op_expr()? {
            self.match_token(TT::Newline);
            Item::Expr(ExprItem {
                expr: expr.unparen(),
                comment,
            })
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
    fn op_expr(&mut self) -> CompileResult<Option<OpExpr>> {
        Ok(match self.mod_or_val_expr()? {
            // val, bin, or fork
            Some(ModOrValExpr::Val(w)) => {
                // println!("first is val {:?}", w);
                Some(match self.mod_expr()? {
                    // bin or fork
                    Some(op) => {
                        // println!("second is op {:?}", op);
                        let x = self
                            .expect_with(
                                format!("right tine or {}'s x argument", op),
                                Self::op_expr,
                            )?
                            .unparen();
                        match x {
                            // fork
                            OpExpr::Op(x) => {
                                // println!("third is op {:?}", x);
                                OpExpr::Fork(
                                    ForkExpr {
                                        left: ModOrValExpr::Val(w),
                                        center: op,
                                        right: x,
                                    }
                                    .into(),
                                )
                            }
                            // bin
                            x => {
                                // println!("third is expr {:?}", x);
                                OpExpr::Bin(BinOpExpr { op, w, x }.into())
                            }
                        }
                    }
                    // val
                    None => {
                        // println!("no second");
                        OpExpr::Val(w)
                    }
                })
            }
            // op, un, atop, or fork
            Some(ModOrValExpr::Mod(op)) => Some(match self.op_expr()?.map(OpExpr::unparen) {
                // atop
                Some(OpExpr::Op(g)) => OpExpr::Atop(AtopExpr { f: op, g }.into()),
                // fork
                Some(OpExpr::Atop(atop)) => OpExpr::Fork(
                    ForkExpr {
                        left: ModOrValExpr::Mod(op),
                        center: atop.f,
                        right: atop.g,
                    }
                    .into(),
                ),
                // un
                Some(x) => OpExpr::Un(UnOpExpr { op, x }.into()),
                // op
                None => OpExpr::Op(op),
            }),
            None => None,
        })
    }
    fn mod_expr(&mut self) -> CompileResult<Option<ModExpr>> {
        Ok(if let Some(m) = self.match_to(un_mod) {
            // Unary
            let mut f = self.expect_with(format!("{} f argument", m), Self::mod_or_val_expr)?;
            Some(ModExpr::Un(
                UnModExpr {
                    m: *m,
                    span: m.span,
                    f,
                }
                .into(),
            ))
        } else if let Some(m) = self.match_to(bin_mod) {
            // Binary
            let mut f = self.expect_with(format!("{}'s f argument", m), Self::mod_or_val_expr)?;
            let mut g = self.expect_with(format!("{}'s g argument", m), Self::mod_or_val_expr)?;
            Some(ModExpr::Bin(
                BinModExpr {
                    m: *m,
                    span: m.span,
                    f,
                    g,
                }
                .into(),
            ))
        } else {
            self.match_to(op).map(ModExpr::Op)
        })
    }
    fn mod_or_val_expr(&mut self) -> CompileResult<Option<ModOrValExpr>> {
        Ok(if let Some(expr) = self.mod_expr()? {
            Some(ModOrValExpr::Mod(expr))
        } else {
            self.val_expr()?.map(ModOrValExpr::Val)
        })
    }
    fn val_expr(&mut self) -> CompileResult<Option<ValExpr>> {
        let first = if let Some(expr) = self.single_val_expr()? {
            expr
        } else {
            return Ok(None);
        };
        let mut items = vec![first];
        while self.match_token(TT::Undertie).is_some() {
            let item = self.expect_with("array item", Self::single_val_expr)?;
            items.push(item);
        }
        Ok(Some(if items.len() == 1 {
            items.swap_remove(0)
        } else {
            let span = items[0].span().join(items.last().unwrap().span());
            ValExpr::Array(ArrayExpr {
                items: items.into_iter().map(OpExpr::Val).collect(),
                tied: true,
                span,
            })
        }))
    }
    fn single_val_expr(&mut self) -> CompileResult<Option<ValExpr>> {
        Ok(Some(if let Some(num) = self.match_to(num) {
            ValExpr::Num(num)
        } else if let Some(c) = self.match_to(char) {
            ValExpr::Char(c)
        } else if let Some(s) = self.match_to(string) {
            ValExpr::String(s)
        } else if self.match_token(TT::OpenParen).is_some() {
            let expr = self.expect_with("expression", Self::op_expr)?.unparen();
            self.expect_token(TT::CloseParen)?;
            match expr {
                OpExpr::Val(expr) => expr,
                expr => ValExpr::Parened(expr.into()),
            }
        } else if let Some(open) = self.match_token(TT::OpenAngle) {
            let mut items = Vec::new();
            while let Some(item) = self.op_expr()? {
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
