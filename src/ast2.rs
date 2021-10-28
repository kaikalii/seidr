use std::{fmt, rc::Rc};

use crate::{lex::Span, num::Num, op::Op};

pub enum ValExpr {
    Num(Num, Span),
    Char(char, Span),
    String(Rc<str>, Span),
    Array(ArrayExpr),
    Parened(Box<OpTreeExpr>),
}

impl ValExpr {
    pub fn span(&self) -> &Span {
        match self {
            ValExpr::Char(_, span) | ValExpr::Num(_, span) | ValExpr::String(_, span) => span,
            ValExpr::Array(expr) => &expr.span,
            ValExpr::Parened(expr) => expr.span(),
        }
    }
}

impl fmt::Debug for ValExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValExpr::Num(n, _) => n.fmt(f),
            ValExpr::Char(c, _) => c.fmt(f),
            ValExpr::String(string, _) => string.fmt(f),
            ValExpr::Array(expr) => expr.fmt(f),
            ValExpr::Parened(expr) => expr.fmt(f),
        }
    }
}

pub enum OpExpr {
    Op(Op, Span),
}

impl fmt::Debug for OpExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OpExpr::Op(op, _) => write!(f, "{}", op),
        }
    }
}

pub enum OpTreeExpr {
    Val(ValExpr),
    Un(Box<UnExpr<OpExpr, OpTreeExpr>>),
    Bin(Box<BinExpr<OpExpr, ValExpr, OpTreeExpr>>),
}

impl OpTreeExpr {
    pub fn span(&self) -> &Span {
        match self {
            OpTreeExpr::Val(expr) => expr.span(),
            OpTreeExpr::Un(expr) => &expr.span,
            OpTreeExpr::Bin(expr) => &expr.span,
        }
    }
}

impl fmt::Debug for OpTreeExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OpTreeExpr::Val(expr) => expr.fmt(f),
            OpTreeExpr::Un(expr) => expr.fmt(f),
            OpTreeExpr::Bin(expr) => expr.fmt(f),
        }
    }
}

pub struct UnExpr<O, X> {
    pub op: O,
    pub x: X,
    pub span: Span,
    pub parened: bool,
}

impl<O, X> fmt::Debug for UnExpr<O, X>
where
    O: fmt::Debug,
    X: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({:?} {:?})", self.op, self.x)
    }
}

pub struct BinExpr<O, W, X> {
    pub op: O,
    pub w: W,
    pub x: X,
    pub span: Span,
    pub parened: bool,
}

impl<O, W, X> fmt::Debug for BinExpr<O, W, X>
where
    O: fmt::Debug,
    W: fmt::Debug,
    X: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({:?} {:?} {:?})", self.w, self.op, self.x)
    }
}

pub struct ArrayExpr {
    pub items: Vec<ValExpr>,
    pub tied: bool,
    pub span: Span,
}

impl fmt::Debug for ArrayExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.items)
    }
}
