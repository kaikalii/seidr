//! Types for the Abstract Syntax Tree

use std::{fmt, rc::Rc};

use crate::{
    error::RuntimeResult,
    lex::{digit_or_inf, ident_body_char, ident_head_char, Comment, Sp, Span},
    num::Num,
    op::*,
};

pub struct Formatter<'w> {
    depth: usize,
    writer: &'w mut dyn fmt::Write,
    prev_alphanum: bool,
}

impl<'w> Formatter<'w> {
    pub fn new<W: fmt::Write>(writer: &'w mut W) -> Self {
        Formatter {
            depth: 0,
            writer,
            prev_alphanum: false,
        }
    }
    fn write_str(&mut self, s: &str) {
        if self.prev_alphanum && s.starts_with(|c| ident_head_char(c) || digit_or_inf(c)) {
            write!(self.writer, " ").unwrap_or_else(|e| panic!("{}", e));
        }

        self.prev_alphanum = s.ends_with(|c| ident_body_char(c) || digit_or_inf(c));

        write!(self.writer, "{}", s).unwrap_or_else(|e| panic!("{}", e));
    }
    pub fn display<T>(&mut self, val: T)
    where
        T: fmt::Display,
    {
        self.write_str(&val.to_string());
    }
    pub fn debug<T>(&mut self, val: T)
    where
        T: fmt::Debug,
    {
        self.write_str(&format!("{:?}", val));
    }
    pub fn newline(&mut self) {
        self.display('\n')
    }
}

pub trait Format {
    fn format(&self, f: &mut Formatter) -> RuntimeResult<()>;
    fn as_string(&self) -> RuntimeResult<String> {
        let mut string = String::new();
        let mut formatter = Formatter::new(&mut string);
        self.format(&mut formatter)?;
        Ok(string)
    }
}

macro_rules! format_display {
    ($ty:ty) => {
        impl fmt::Display for $ty {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "{}", self.as_string()?)
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
format_display!(TrainExpr);

#[derive(Debug)]
pub enum Item {
    Newline,
    Comment(Comment),
    Expr(ExprItem),
}

impl Format for Item {
    fn format(&self, f: &mut Formatter) -> RuntimeResult<()> {
        match self {
            Item::Newline => {}
            Item::Expr(expr) => expr.format(f)?,
            Item::Comment(comment) => f.display(comment),
        };
        f.newline();
        Ok(())
    }
}

#[derive(Debug)]
pub struct ExprItem {
    pub expr: OpExpr,
    pub comment: Option<Comment>,
}

impl Format for ExprItem {
    fn format(&self, f: &mut Formatter) -> RuntimeResult<()> {
        if let Some(comment) = &self.comment {
            f.display(comment);
        }
        self.expr.format(f)
    }
}

pub enum OpExpr {
    Val(ValExpr),
    Un(Box<UnOpExpr>),
    Bin(Box<BinOpExpr>),
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

impl Format for OpExpr {
    fn format(&self, f: &mut Formatter) -> RuntimeResult<()> {
        match self {
            OpExpr::Val(expr) => expr.format(f),
            OpExpr::Un(expr) => expr.format(f),
            OpExpr::Bin(expr) => expr.format(f),
        }
    }
}

pub enum ValExpr {
    Num(Sp<Num>),
    Char(Sp<char>),
    String(Sp<Rc<str>>),
    Array(ArrayExpr),
    Parened(Box<OpExpr>),
    Mod(ModExpr),
}

impl ValExpr {
    pub fn span(&self) -> &Span {
        match self {
            ValExpr::Char(c) => &c.span,
            ValExpr::Num(num) => &num.span,
            ValExpr::String(string) => &string.span,
            ValExpr::Array(expr) => &expr.span,
            ValExpr::Parened(expr) => expr.span(),
            ValExpr::Mod(expr) => expr.span(),
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
            ValExpr::Mod(expr) => expr.fmt(f),
        }
    }
}

impl Format for ValExpr {
    fn format(&self, f: &mut Formatter) -> RuntimeResult<()> {
        match self {
            ValExpr::Num(n) => f.display(n.string_format(&n.span.as_string())),
            ValExpr::Char(c) => f.debug(c),
            ValExpr::String(string) => f.debug(string),
            ValExpr::Array(expr) => expr.format(f)?,
            ValExpr::Parened(expr) => {
                f.display('(');
                expr.format(f)?;
                f.display(')');
            }
            ValExpr::Mod(expr) => expr.format(f)?,
        }
        Ok(())
    }
}

pub enum ModExpr {
    Op(Sp<Op>),
    Un(Box<UnModExpr>),
    Bin(Box<BinModExpr>),
    Parened(Box<TrainExpr>),
}

impl ModExpr {
    pub fn span(&self) -> &Span {
        match self {
            ModExpr::Op(op) => &op.span,
            ModExpr::Un(expr) => &expr.span,
            ModExpr::Bin(expr) => &expr.span,
            ModExpr::Parened(expr) => expr.span(),
        }
    }
}

impl fmt::Debug for ModExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModExpr::Op(op) => op.fmt(f),
            ModExpr::Un(expr) => expr.fmt(f),
            ModExpr::Bin(expr) => expr.fmt(f),
            ModExpr::Parened(expr) => expr.fmt(f),
        }
    }
}

