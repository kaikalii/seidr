use std::{convert::Infallible, fmt, ops::*, rc::Rc};

use crate::{
    error::{CompileError, CompileResult},
    eval::EvalResult,
    lex::Span,
    num::Num,
    op::Op,
    types::{ArrayType, AtomType, Ty},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Atom {
    Num(Num),
    Char(char),
    Op(Op),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Val {
    Atom(Atom),
    Array(Array),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Array {
    String(Rc<str>),
    List(Vec<Val>),
}

impl Array {
    pub fn is_empty(&self) -> bool {
        match self {
            Array::String(s) => s.is_empty(),
            Array::List(list) => list.is_empty(),
        }
    }
    pub fn len(&self) -> usize {
        match self {
            Array::String(s) => s.chars().count(),
            Array::List(list) => list.len(),
        }
    }
    pub fn iter(&self) -> Box<dyn Iterator<Item = Val> + '_> {
        match self {
            Array::String(s) => Box::new(s.chars().map(Atom::Char).map(Val::Atom)),
            Array::List(list) => Box::new(list.iter().cloned()),
        }
    }
}

impl IntoIterator for Array {
    type Item = Val;
    type IntoIter = Box<dyn Iterator<Item = Val>>;
    fn into_iter(self) -> Self::IntoIter {
        match self {
            Array::String(s) => Box::new(
                s.chars()
                    .collect::<Vec<_>>()
                    .into_iter()
                    .map(Atom::Char)
                    .map(Val::Atom),
            ),
            Array::List(list) => Box::new(list.into_iter()),
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

impl fmt::Display for Atom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Atom::Num(n) => n.fmt(f),
            Atom::Op(op) => op.fmt(f),
            Atom::Char(c) => write!(f, "{:?}", c),
        }
    }
}

impl fmt::Display for Array {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Array::String(s) => write!(f, "{:?}", s),
            Array::List(items) => {
                write!(f, "〈")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    item.fmt(f)?;
                }
                write!(f, "〉")
            }
        }
    }
}

impl Atom {
    pub fn ty(&self) -> AtomType {
        match self {
            Atom::Num(_) => AtomType::Num,
            Atom::Char(_) => AtomType::Char,
            Atom::Op(_) => AtomType::Op,
        }
    }
}

impl Val {
    pub fn ty(&self) -> Ty {
        match self {
            Val::Atom(atom) => atom.ty().into(),
            Val::Array(arr) => match arr {
                Array::String(s) => {
                    ArrayType::StaticHomo(Box::new(AtomType::Char.into()), s.chars().count())
                }
                Array::List(items) => {
                    let mut types: Vec<Ty> = items.iter().map(Val::ty).collect();
                    if types.windows(2).all(|win| win[0] == win[1]) {
                        let len = types.len();
                        if let Some(ty) = types.pop() {
                            ArrayType::StaticHomo(ty.into(), len)
                        } else {
                            ArrayType::Empty
                        }
                    } else {
                        ArrayType::StaticHetero(types)
                    }
                }
            }
            .into(),
        }
    }
}

impl From<bool> for Atom {
    fn from(b: bool) -> Self {
        Atom::Num(Num::Int(b as i64))
    }
}

impl From<Atom> for Val {
    fn from(atom: Atom) -> Self {
        Val::Atom(atom)
    }
}

impl From<Num> for Val {
    fn from(num: Num) -> Self {
        Atom::Num(num).into()
    }
}

impl From<Op> for Val {
    fn from(op: Op) -> Self {
        Atom::Op(op).into()
    }
}

impl From<char> for Val {
    fn from(c: char) -> Self {
        Atom::Char(c).into()
    }
}

impl From<Array> for Val {
    fn from(arr: Array) -> Self {
        Val::Array(arr)
    }
}

impl Atom {
    pub fn add(self, other: Self) -> EvalResult<Self> {
        match (self, other) {
            (Atom::Num(a), Atom::Num(b)) => Ok(Atom::Num(a + b)),
            (Atom::Num(a), Atom::Char(b)) => Ok(Atom::Char(
                char::from_u32(u32::from(a).saturating_add(b as u32)).unwrap_or_default(),
            )),
            (Atom::Char(a), Atom::Num(b)) => Ok(Atom::Char(
                char::from_u32((a as u32).saturating_add(u32::from(b))).unwrap_or_default(),
            )),
            (w, x) => Op::Add.err_bin(AtomType::Char, AtomType::Char),
        }
    }
    pub fn sub(self, other: Self) -> EvalResult<Self> {
        match (self, other) {
            (Atom::Num(a), Atom::Num(b)) => Ok(Atom::Num(a - b)),
            (Atom::Num(a), Atom::Char(b)) => Ok(Atom::Char(
                char::from_u32(u32::from(a).saturating_sub(b as u32)).unwrap_or_default(),
            )),
            (Atom::Char(a), Atom::Num(b)) => Ok(Atom::Char(
                char::from_u32((a as u32).saturating_sub(u32::from(b))).unwrap_or_default(),
            )),
            (w, x) => Op::Sub.err_bin(AtomType::Char, AtomType::Char),
        }
    }
    pub fn mul(self, other: Self) -> EvalResult<Self> {
        match (self, other) {
            (Atom::Num(a), Atom::Num(b)) => Ok(Atom::Num(a * b)),
            (w, x) => Op::Mul.err_bin(w.ty(), x.ty()),
        }
    }
    pub fn div(self, other: Self) -> EvalResult<Self> {
        match (self, other) {
            (Atom::Num(a), Atom::Num(b)) => Ok(Atom::Num(a / b)),
            (w, x) => Op::Div.err_bin(w.ty(), x.ty()),
        }
    }
}

impl FromIterator<Val> for Array {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = Val>,
    {
        Array::from_try_iter::<_, Infallible>(iter.into_iter().map(Ok)).unwrap()
    }
}

impl Array {
    pub fn from_try_iter<I, E>(iter: I) -> Result<Array, E>
    where
        I: IntoIterator<Item = Result<Val, E>>,
    {
        let items: Vec<Val> = iter.into_iter().collect::<Result<_, _>>()?;
        Ok(
            if items
                .iter()
                .all(|value| matches!(value, Val::Atom(Atom::Char(_))))
            {
                Array::String(
                    items
                        .into_iter()
                        .map(|val| {
                            if let Val::Atom(Atom::Char(c)) = val {
                                c
                            } else {
                                unreachable!()
                            }
                        })
                        .collect::<String>()
                        .into(),
                )
            } else {
                Array::List(items)
            },
        )
    }
}
