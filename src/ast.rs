//! Types for the Abstract Syntax Tree

use std::{fmt, rc::Rc};

use crate::{
    error::RuntimeResult,
    format::{Format, Formatter},
    lex::{Comment, Ident, Role, Sp, Span},
    num::Num,
    op::*,
};

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
format_display!(Expr);
format_display!(UnExpr);
format_display!(BinExpr);
format_display!(ArrayExpr);

pub enum Item {
    Newline,
    Comment(Comment),
    Expr(ExprItem),
}

impl fmt::Debug for Item {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Item::Newline => write!(f, "\\n"),
            Item::Comment(comment) => comment.fmt(f),
            Item::Expr(expr) => expr.expr.fmt(f),
        }
    }
}

impl Format for Item {
    fn format(&self, f: &mut Formatter) -> RuntimeResult<()> {
        match self {
            Item::Newline => {}
            Item::Comment(comment) => f.display(comment),
            Item::Expr(expr) => expr.format(f)?,
        };
        f.newline();
        Ok(())
    }
}

pub struct ExprItem {
    pub expr: Expr,
    pub comment: Option<Comment>,
}

impl Format for ExprItem {
    fn format(&self, f: &mut Formatter) -> RuntimeResult<()> {
        if let Some(comment) = &self.comment {
            f.display(comment);
            f.newline();
        }
        self.expr.format(f)
    }
}

pub enum Expr {
    Op(Sp<Op>),
    UnMod(Sp<RuneUnMod>),
    BinMod(Sp<RuneBinMod>),
    Ident(Sp<Ident>),
    Num(Sp<Num>),
    Char(Sp<char>),
    String(Sp<Rc<str>>),
    Array(ArrayExpr),
    Parened(Box<Expr>),
    Un(Box<UnExpr>),
    Bin(Box<BinExpr>),
    Assign(Box<AssignExpr>),
}

impl Expr {
    pub fn un(op: Self, inner: Self) -> Self {
        Expr::Un(UnExpr { op, inner }.into())
    }
    pub fn bin(op: Self, left: Self, right: Self) -> Self {
        Expr::Bin(BinExpr { op, left, right }.into())
    }
    pub fn role(&self) -> Role {
        use Expr::*;
        match self {
            Num(_) | Char(_) | String(_) | Array(_) => Role::Value,
            Op(_) => Role::Function,
            UnMod(_) => Role::UnModifier,
            BinMod(_) => Role::BinModifier,
            Ident(ident) => ident.role(),
            Parened(expr) => expr.role(),
            Un(expr) => expr.op.role().un(expr.inner.role()),
            Bin(expr) => expr.op.role().bin(expr.left.role(), expr.right.role()),
            Assign(expr) => expr.name.role(),
        }
    }
    pub fn span(&self) -> &Span {
        match self {
            Expr::Op(expr) => &expr.span,
            Expr::UnMod(expr) => &expr.span,
            Expr::BinMod(expr) => &expr.span,
            Expr::Ident(expr) => &expr.span,
            Expr::Num(expr) => &expr.span,
            Expr::Char(expr) => &expr.span,
            Expr::String(expr) => &expr.span,
            Expr::Array(expr) => &expr.span,
            Expr::Parened(expr) => expr.span(),
            Expr::Un(expr) => expr.op.span(),
            Expr::Bin(expr) => expr.op.span(),
            Expr::Assign(expr) => &expr.span,
        }
    }
}

impl fmt::Debug for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // write!(
        //     f,
        //     "{}:",
        //     format!("{:?}", self.role()).chars().next().unwrap()
        // )?;
        match self {
            Expr::Op(expr) => expr.fmt(f),
            Expr::UnMod(expr) => expr.fmt(f),
            Expr::BinMod(expr) => expr.fmt(f),
            Expr::Ident(expr) => expr.fmt(f),
            Expr::Num(expr) => expr.fmt(f),
            Expr::Char(expr) => expr.fmt(f),
            Expr::String(expr) => expr.fmt(f),
            Expr::Array(expr) => expr.fmt(f),
            Expr::Parened(expr) => {
                write!(f, "(")?;
                expr.fmt(f)?;
                write!(f, ")")
            }
            Expr::Un(expr) => expr.fmt(f),
            Expr::Bin(expr) => expr.fmt(f),
            Expr::Assign(expr) => expr.fmt(f),
        }
    }
}

impl Format for Expr {
    fn format(&self, f: &mut Formatter) -> RuntimeResult<()> {
        match self {
            Expr::Op(expr) => f.display(expr),
            Expr::UnMod(expr) => f.display(expr),
            Expr::BinMod(expr) => f.display(expr),
            Expr::Ident(expr) => f.display(expr),
            Expr::Num(expr) => f.display(expr),
            Expr::Char(expr) => f.debug(expr),
            Expr::String(expr) => f.debug(expr),
            Expr::Array(expr) => expr.format(f)?,
            Expr::Parened(expr) => {
                f.display('(');
                expr.format(f)?;
                f.display(')');
            }
            Expr::Un(expr) => expr.format(f)?,
            Expr::Bin(expr) => expr.format(f)?,
            Expr::Assign(expr) => expr.format(f)?,
        }
        Ok(())
    }
}

pub struct UnExpr {
    pub op: Expr,
    pub inner: Expr,
}

impl fmt::Debug for UnExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({:?} {:?})", self.op, self.inner)
    }
}

impl Format for UnExpr {
    fn format(&self, f: &mut Formatter) -> RuntimeResult<()> {
        self.op.format(f)?;
        if matches!(&self.inner, Expr::Bin(_)) {
            f.display(" ");
        }
        self.inner.format(f)
    }
}

pub struct BinExpr {
    pub op: Expr,
    pub left: Expr,
    pub right: Expr,
}

impl fmt::Debug for BinExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({:?} {:?} {:?})", self.left, self.op, self.right)
    }
}

impl Format for BinExpr {
    fn format(&self, f: &mut Formatter) -> RuntimeResult<()> {
        self.left.format(f)?;
        f.display(" ");
        self.op.format(f)?;
        f.display(" ");
        self.right.format(f)
    }
}

pub struct AtopExpr {
    pub f: Expr,
    pub g: Expr,
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
    pub left: Expr,
    pub center: Expr,
    pub right: Expr,
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

pub struct AssignExpr {
    pub name: Ident,
    pub op: AssignOp,
    pub body: Expr,
    pub span: Span,
}

impl fmt::Debug for AssignExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({} {} {:?})", self.name, self.op, self.body)
    }
}

impl Format for AssignExpr {
    fn format(&self, f: &mut Formatter) -> RuntimeResult<()> {
        f.display(&self.name);
        f.display(' ');
        f.display(self.op);
        f.display(' ');
        self.body.format(f)
    }
}

pub struct ArrayExpr {
    pub items: Vec<(Expr, bool)>,
    pub span: Span,
}

impl fmt::Debug for ArrayExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list()
            .entries(self.items.iter().map(|(item, _)| item))
            .finish()
    }
}

impl Format for ArrayExpr {
    fn format(&self, f: &mut Formatter) -> RuntimeResult<()> {
        f.display('⟨');
        for (i, (item, comma)) in self.items.iter().enumerate() {
            if i > 0 {
                f.display(' ');
            }
            item.format(f)?;
            if *comma {
                f.display(',');
            }
        }
        f.display('⟩');
        Ok(())
    }
}
