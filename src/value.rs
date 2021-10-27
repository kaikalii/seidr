use std::{fmt, ops::*};

use crate::{
    error::{CompileErrorKind, CompileResult},
    lex::Span,
    num::Num,
    op::Op,
    types::{ArrayType, AtomType, Type},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Atom {
    Num(Num),
    Char(char),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Atom(Atom),
    Array(Array),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Array {
    String(String),
    List(Vec<Value>),
}

impl Array {
    pub fn iter(&self) -> Box<dyn Iterator<Item = Value> + '_> {
        match self {
            Array::String(s) => Box::new(s.chars().map(Atom::Char).map(Value::Atom)),
            Array::List(list) => Box::new(list.iter().cloned()),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Atom(atom) => atom.fmt(f),
            Value::Array(arr) => arr.fmt(f),
        }
    }
}

impl fmt::Display for Atom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Atom::Num(n) => n.fmt(f),
            Atom::Char(c) => write!(f, "{:?}", c),
        }
    }
}

impl fmt::Display for Array {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Array::String(s) => write!(f, "{:?}", s),
            Array::List(items) => {
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    item.fmt(f)?;
                }
                write!(f, "]")
            }
        }
    }
}

impl Atom {
    pub fn ty(&self) -> AtomType {
        match self {
            Atom::Num(_) => AtomType::Num,
            Atom::Char(_) => AtomType::Char,
        }
    }
}

impl Value {
    pub fn ty(&self) -> Type {
        match self {
            Value::Atom(atom) => atom.ty().into(),
            Value::Array(arr) => match arr {
                Array::String(s) => ArrayType::StaticHomo(AtomType::Char.into(), s.chars().count()),
                Array::List(items) => {
                    let mut types: Vec<Type> = items.iter().map(Value::ty).collect();
                    if types.windows(2).all(|win| win[0] == win[1]) {
                        let len = types.len();
                        if let Some(ty) = types.pop() {
                            ArrayType::StaticHomo(ty, len)
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

impl From<Atom> for Value {
    fn from(atom: Atom) -> Self {
        Value::Atom(atom)
    }
}

impl From<Num> for Value {
    fn from(num: Num) -> Self {
        Value::Atom(Atom::Num(num))
    }
}

impl From<Array> for Value {
    fn from(arr: Array) -> Self {
        Value::Array(arr)
    }
}

impl Atom {
    pub fn add(self, other: Self, span: &Span) -> CompileResult<Self> {
        match (self, other) {
            (Atom::Num(a), Atom::Num(b)) => Ok(Atom::Num(a + b)),
            (Atom::Num(a), Atom::Char(b)) => Ok(Atom::Char(
                char::from_u32(u32::from(a).saturating_add(b as u32)).unwrap_or_default(),
            )),
            (Atom::Char(a), Atom::Num(b)) => Ok(Atom::Char(
                char::from_u32((a as u32).saturating_add(u32::from(b))).unwrap_or_default(),
            )),
            (Atom::Char(_), Atom::Char(_)) => Err(CompileErrorKind::IncompatibleBinTypes(
                Op::Add,
                AtomType::Char.into(),
                AtomType::Char.into(),
            )
            .at(span.clone())),
        }
    }
    pub fn sub(self, other: Self, span: &Span) -> CompileResult<Self> {
        match (self, other) {
            (Atom::Num(a), Atom::Num(b)) => Ok(Atom::Num(a - b)),
            (Atom::Num(a), Atom::Char(b)) => Ok(Atom::Char(
                char::from_u32(u32::from(a).saturating_sub(b as u32)).unwrap_or_default(),
            )),
            (Atom::Char(a), Atom::Num(b)) => Ok(Atom::Char(
                char::from_u32((a as u32).saturating_sub(u32::from(b))).unwrap_or_default(),
            )),
            (Atom::Char(_), Atom::Char(_)) => Err(CompileErrorKind::IncompatibleBinTypes(
                Op::Sub,
                AtomType::Char.into(),
                AtomType::Char.into(),
            )
            .at(span.clone())),
        }
    }
    pub fn mul(self, other: Self, span: &Span) -> CompileResult<Self> {
        match (self, other) {
            (Atom::Num(a), Atom::Num(b)) => Ok(Atom::Num(a * b)),
            (a, b) => {
                Err(
                    CompileErrorKind::IncompatibleBinTypes(Op::Sub, a.ty().into(), a.ty().into())
                        .at(span.clone()),
                )
            }
        }
    }
    pub fn div(self, other: Self, span: &Span) -> CompileResult<Self> {
        match (self, other) {
            (Atom::Num(a), Atom::Num(b)) => Ok(Atom::Num(a / b)),
            (a, b) => {
                Err(
                    CompileErrorKind::IncompatibleBinTypes(Op::Sub, a.ty().into(), a.ty().into())
                        .at(span.clone()),
                )
            }
        }
    }
}

impl Array {
    pub fn from_iter<I>(iter: I) -> CompileResult<Array>
    where
        I: IntoIterator<Item = CompileResult<Value>>,
    {
        let items: Vec<Value> = iter.into_iter().collect::<CompileResult<_>>()?;
        Ok(
            if items
                .iter()
                .all(|value| matches!(value, Value::Atom(Atom::Char(_))))
            {
                Array::String(
                    items
                        .into_iter()
                        .map(|val| {
                            if let Value::Atom(Atom::Char(c)) = val {
                                c
                            } else {
                                unreachable!()
                            }
                        })
                        .collect(),
                )
            } else {
                Array::List(items)
            },
        )
    }
}
