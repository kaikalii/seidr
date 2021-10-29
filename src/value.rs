use std::fmt;

use crate::{array::Array, num::Num};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Val {
    Num(Num),
    Char(char),
    Array(Array),
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

impl fmt::Debug for Val {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Val::Num(num) => num.fmt(f),
            Val::Char(c) => c.fmt(f),
            Val::Array(arr) => arr.fmt(f),
        }
    }
}

impl fmt::Display for Val {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Val::Num(num) => num.fmt(f),
            Val::Char(c) => c.fmt(f),
            Val::Array(arr) => arr.fmt(f),
        }
    }
}
