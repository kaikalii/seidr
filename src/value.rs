use std::fmt;

use crate::num::Num;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Val {
    Num(Num),
    Char(char),
}

impl fmt::Debug for Val {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Val::Num(num) => num.fmt(f),
            Val::Char(c) => c.fmt(f),
        }
    }
}

impl fmt::Display for Val {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Val::Num(num) => num.fmt(f),
            Val::Char(c) => c.fmt(f),
        }
    }
}
