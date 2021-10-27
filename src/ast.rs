use std::fmt;

use crate::{
    lex::{Ident, Span},
    num::Num,
    op::Op,
};

pub enum Item {
    Expr(Expr),
}

impl fmt::Debug for Item {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Item::Expr(expr) => expr.fmt(f),
        }
    }
}

pub enum Expr {
    Num(Num, Span),
    Ident(Ident, Span),
    Un(Box<UnExpr>),
    Bin(Box<BinExpr>),
}

impl Expr {
    pub fn span(&self) -> &Span {
        match self {
            Expr::Num(_, span) | Expr::Ident(_, span) => span,
            Expr::Un(expr) => &expr.span,
            Expr::Bin(expr) => &expr.span,
        }
    }
}

impl fmt::Debug for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::Num(n, _) => n.fmt(f),
            Expr::Ident(ident, _) => ident.fmt(f),
            Expr::Un(expr) => expr.fmt(f),
            Expr::Bin(expr) => expr.fmt(f),
        }
    }
}

pub struct UnExpr {
    pub op: Op,
    pub inner: Expr,
    pub op_span: Span,
    pub span: Span,
}

impl fmt::Debug for UnExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({} {:?})", self.op, self.inner)
    }
}

pub struct BinExpr {
    pub op: Op,
    pub left: Expr,
    pub right: Expr,
    pub op_span: Span,
    pub span: Span,
}

impl fmt::Debug for BinExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({:?} {} {:?})", self.left, self.op, self.right)
    }
}
