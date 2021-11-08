use std::fmt;

use crate::{
    ast::{Format, Formatter},
    error::RuntimeResult,
    op::*,
    value::Val,
};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct UnModded {
    pub m: RuneUnMod,
    pub f: Val,
}

impl fmt::Debug for UnModded {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({} {:?})", self.m, self.f)
    }
}

impl Format for UnModded {
    fn format(&self, f: &mut Formatter) -> RuntimeResult<()> {
        f.display(self.m);
        self.f.format(f)?;
        Ok(())
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BinModded {
    pub m: RuneBinMod,
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
        f.display(self.m);
        self.f.format(f)?;
        self.g.format(f)?;
        Ok(())
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Atop {
    pub f: Val,
    pub g: Val,
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
    pub center: Val,
    pub right: Val,
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
        self.right.format(f)?;
        Ok(())
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Function {
    Op(Op),
    UnMod(Box<UnModded>),
    BinMod(Box<BinModded>),
    Atop(Box<Atop>),
    Fork(Box<Fork>),
}

impl Function {
    pub const fn type_name(&self) -> &'static str {
        match self {
            Function::Op(_) => "operator",
            Function::UnMod(_) => "unary modifier",
            Function::BinMod(_) => "binary modifier",
            Function::Atop(atop) => atop.f.type_name(),
            Function::Fork(fork) => fork.center.type_name(),
        }
    }
}

impl From<Op> for Function {
    fn from(op: Op) -> Self {
        Function::Op(op)
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
            Function::UnMod(m) => m.format(f),
            Function::BinMod(m) => m.format(f),
            Function::Atop(atop) => atop.format(f),
            Function::Fork(fork) => fork.format(f),
        }
    }
}
