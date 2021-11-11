use std::{fmt, rc::Rc};

use crate::{
    cwt::ValNode,
    error::RuntimeResult,
    format::{Format, Formatter},
    op::*,
    value::Val,
};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Modifier<R> {
    Rune(R),
    Node(Rc<ValNode>),
}

impl<R> fmt::Debug for Modifier<R>
where
    Self: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl<R> From<ValNode> for Modifier<R> {
    fn from(node: ValNode) -> Self {
        Modifier::Node(node.into())
    }
}

pub type UnMod = Modifier<RuneUnMod>;
pub type BinMod = Modifier<RuneBinMod>;

impl From<RuneUnMod> for UnMod {
    fn from(rune: RuneUnMod) -> Self {
        Modifier::Rune(rune)
    }
}

impl From<RuneBinMod> for BinMod {
    fn from(rune: RuneBinMod) -> Self {
        Modifier::Rune(rune)
    }
}

impl fmt::Display for UnMod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Modifier::Rune(rune) => rune.fmt(f),
            Modifier::Node(_) => "<unary modifier>".fmt(f),
        }
    }
}

impl fmt::Display for BinMod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Modifier::Rune(rune) => rune.fmt(f),
            Modifier::Node(_) => "<binary modifier>".fmt(f),
        }
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct UnModded {
    pub m: UnMod,
    pub f: Val,
}

impl fmt::Debug for UnModded {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({} {:?})", self.m, self.f)
    }
}

impl Format for UnModded {
    fn format(&self, f: &mut Formatter) -> RuntimeResult<()> {
        f.display(&self.m);
        self.f.format(f)?;
        Ok(())
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BinModded {
    pub m: BinMod,
    pub f: Val,
    pub g: Val,
}

impl fmt::Debug for BinModded {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({} {:?} {:?})", self.m, self.f, self.g)
    }
}

impl Format for BinModded {
    fn format(&self, f: &mut Formatter) -> RuntimeResult<()> {
        f.display(&self.m);
        self.f.format(f)?;
        self.g.format(f)?;
        Ok(())
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Atop {
    pub f: Function,
    pub g: Function,
}

impl fmt::Debug for Atop {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({:?} {:?})", self.f, self.g)
    }
}

impl Format for Atop {
    fn format(&self, f: &mut Formatter) -> RuntimeResult<()> {
        self.f.format(f)?;
        self.g.format(f)?;
        Ok(())
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Fork {
    pub left: Val,
    pub center: Function,
    pub right: Function,
}

impl fmt::Debug for Fork {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({:?} {:?} {:?})", self.left, self.center, self.right)
    }
}

impl Format for Fork {
    fn format(&self, f: &mut Formatter) -> RuntimeResult<()> {
        self.left.format(f)?;
        self.center.format(f)?;
        let paren = !matches!(self.right, Function::Op(_));
        if paren {
            f.display('(');
        }
        self.right.format(f)?;
        if paren {
            f.display(')');
        }
        Ok(())
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Function {
    Op(Op),
    Node(Rc<ValNode>),
    UnMod(Box<UnModded>),
    BinMod(Box<BinModded>),
    Atop(Box<Atop>),
    Fork(Box<Fork>),
}

impl Function {
    pub const fn type_name(&self) -> &'static str {
        match self {
            Function::Op(_) => "function",
            Function::Node(_) => "function",
            Function::UnMod(_) => "unary modifier",
            Function::BinMod(_) => "binary modifier",
            Function::Atop(atop) => atop.f.type_name(),
            Function::Fork(fork) => fork.center.type_name(),
        }
    }
}

impl<O> From<O> for Function
where
    O: Into<Op>,
{
    fn from(op: O) -> Self {
        Function::Op(op.into())
    }
}

impl From<UnModded> for Function {
    fn from(m: UnModded) -> Self {
        Function::UnMod(m.into())
    }
}

impl From<BinModded> for Function {
    fn from(m: BinModded) -> Self {
        Function::BinMod(m.into())
    }
}

impl fmt::Debug for Function {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Function::Op(op) => op.fmt(f),
            Function::Node(node) => node.fmt(f),
            Function::UnMod(un) => un.fmt(f),
            Function::BinMod(bin) => bin.fmt(f),
            Function::Atop(atop) => atop.f.fmt(f),
            Function::Fork(fork) => fork.center.fmt(f),
        }
    }
}

impl Format for Function {
    fn format(&self, f: &mut Formatter) -> RuntimeResult<()> {
        match self {
            Function::Op(op) => {
                f.display(op);
                Ok(())
            }
            Function::Node(node) => {
                f.display("<function>");
                Ok(())
            }
            Function::UnMod(m) => m.format(f),
            Function::BinMod(m) => m.format(f),
            Function::Atop(atop) => atop.format(f),
            Function::Fork(fork) => fork.format(f),
        }
    }
}
