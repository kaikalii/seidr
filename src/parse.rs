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
        Ok(Some(if let Some(expr) = self.train()? {
            self.match_token(TT::Newline);
            Item::Function(ExprItem { expr, comment })
        } else if let Some(expr) = self.op_expr()? {
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
        Ok(if let Some(assign) = self.assign::<OpExpr>()? {
            Some(OpExpr::Assign(assign.into()))
        } else {
            match self.mod_or_val_expr()? {
                // un
                Some(ValExpr::Mod(op)) => {
                    let x = self.expect_with(format!("{}'s x argument", op), Self::op_expr)?;
                    // Simplify negative number
                    if let ModExpr::Op(op) = &op {
                        if let Op::Pervasive(Pervasive::Math(MathOp::Sub)) = &**op {
                            if let OpExpr::Val(ValExpr::Num(n)) = &x {
                                return Ok(Some(OpExpr::Val(ValExpr::Num(
                                    n.span.clone().sp(-**n),
                                ))));
                            }
                        }
                    }
                    Some(OpExpr::Un(UnOpExpr { op, x }.into()))
                }
                // val or bin
                Some(w) => Some(match self.mod_expr()? {
                    Some(op) => {
                        let x = self.expect_with(format!("{}'s x argument", op), Self::op_expr)?;
                        OpExpr::Bin(BinOpExpr { op, w, x }.into())
                    }
                    None => OpExpr::Val(w),
                }),
                None => None,
            }
        })
    }
    fn mod_expr(&mut self) -> CompileResult<Option<ModExpr>> {
        // Parened
        if let Some(expr) = self.parened_mod_expr()? {
            return Ok(Some(expr));
        }
        // Op
        if let Some(op) = self.match_to(op) {
            return Ok(Some(ModExpr::Op(op)));
        }
        // Ident
        if let Some(ident) = self.match_to(ident::<TrainExpr>) {
            return Ok(Some(ModExpr::Ident(ident)));
        }
        // Un
        if let Some(m) = self.match_to(un_mod) {
            let f = self.expect_with(format!("{}'s f argument", m), Self::mod_or_val_expr)?;
            return Ok(Some(ModExpr::Un(
                UnModExpr {
                    m: *m,
                    f,
                    span: m.span,
                }
                .into(),
            )));
        }
        // Bin
        if let Some(m) = self.match_to(bin_mod) {
            let f = self.expect_with(format!("{}'s f argument", m), Self::mod_or_val_expr)?;
            let g = self.expect_with(format!("{}'s g argument", m), Self::mod_or_val_expr)?;
            return Ok(Some(ModExpr::Bin(
                BinModExpr {
                    m: *m,
                    f,
                    g,
                    span: m.span,
                }
                .into(),
            )));
        }
        Ok(None)
    }
    fn mod_or_val_expr(&mut self) -> CompileResult<Option<ValExpr>> {
        Ok(if let Some(expr) = self.mod_expr()? {
            Some(ValExpr::Mod(expr))
        } else {
            self.val_expr()?
        })
    }
    fn val_expr(&mut self) -> CompileResult<Option<ValExpr>> {
        Ok(Some(if let Some(ident) = self.match_to(ident::<OpExpr>) {
            ValExpr::Ident(ident)
        } else if let Some(num) = self.match_to(num) {
            ValExpr::Num(num)
        } else if let Some(c) = self.match_to(char) {
            ValExpr::Char(c)
        } else if let Some(s) = self.match_to(string) {
            ValExpr::String(s)
        } else if let Some(val) = self.parened_op_expr()? {
            val
        } else if let Some(array) = self.array()? {
            ValExpr::Array(array)
        } else {
            return Ok(None);
        }))
    }
    fn array(&mut self) -> CompileResult<Option<ArrayExpr>> {
        let open = if let Some(token) = self.match_token(TT::OpenAngle) {
            token
        } else {
            return Ok(None);
        };
        let mut items = Vec::new();
        while let Some(item) = self.array_item()? {
            items.push((item, self.match_token(TT::Comma).is_some()));
        }
        let close = self.expect_token_or(TT::CloseAngle, "array item")?;
        let span = open.span.join(&close.span);
        Ok(Some(ArrayExpr { items, span }))
    }
    fn array_item(&mut self) -> CompileResult<Option<ArrayItemExpr>> {
        Ok(if let Some(expr) = self.train()? {
            Some(ArrayItemExpr::Function(expr))
        } else {
            self.op_expr()?.map(ArrayItemExpr::Val)
        })
    }
    fn parened_op_expr(&mut self) -> CompileResult<Option<ValExpr>> {
        let start = self.curr;
        if self.match_token(TT::OpenParen).is_none() {
            return Ok(None);
        }
        Ok(if let Some(expr) = self.op_expr()? {
            self.expect_token(TT::CloseParen)?;
            Some(match expr {
                OpExpr::Val(expr) => expr,
                expr => ValExpr::Parened(expr.into()),
            })
        } else {
            self.curr = start;
            None
        })
    }
    fn parened_mod_expr(&mut self) -> CompileResult<Option<ModExpr>> {
        let start = self.curr;
        if self.match_token(TT::OpenParen).is_none() {
            return Ok(None);
        }
        Ok(if let Some(train) = self.train()? {
            self.expect_token(TT::CloseParen)?;
            Some(match train {
                TrainExpr::Single(expr) => expr,
                train => ModExpr::Parened(train.into()),
            })
        } else {
            self.curr = start;
            None
        })
    }
    fn train(&mut self) -> CompileResult<Option<TrainExpr>> {
        Ok(if let Some(assign) = self.assign::<TrainExpr>()? {
            Some(TrainExpr::Assign(assign.into()))
        } else if let Some(train) = self.fork_or_single()? {
            Some(train)
        } else {
            self.atop()?.map(Into::into).map(TrainExpr::Atop)
        })
    }
    fn fork_or_single(&mut self) -> CompileResult<Option<TrainExpr>> {
        Ok(Some(if let Some(fork) = self.fork()? {
            TrainExpr::Fork(fork.into())
        } else {
            let start = self.curr;
            let single = if let Some(single) = self.mod_expr()? {
                single
            } else {
                return Ok(None);
            };
            if self.mod_expr()?.is_some() || self.op_expr()?.is_some() {
                self.curr = start;
                return Ok(None);
            }
            TrainExpr::Single(single)
        }))
    }
    fn atop(&mut self) -> CompileResult<Option<AtopExpr>> {
        let start = self.curr;
        let f = if let Some(f) = self.mod_expr()? {
            f
        } else {
            return Ok(None);
        };
        let g = if let Some(g) = self.fork_or_single()? {
            g
        } else {
            self.curr = start;
            return Ok(None);
        };
        Ok(Some(AtopExpr { f, g }))
    }
    fn fork(&mut self) -> CompileResult<Option<ForkExpr>> {
        let start = self.curr;
        let left = if let Some(left) = self.mod_or_val_expr()? {
            left
        } else {
            return Ok(None);
        };
        let center = if let Some(center) = self.mod_expr()? {
            center
        } else {
            self.curr = start;
            return Ok(None);
        };
        let right = if let Some(right) = self.fork_or_single()? {
            right
        } else {
            self.curr = start;
            return Ok(None);
        };
        Ok(Some(ForkExpr {
            left,
            center,
            right,
        }))
    }
    fn assign<T>(&mut self) -> CompileResult<Option<AssignExpr<T>>>
    where
        T: ExprParse,
    {
        let start = self.curr;
        let ident = if let Some(ident) = self.match_to(ident::<T>) {
            ident
        } else {
            return Ok(None);
        };
        let op = if let Some(op) = self.match_to(assign_op) {
            op
        } else {
            self.curr = start;
            return Ok(None);
        };
        let body = self.expect_with(T::EXPECTATION, T::parse)?;
        Ok(Some(AssignExpr {
            name: ident.data,
            op: *op,
            body,
            span: ident.span,
        }))
    }
}

trait ExprParse: Sized {
    const EXPECTATION: &'static str;
    fn ident_matches(ident: &Ident) -> bool;
    fn parse(parser: &mut Parser) -> CompileResult<Option<Self>>;
}

impl ExprParse for OpExpr {
    const EXPECTATION: &'static str = "value";
    fn ident_matches(ident: &Ident) -> bool {
        ident.is_val()
    }
    fn parse(parser: &mut Parser) -> CompileResult<Option<Self>> {
        parser.op_expr()
    }
}

impl ExprParse for TrainExpr {
    const EXPECTATION: &'static str = "function";
    fn ident_matches(ident: &Ident) -> bool {
        ident.is_function()
    }
    fn parse(parser: &mut Parser) -> CompileResult<Option<TrainExpr>> {
        parser.train()
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

fn ident<T>(tt: &TT) -> Option<Ident>
where
    T: ExprParse,
{
    if let TT::Ident(ident) = tt {
        if T::ident_matches(ident) {
            return Some(ident.clone());
        }
    }
    None
}

fn any_ident(tt: &TT) -> Option<Ident> {
    if let TT::Ident(ident) = tt {
        Some(ident.clone())
    } else {
        None
    }
}
