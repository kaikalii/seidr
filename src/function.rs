use std::fmt;

use crate::{op::*, value::Val};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Atop {
    pub f: Val,
    pub g: Val,
}

impl fmt::Debug for Atop {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl fmt::Display for Atop {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.f, self.g)
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
        write!(f, "{}", self)
    }
}

impl fmt::Display for Fork {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}{}", self.left, self.center, self.right)
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Function {
    Op(Op),
    UnMod(RuneUnMod),
    BinMod(RuneBinMod),
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

impl From<RuneUnMod> for Function {
    fn from(m: RuneUnMod) -> Self {
        Function::UnMod(m)
    }
}

impl From<RuneBinMod> for Function {
    fn from(m: RuneBinMod) -> Self {
        Function::BinMod(m)
    }
}

impl fmt::Debug for Function {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl fmt::Display for Function {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Function::Op(op) => op.fmt(f),
            Function::UnMod(m) => m.fmt(f),
            Function::BinMod(m) => m.fmt(f),
            Function::Atop(atop) => atop.fmt(f),
            Function::Fork(fork) => fork.fmt(f),
        }
    }
}