impl Format for ModExpr {
    fn format(&self, f: &mut Formatter) -> RuntimeResult<()> {
        match self {
            ModExpr::Op(op) => f.display(op),
            ModExpr::Un(expr) => expr.format(f)?,
            ModExpr::Bin(expr) => expr.format(f)?,
            ModExpr::Parened(expr) => {
                f.display('(');
                expr.format(f)?;
                f.display(')');
            }
        }
        Ok(())
    }
}

pub struct UnOpExpr {
    pub op: ModExpr,
    pub x: OpExpr,
}

impl fmt::Debug for UnOpExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(un-op {:?} {:?})", self.op, self.x)
    }
}

impl Format for UnOpExpr {
    fn format(&self, f: &mut Formatter) -> RuntimeResult<()> {
        self.op.format(f)?;
        if matches!(&self.x, OpExpr::Bin(_)) {
            f.display(" ");
        }
        self.x.format(f)
    }
}

pub struct BinOpExpr {
    pub op: ModExpr,
    pub w: ValExpr,
    pub x: OpExpr,
}

impl fmt::Debug for BinOpExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(bin-op {:?} {:?} {:?})", self.w, self.op, self.x)
    }
}

impl Format for BinOpExpr {
    fn format(&self, f: &mut Formatter) -> RuntimeResult<()> {
        self.w.format(f)?;
        f.display(" ");
        self.op.format(f)?;
        f.display(" ");
        self.x.format(f)
    }
}

pub enum TrainExpr {
    Single(ModExpr),
    Atop(Box<AtopExpr>),
    Fork(Box<ForkExpr>),
}

impl TrainExpr {
    pub fn span(&self) -> &Span {
        match self {
            TrainExpr::Single(expr) => expr.span(),
            TrainExpr::Atop(expr) => expr.f.span(),
            TrainExpr::Fork(expr) => expr.center.span(),
        }
    }
}

impl fmt::Debug for TrainExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TrainExpr::Single(expr) => write!(f, "(single {:?})", expr),
            TrainExpr::Atop(expr) => expr.fmt(f),
            TrainExpr::Fork(expr) => expr.fmt(f),
        }
    }
}

impl Format for TrainExpr {
    fn format(&self, f: &mut Formatter) -> RuntimeResult<()> {
        match self {
            TrainExpr::Single(expr) => expr.format(f),
            TrainExpr::Atop(expr) => expr.format(f),
            TrainExpr::Fork(expr) => expr.format(f),
        }
    }
}

pub struct AtopExpr {
    pub f: ModExpr,
    pub g: TrainExpr,
}

impl fmt::Debug for AtopExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(atop {:?} {:?})", self.f, self.g)
    }
}

impl Format for AtopExpr {
    fn format(&self, f: &mut Formatter) -> RuntimeResult<()> {
        self.f.format(f)?;
        self.g.format(f)
    }
}

pub struct ForkExpr {
    pub left: ValExpr,
    pub center: ModExpr,
    pub right: TrainExpr,
}

impl fmt::Debug for ForkExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "(fork {:?} {:?} {:?})",
            self.left, self.center, self.right
        )
    }
}

impl Format for ForkExpr {
    fn format(&self, f: &mut Formatter) -> RuntimeResult<()> {
        self.left.format(f)?;
        self.center.format(f)?;
        self.right.format(f)
    }
}

pub struct UnModExpr {
    pub m: RuneUnMod,
    pub f: ValExpr,
    pub span: Span,
}

impl fmt::Debug for UnModExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(un-mod {:?} {:?})", self.m, self.f)
    }
}

impl Format for UnModExpr {
    fn format(&self, f: &mut Formatter) -> RuntimeResult<()> {
        f.display(&self.m);
        self.f.format(f)
    }
}

pub struct BinModExpr {
    pub m: RuneBinMod,
    pub f: ValExpr,
    pub g: ValExpr,
    pub span: Span,
}

impl fmt::Debug for BinModExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(bin-mod {:?} {:?} {:?})", self.m, self.f, self.g)
    }
}

impl Format for BinModExpr {
    fn format(&self, f: &mut Formatter) -> RuntimeResult<()> {
        f.display(&self.m);
        self.f.format(f)?;
        self.g.format(f)
    }
}

pub struct ArrayExpr {
    pub items: Vec<ArrayItemExpr>,
    pub tied: bool,
    pub span: Span,
}

impl fmt::Debug for ArrayExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.items)
    }
}

impl Format for ArrayExpr {
    fn format(&self, f: &mut Formatter) -> RuntimeResult<()> {
        if !self.tied {
            f.display('⟨');
        }
        for (i, item) in self.items.iter().enumerate() {
            if i > 0 {
                if self.tied {
                    f.display('‿');
                } else {
                    f.display(", ");
                }
            }
            item.format(f)?;
        }
        if !self.tied {
            f.display('⟩');
        }
        Ok(())
    }
}

pub enum ArrayItemExpr {
    Val(OpExpr),
    Function(TrainExpr),
}

impl ArrayItemExpr {
    pub fn span(&self) -> &Span {
        match self {
            ArrayItemExpr::Val(expr) => expr.span(),
            ArrayItemExpr::Function(expr) => expr.span(),
        }
    }
}

impl fmt::Debug for ArrayItemExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArrayItemExpr::Val(expr) => expr.fmt(f),
            ArrayItemExpr::Function(expr) => expr.fmt(f),
        }
    }
}

impl Format for ArrayItemExpr {
    fn format(&self, f: &mut Formatter) -> RuntimeResult<()> {
        match self {
            ArrayItemExpr::Val(expr) => expr.format(f),
            ArrayItemExpr::Function(expr) => expr.format(f),
        }
    }
}
