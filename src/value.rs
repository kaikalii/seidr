use std::{fmt, rc::Rc};

use crate::{
    array::Array,
    ast::{Bin, Un},
    lex::Sp,
    num::Num,
    op::Op,
};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Atom {
    Num(Num),
    Char(char),
    Op(Op),
}

impl From<bool> for Atom {
    fn from(b: bool) -> Self {
        Atom::Num(Num::Int(b as i64))
    }
}

impl From<Num> for Atom {
    fn from(num: Num) -> Self {
        Atom::Num(num)
    }
}

impl From<char> for Atom {
    fn from(c: char) -> Self {
        Atom::Char(c)
    }
}

impl From<Op> for Atom {
    fn from(op: Op) -> Self {
        Atom::Op(op)
    }
}

impl fmt::Debug for Atom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Atom::Num(num) => num.fmt(f),
            Atom::Char(c) => c.fmt(f),
            Atom::Op(op) => op.fmt(f),
        }
    }
}

impl fmt::Display for Atom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Atom::Num(num) => num.fmt(f),
            Atom::Char(c) => c.fmt(f),
            Atom::Op(op) => op.fmt(f),
        }
    }
}

#[derive(Clone)]
pub enum Val {
    Atom(Atom),
    Array(Array),
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
