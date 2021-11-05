//! Types for the Abstract Syntax Tree

use std::{
    fmt::{self, Debug, Write},
    rc::Rc,
};

use crate::{
    lex::{Comment, Sp, Span},
    num::Num,
    op::*,
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
format_display!(OpExpr);
format_display!(ModExpr);
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
    pub expr: OpExpr,
    pub comment: Option<Comment>,
}

pub enum ValExpr {
    Num(Sp<Num>),
    Char(Sp<char>),
    String(Sp<Rc<str>>),
    Array(ArrayExpr),
    Parened(Box<OpExpr>),
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

pub enum ModExpr {
    Op(Sp<Op>),
    Val(ValExpr),
    Un(Box<UnModExpr>),
    Bin(Box<BinModExpr>),
}

pub struct UnModExpr {
    pub m: RuneUnMod,
    pub span: Span,
    pub f: ModExpr,
}

pub struct BinModExpr {
    pub m: RuneBinMod,
    pub span: Span,
    pub f: ModExpr,
    pub g: ModExpr,
}

impl ModExpr {
    pub fn span(&self) -> &Span {
        match self {
            ModExpr::Op(op) => &op.span,
            ModExpr::Val(expr) => expr.span(),
            ModExpr::Un(expr) => &expr.span,
            ModExpr::Bin(expr) => &expr.span,
        }
    }
}

impl fmt::Debug for ModExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModExpr::Op(op) => op.fmt(f),
            ModExpr::Val(expr) => expr.fmt(f),
            ModExpr::Un(expr) => expr.fmt(f),
            ModExpr::Bin(expr) => expr.fmt(f),
        }
    }
}

impl fmt::Debug for UnModExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({} {})", self.m, self.f)
    }
}

impl fmt::Debug for BinModExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({} {} {})", self.m, self.f, self.g)
    }
}

pub enum OpExpr {
    Val(ValExpr),
    Un(Box<UnOpExpr>),
    Bin(Box<BinOpExpr>),
}

pub struct UnOpExpr {
    pub op: ModExpr,
    pub x: OpExpr,
}

pub struct BinOpExpr {
    pub op: ModExpr,
    pub w: ValExpr,
    pub x: OpExpr,
}

impl OpExpr {
    pub fn span(&self) -> &Span {
        match self {
            OpExpr::Val(expr) => expr.span(),
            OpExpr::Un(expr) => expr.op.span(),
            OpExpr::Bin(expr) => expr.op.span(),
        }
    }
    pub fn unparen(self) -> Self {
        match self {
            OpExpr::Val(ValExpr::Parened(expr)) => *expr,
            expr => expr,
        }
    }
}

impl fmt::Debug for OpExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OpExpr::Val(expr) => expr.fmt(f),
            OpExpr::Un(expr) => expr.fmt(f),
            OpExpr::Bin(expr) => expr.fmt(f),
        }
    }
}

impl fmt::Debug for UnOpExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({} {})", self.op, self.x)
    }
}

impl fmt::Debug for BinOpExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({} {} {})", self.w, self.op, self.x)
    }
}

pub struct ArrayExpr {
    pub items: Vec<OpExpr>,
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

impl Format for Op {
    fn format(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl Format for RuneUnMod {
    fn format(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl Format for RuneBinMod {
    fn format(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl Format for ModExpr {
    fn format(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            ModExpr::Op(op) => op.format(f),
            ModExpr::Val(expr) => expr.format(f),
            ModExpr::Un(expr) => expr.format(f),
            ModExpr::Bin(expr) => expr.format(f),
        }
    }
}

impl Format for OpExpr {
    fn format(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            OpExpr::Val(expr) => expr.format(f),
            OpExpr::Un(expr) => expr.format(f),
            OpExpr::Bin(expr) => expr.format(f),
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

impl Format for UnOpExpr {
    fn format(&self, f: &mut Formatter) -> fmt::Result {
        self.op.format(f)?;
        if matches!(&self.x, OpExpr::Bin(_)) {
            write!(f, " ")?;
        }
        self.x.format(f)
    }
}

impl Format for BinOpExpr {
    fn format(&self, f: &mut Formatter) -> fmt::Result {
        self.w.format(f)?;
        write!(f, " ")?;
        self.op.format(f)?;
        write!(f, " ")?;
        self.x.format(f)
    }
}

impl Format for UnModExpr {
    fn format(&self, f: &mut Formatter) -> fmt::Result {
        self.m.format(f)?;
        self.f.format(f)
    }
}

impl Format for BinModExpr {
    fn format(&self, f: &mut Formatter) -> fmt::Result {
        self.m.format(f)?;
        self.f.format(f)?;
        self.g.format(f)
    }
}
