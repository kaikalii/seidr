//! Types for the Abstract Syntax Tree

use std::{
    fmt::{self, Write},
    rc::Rc,
};

use crate::{
    lex::{Comment, Sp, Span},
    num::Num,
    op::Op,
};

macro_rules! format_display {
    ($ty:ty) => {
        impl fmt::Display for $ty {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "{}", self.as_string())
            }
        }
    };
}

format_display!(Item);
format_display!(ExprItem);
format_display!(OpTreeExpr);
format_display!(OpExpr);
format_display!(ValExpr);
format_display!(UnOpExpr);
format_display!(BinOpExpr);
format_display!(ArrayExpr);

#[derive(Debug)]
pub enum Item {
    Newline,
    Comment(Comment),
    Expr(ExprItem),
}

#[derive(Debug)]
pub struct ExprItem {
    pub expr: OpTreeExpr,
    pub comment: Option<Comment>,
}

pub enum ValExpr {
    Num(Sp<Num>),
    Char(Sp<char>),
    String(Sp<Rc<str>>),
    Array(ArrayExpr),
    Parened(Box<OpTreeExpr>),
}

impl ValExpr {
    pub fn span(&self) -> &Span {
        match self {
            ValExpr::Char(c) => &c.span,
            ValExpr::Num(num) => &num.span,
            ValExpr::String(string) => &string.span,
            ValExpr::Array(expr) => &expr.span,
            ValExpr::Parened(expr) => expr.span(),
        }
    }
}

impl fmt::Debug for ValExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValExpr::Num(n) => n.fmt(f),
            ValExpr::Char(c) => c.fmt(f),
            ValExpr::String(string) => string.fmt(f),
            ValExpr::Array(expr) => expr.fmt(f),
            ValExpr::Parened(expr) => expr.fmt(f),
        }
    }
}

pub enum OpExpr {
    Op(Sp<Op>),
}

impl OpExpr {
    pub fn span(&self) -> &Span {
        match self {
            OpExpr::Op(op) => &op.span,
        }
    }
}

impl fmt::Debug for OpExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OpExpr::Op(op) => write!(f, "{}", op),
        }
    }
}

pub enum OpTreeExpr {
    Val(ValExpr),
    Un(Box<UnOpExpr>),
    Bin(Box<BinOpExpr>),
}

pub type UnOpExpr = Un<OpExpr, OpTreeExpr>;
pub type BinOpExpr = Bin<OpExpr, ValExpr, OpTreeExpr>;

impl OpTreeExpr {
    pub fn span(&self) -> &Span {
        match self {
            OpTreeExpr::Val(expr) => expr.span(),
            OpTreeExpr::Un(expr) => expr.op.span(),
            OpTreeExpr::Bin(expr) => expr.op.span(),
        }
    }
    pub fn unparen(self) -> Self {
        match self {
            OpTreeExpr::Val(ValExpr::Parened(expr)) => *expr,
            expr => expr,
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

#[derive(Clone)]
pub struct Un<O, X> {
    pub op: O,
    pub x: X,
}

impl<O, X> fmt::Debug for Un<O, X>
where
    O: fmt::Debug,
    X: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({:?} {:?})", self.op, self.x)
    }
}

#[derive(Clone)]
pub struct Bin<O, W, X> {
    pub op: O,
    pub w: W,
    pub x: X,
}

impl<O, W, X> fmt::Debug for Bin<O, W, X>
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
    pub items: Vec<OpTreeExpr>,
    pub tied: bool,
    pub span: Span,
}

impl fmt::Debug for ArrayExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.items)
    }
}

pub struct Formatter<'w> {
    depth: usize,
    writer: &'w mut dyn Write,
}

impl<'w> Formatter<'w> {
    pub fn new<W: Write>(writer: &'w mut W) -> Self {
        Formatter { depth: 0, writer }
    }
}

impl<'w> Write for Formatter<'w> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.writer.write_str(s)
    }
}

pub trait Format {
    fn format(&self, f: &mut Formatter) -> fmt::Result;
    fn as_string(&self) -> String {
        let mut string = String::new();
        let mut formatter = Formatter::new(&mut string);
        let _ = self.format(&mut formatter);
        string
    }
}

impl Format for Item {
    fn format(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Item::Newline => {}
            Item::Expr(expr) => expr.format(f)?,
            Item::Comment(comment) => write!(f, "{}", comment)?,
        };
        writeln!(f)
    }
}

impl Format for ExprItem {
    fn format(&self, f: &mut Formatter) -> fmt::Result {
        if let Some(comment) = &self.comment {
            writeln!(f, "{}", comment)?;
        }
        self.expr.format(f)
    }
}

impl Format for ValExpr {
    fn format(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            ValExpr::Num(n) => {
                let s = n.span.as_string();
                if s.contains('e') || s.contains('E') {
                    write!(f, "{}", s)
                } else {
                    let n = **n;
                    if n < Num::Int(0) {
                        write!(f, "‾")?;
                    }
                    let n = n.abs().to_string();
                    let mut parts = n.split('.');
                    let left = parts.next().unwrap();
                    let right = parts.next();
                    for (i, c) in left.chars().enumerate() {
                        let i = left.len() - i - 1;
                        write!(f, "{}", c)?;
                        if i > 0 && i % 3 == 0 {
                            write!(f, "_")?;
                        }
                    }
                    if let Some(right) = right {
                        write!(f, ".")?;
                        for (i, c) in right.chars().enumerate() {
                            write!(f, "{}", c)?;
                            if i > 0 && i % 3 == 2 {
                                write!(f, "_")?;
                            }
                        }
                    }
                    Ok(())
                }
            }
            ValExpr::Char(c) => write!(f, "{:?}", c),
            ValExpr::String(string) => write!(f, "{:?}", string),
            ValExpr::Array(expr) => expr.format(f),
            ValExpr::Parened(expr) => {
                write!(f, "(")?;
                expr.format(f)?;
                write!(f, ")")
            }
        }
    }
}

impl Format for OpExpr {
    fn format(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            OpExpr::Op(op) => write!(f, "{}", op),
        }
    }
}

impl Format for OpTreeExpr {
    fn format(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            OpTreeExpr::Val(expr) => expr.format(f),
            OpTreeExpr::Un(expr) => expr.format(f),
            OpTreeExpr::Bin(expr) => expr.format(f),
        }
    }
}

impl Format for ArrayExpr {
    fn format(&self, f: &mut Formatter) -> fmt::Result {
        if !self.tied {
            write!(f, "⟨")?;
        }
        for (i, item) in self.items.iter().enumerate() {
            if i > 0 {
                if self.tied {
                    write!(f, "‿")?;
                } else {
                    write!(f, ", ")?;
                }
            }
            item.format(f)?;
        }
        if !self.tied {
            write!(f, "⟩")?;
        }
        Ok(())
    }
}

impl<O, W, X> Format for Bin<O, W, X>
where
    O: Format,
    W: Format,
    X: Format,
{
    fn format(&self, f: &mut Formatter) -> fmt::Result {
        self.w.format(f)?;
        write!(f, " ")?;
        self.op.format(f)?;
        write!(f, " ")?;
        self.x.format(f)?;
        Ok(())
    }
}

impl<O, X> Format for Un<O, X>
where
    O: Format,
    X: Format,
{
    fn format(&self, f: &mut Formatter) -> fmt::Result {
        self.op.format(f)?;
        write!(f, " ")?;
        self.x.format(f)?;
        Ok(())
    }
}
