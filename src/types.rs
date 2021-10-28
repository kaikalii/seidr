use std::fmt;

use crate::{
    check::Check,
    error::CompileResult,
    num::Num,
    value::{Array, Value},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Atom(AtomType),
    Array(ArrayType),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AtomType {
    Num,
    Char,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArrayType {
    Empty,
    StaticHomo(Box<Type>, usize),
    DynamicHomo(Box<Type>),
    StaticHetero(Vec<Type>),
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Atom(ty) => ty.fmt(f),
            Type::Array(ty) => ty.fmt(f),
        }
    }
}

impl fmt::Display for AtomType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AtomType::Num => "num".fmt(f),
            AtomType::Char => "char".fmt(f),
        }
    }
}

impl fmt::Display for ArrayType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArrayType::Empty => write!(f, "〈]"),
            ArrayType::StaticHomo(ty, n) => write!(f, "〈{}; {}〉", ty, n),
            ArrayType::DynamicHomo(ty) => write!(f, "〈{}〉", ty),
            ArrayType::StaticHetero(tys) => {
                write!(f, "〈")?;
                for (i, ty) in tys.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ");
                    }
                    ty.fmt(f)?;
                }
                write!(f, "〉")
            }
        }
    }
}

impl From<AtomType> for Type {
    fn from(at: AtomType) -> Self {
        Type::Atom(at)
    }
}

impl From<ArrayType> for Type {
    fn from(at: ArrayType) -> Self {
        Type::Array(at)
    }
}

impl FromIterator<Type> for Type {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = Type>,
    {
        let mut tys: Vec<Type> = iter.into_iter().collect();
        let all_same = tys.windows(2).all(|win| win[0] == win[1]);
        if all_same {
            let len = tys.len();
            ArrayType::StaticHomo(tys.pop().unwrap().into(), len)
        } else {
            ArrayType::StaticHetero(tys)
        }
        .into()
    }
}
