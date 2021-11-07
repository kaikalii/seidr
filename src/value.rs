use std::fmt;

use crate::{array::Array, function::Function, num::Num, op::*, pervade::PervadedArray};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Atom {
    Num(Num),
    Char(char),
    Function(Function),
}

impl Atom {
    pub const fn type_name(&self) -> &'static str {
        match self {
            Atom::Num(_) => "number",
            Atom::Char(_) => "character",
            Atom::Function(f) => f.type_name(),
        }
    }
}

impl From<bool> for Atom {
    fn from(b: bool) -> Self {
        (b as i64).into()
    }
}

impl<N> From<N> for Atom
where
    N: Into<Num>,
{
    fn from(num: N) -> Self {
        Atom::Num(num.into())
    }
}

impl From<char> for Atom {
    fn from(c: char) -> Self {
        Atom::Char(c)
    }
}

impl From<Op> for Atom {
    fn from(op: Op) -> Self {
        Function::Op(op).into()
    }
}

impl From<RuneUnMod> for Atom {
    fn from(m: RuneUnMod) -> Self {
        Function::UnMod(m).into()
    }
}

impl From<RuneBinMod> for Atom {
    fn from(m: RuneBinMod) -> Self {
        Function::BinMod(m).into()
    }
}

impl From<Function> for Atom {
    fn from(f: Function) -> Self {
        Atom::Function(f)
    }
}

impl fmt::Debug for Atom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl fmt::Display for Atom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Atom::Num(num) => num.fmt(f),
            Atom::Char(c) => write!(f, "{:?}", c),
            Atom::Function(fun) => fun.fmt(f),
        }
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Val {
    Atom(Atom),
    Array(Array),
}

impl Val {
    pub const fn type_name(&self) -> &'static str {
        match self {
            Val::Array(_) => "array",
            Val::Atom(atom) => atom.type_name(),
        }
    }
    pub fn into_array(self) -> Array {
        match self {
            Val::Array(arr) => arr,
            Val::Atom(_) => Array::concrete(Some(self)),
        }
    }
}

impl<A> From<A> for Val
where
    A: Into<Atom>,
{
    fn from(atom: A) -> Self {
        Val::Atom(atom.into())
    }
}

impl From<Array> for Val {
    fn from(arr: Array) -> Self {
        Val::Array(arr)
    }
}

impl From<PervadedArray> for Val {
    fn from(pa: PervadedArray) -> Self {
        Val::Array(pa.into())
    }
}

impl fmt::Debug for Val {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Val::Atom(atom) => atom.fmt(f),
            Val::Array(arr) => arr.fmt(f),
        }
    }
}

impl fmt::Display for Val {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Val::Atom(atom) => atom.fmt(f),
            Val::Array(arr) => arr.fmt(f),
        }
    }
}

impl<V> FromIterator<V> for Val
where
    V: Into<Val>,
{
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = V>,
    {
        Val::Array(Array::from_iter(iter))
    }
}
