//! Types for the Abstract Syntax Tree

use std::{fmt, rc::Rc};

use crate::{
    error::RuntimeResult,
    format::{Format, Formatter},
    lex::*,
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
    Param(Sp<Param>),
    Ident(Sp<Ident>),
    Num(Sp<Num>),
    Char(Sp<char>),
    String(Sp<Rc<str>>),
    Array(ArrayExpr),
    Parened(Box<Expr>),
    Un(Box<UnExpr>),
    Bin(Box<BinExpr>),
    Assign(Box<AssignExpr>),
    Function(Box<FunctionLiteral>),
}

impl Expr {
    pub fn un(op: Self, inner: Self) -> Self {
        Expr::Un(UnExpr { op, inner }.into())
    }
    pub fn bin(op: Self, left: Self, right: Self, kind: BinKind) -> Self {
        Expr::Bin(
            BinExpr {
                op,
                left,
                right,
                kind,
            }
            .into(),
        )
    }
    pub fn role(&self) -> Role {
        use Expr::*;
        match self {
            Param(param) => param.role(),
            Num(_) | Char(_) | String(_) | Array(_) => Role::Value,
            Op(_) => Role::Function,
            UnMod(_) => Role::UnModifier,
            BinMod(_) => Role::BinModifier,
            Ident(ident) => ident.role(),
            Parened(expr) => expr.role(),
            Un(expr) => expr.op.role().un(expr.inner.role()),
            Bin(expr) => expr.op.role().bin(expr.left.role(), expr.right.role()),
            Assign(expr) => expr.name.role(),
            Function(items) => items.expressions().fold(Role::Function, |max, expr| {
                let expr_role = expr
                    .max_param()
                    .map(|param| param.place.min_role())
                    .unwrap_or(Role::Function);
                max.max(expr_role)
            }),
        }
    }
    pub fn max_param(&self) -> Option<&Sp<Param>> {
        use Expr::*;
        match self {
            Param(param) => Some(param),
            Array(expr) => expr
                .items
                .iter()
                .fold(None, |acc, (expr, _)| expr.max_param().max(acc)),
            Parened(expr) => expr.max_param(),
            Un(expr) => expr.op.max_param().max(expr.inner.max_param()),
            Bin(expr) => expr
                .op
                .max_param()
                .max(expr.left.max_param())
                .max(expr.right.max_param()),
            Assign(expr) => expr.body.max_param(),
            _ => None,
        }
    }
    pub fn span(&self) -> &Span {
        match self {
            Expr::Op(expr) => &expr.span,
            Expr::UnMod(expr) => &expr.span,
            Expr::BinMod(expr) => &expr.span,
            Expr::Param(expr) => &expr.span,
            Expr::Ident(expr) => &expr.span,
            Expr::Num(expr) => &expr.span,
            Expr::Char(expr) => &expr.span,
            Expr::String(expr) => &expr.span,
            Expr::Array(expr) => &expr.span,
            Expr::Parened(expr) => expr.span(),
            Expr::Un(expr) => expr.op.span(),
            Expr::Bin(expr) => expr.op.span(),
            Expr::Assign(expr) => &expr.span,
            Expr::Function(body) => &body.span,
        }
    }
}

impl fmt::Debug for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::Op(expr) => expr.fmt(f),
            Expr::UnMod(expr) => expr.fmt(f),
            Expr::BinMod(expr) => expr.fmt(f),
            Expr::Ident(expr) => expr.fmt(f),
            Expr::Param(expr) => expr.fmt(f),
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
            Expr::Function(expr) => expr.fmt(f),
        }
    }
}

impl Format for Expr {
    fn format(&self, f: &mut Formatter) -> RuntimeResult<()> {
        match self {
            Expr::Op(expr) => f.display(expr),
            Expr::UnMod(expr) => f.display(expr),
            Expr::BinMod(expr) => f.display(expr),
            Expr::Param(expr) => f.display(expr),
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
            Expr::Function(func) => func.format(f)?,
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
        if let Expr::Bin(bin) = &self.inner {
            if bin.kind == BinKind::Function {
                f.display(" ");
            }
        }
        self.inner.format(f)
    }
}

pub struct BinExpr {
    pub op: Expr,
    pub left: Expr,
    pub right: Expr,
    pub kind: BinKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinKind {
    Function,
    Modifier,
    Fork,
}

impl fmt::Debug for BinExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            BinKind::Function => write!(f, "({:?} {:?} {:?})", self.left, self.op, self.right),
            BinKind::Fork => write!(f, "({:?}{:?}{:?})", self.left, self.op, self.right),
            BinKind::Modifier => write!(f, "({:?}{:?}{:?})", self.op, self.left, self.right),
        }
    }
}

impl Format for BinExpr {
    fn format(&self, f: &mut Formatter) -> RuntimeResult<()> {
        match self.kind {
            BinKind::Function => {
                self.left.format(f)?;
                f.display(" ");
                self.op.format(f)?;
                f.display(" ");
            }
            BinKind::Fork => {
                self.left.format(f)?;
                self.op.format(f)?;
            }
            BinKind::Modifier => {
                self.op.format(f)?;
                self.left.format(f)?;
            }
        }
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

pub struct FunctionLiteral {
    pub items: Vec<Item>,
    pub span: Span,
}

impl FunctionLiteral {
    pub fn expressions(&self) -> impl Iterator<Item = &Expr> {
        self.items.iter().filter_map(|item| {
            if let Item::Expr(expr) = item {
                Some(&expr.expr)
            } else {
                None
            }
        })
    }
    pub fn max_param(&self) -> Option<Param> {
        self.expressions()
            .map(|expr| expr.max_param())
            .max()
            .flatten()
            .map(|param| param.data)
    }
}

impl fmt::Debug for FunctionLiteral {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "⦑")?;
        for item in &self.items {
            item.fmt(f)?;
            write!(f, ", ")?;
        }
        write!(f, "⦒")
    }
}

impl Format for FunctionLiteral {
    fn format(&self, f: &mut Formatter) -> RuntimeResult<()> {
        f.display('⦑');
        if self.items.len() == 1 {
            f.display(' ');
        } else {
            f.indent(2);
        }
        for item in &self.items {
            item.format(f)?;
        }
        if self.items.len() == 1 {
            f.display(' ');
        } else {
            f.deindent(2);
        }
        f.display('⦒');
        Ok(())
    }
}
