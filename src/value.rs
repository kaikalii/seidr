use std::fmt;

use crate::{
    array::Array,
    error::RuntimeResult,
    format::{Format, Formatter},
    function::*,
    lex::Span,
    num::Num,
    op::*,
    pervade::LazyPervade,
};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Atom {
    Num(Num),
    Char(char),
    Function(Function),
    UnMod(UnMod),
    BinMod(BinMod),
}

impl Atom {
    pub const fn type_name(&self) -> &'static str {
        match self {
            Atom::Num(_) => "number",
            Atom::Char(_) => "character",
            Atom::Function(f) => f.type_name(),
            Atom::UnMod(_) => "unary modifier",
            Atom::BinMod(_) => "binary modifier",
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

impl From<UnMod> for Atom {
    fn from(m: UnMod) -> Self {
        Atom::UnMod(m)
    }
}

impl From<BinMod> for Atom {
    fn from(m: BinMod) -> Self {
        Atom::BinMod(m)
    }
}

impl From<UnModded> for Atom {
    fn from(m: UnModded) -> Self {
        Function::from(m).into()
    }
}

impl From<BinModded> for Atom {
    fn from(m: BinModded) -> Self {
        Function::from(m).into()
    }
}

impl From<Function> for Atom {
    fn from(f: Function) -> Self {
        Atom::Function(f)
    }
}

impl fmt::Debug for Atom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Atom::Num(n) => n.fmt(f),
            Atom::Char(c) => c.fmt(f),
            Atom::Function(fun) => fun.fmt(f),
            Atom::UnMod(m) => m.fmt(f),
            Atom::BinMod(m) => m.fmt(f),
        }
    }
}

impl Format for Atom {
    fn format(&self, f: &mut Formatter) -> RuntimeResult<()> {
        match self {
            Atom::Num(num) => f.display(num),
            Atom::Char(c) => f.debug(c),
            Atom::Function(fun) => fun.format(f)?,
            Atom::UnMod(m) => f.display(m),
            Atom::BinMod(m) => f.display(m),
        }
        Ok(())
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Val {
    Atom(Atom),
    Array(Array),
}

fn _val_size() {
    use std::mem::transmute;
    let _: [u8; 48] = unsafe { transmute(Atom::from(1i64)) };
    let _: [u8; 40] = unsafe { transmute(Array::string("")) };
    let _: [u8; 56] = unsafe { transmute(Val::from(1i64)) };
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
    pub fn matches(&self, other: &Self) -> RuntimeResult<bool> {
        match (self, other) {
            (Val::Atom(a), Val::Atom(b)) => Ok(a == b),
            (Val::Array(a), Val::Array(b)) => a.matches(b),
            _ => Ok(false),
        }
    }
    pub fn limited_depth(&self) -> RuntimeResult<usize> {
        match self {
            Val::Atom(_) => Ok(0),
            Val::Array(arr) => arr.limited_depth(),
        }
    }
    pub fn depth(&self, span: &Span) -> RuntimeResult<usize> {
        match self {
            Val::Atom(_) => Ok(0),
            Val::Array(arr) => arr.depth(span),
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

impl From<LazyPervade> for Val {
    fn from(pa: LazyPervade) -> Self {
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

impl Format for Val {
    fn format(&self, f: &mut Formatter) -> RuntimeResult<()> {
        match self {
            Val::Atom(atom) => atom.format(f),
            Val::Array(arr) => arr.format(f),
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
