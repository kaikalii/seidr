use std::{fmt, rc::Rc};

use crate::{
    array::Array,
    ast::{Bin, Un},
    lex::Sp,
    num::Num,
    op::Op,
};

#[derive(Clone)]
pub enum Val {
    Num(Num),
    Char(char),
    Array(Array),
    Op(Op),
}

impl From<Num> for Val {
    fn from(num: Num) -> Self {
        Val::Num(num)
    }
}

impl From<char> for Val {
    fn from(c: char) -> Self {
        Val::Char(c)
    }
}

impl From<Array> for Val {
    fn from(arr: Array) -> Self {
        Val::Array(arr)
    }
}

impl From<Op> for Val {
    fn from(op: Op) -> Self {
        Val::Op(op)
    }
}

impl fmt::Debug for Val {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Val::Num(num) => num.fmt(f),
            Val::Char(c) => c.fmt(f),
            Val::Array(arr) => arr.fmt(f),
            Val::Op(op) => op.fmt(f),
        }
    }
}

impl fmt::Display for Val {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Val::Num(num) => num.fmt(f),
            Val::Char(c) => c.fmt(f),
            Val::Array(arr) => arr.fmt(f),
            Val::Op(op) => op.fmt(f),
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
