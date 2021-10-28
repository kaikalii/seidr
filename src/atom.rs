use std::fmt;

use crate::num::Num;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Atom {
    Num(Num),
    Char(char),
}

impl fmt::Debug for Atom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Atom::Num(num) => num.fmt(f),
            Atom::Char(c) => c.fmt(f),
        }
    }
}

impl fmt::Display for Atom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Atom::Num(num) => num.fmt(f),
            Atom::Char(c) => c.fmt(f),
        }
    }
}
