use std::fmt;

use crate::{
    error::CompileResult,
    ev::Ev,
    num::Num,
    value::{Array, Val},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ty {
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
    StaticHomo(Box<Ty>, usize),
    DynamicHomo(Box<Ty>),
    StaticHetero(Vec<Ty>),
}

impl fmt::Display for Ty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Ty::Atom(ty) => ty.fmt(f),
            Ty::Array(ty) => ty.fmt(f),
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

impl From<AtomType> for Ty {
    fn from(at: AtomType) -> Self {
        Ty::Atom(at)
    }
}

impl From<ArrayType> for Ty {
    fn from(at: ArrayType) -> Self {
        Ty::Array(at)
    }
}

impl FromIterator<Ty> for Ty {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = Ty>,
    {
        let mut tys: Vec<Ty> = iter.into_iter().collect();
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
