use std::{fmt, rc::Rc};

use crate::{
    array::Array,
    ast::{Bin, Un},
    lex::Sp,
    num::Num,
};

#[derive(Clone)]
pub enum Val {
    Num(Num),
    Char(char),
    Array(Array),
    Un(Rc<UnVal>),
    Bin(Rc<BinVal>),
}

pub type UnVal = Un<Sp<Val>, Val>;
pub type BinVal = Bin<Sp<Val>, Val, Val>;

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

impl From<UnVal> for Val {
    fn from(un: UnVal) -> Self {
        Val::Un(un.into())
    }
}

impl From<BinVal> for Val {
    fn from(bin: BinVal) -> Self {
        Val::Bin(bin.into())
    }
}

impl fmt::Debug for Val {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Val::Num(num) => num.fmt(f),
            Val::Char(c) => c.fmt(f),
            Val::Array(arr) => arr.fmt(f),
            Val::Un(expr) => expr.fmt(f),
            Val::Bin(expr) => expr.fmt(f),
        }
    }
}

impl fmt::Display for Val {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Val::Num(num) => num.fmt(f),
            Val::Char(c) => c.fmt(f),
            Val::Array(arr) => arr.fmt(f),
            Val::Un(expr) => write!(f, "{} {}", expr.op.0, expr.x),
            Val::Bin(expr) => write!(f, "{} {} {}", expr.w, expr.op.0, expr.x),
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
