use std::fmt;

use crate::{
    checked::Checked,
    error::CompileResult,
    num::Num,
    value::{Array, Value},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Atom(AtomType),
    Array(Box<ArrayType>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AtomType {
    Num,
    Char,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArrayType {
    Empty,
    StaticHomo(Type, usize),
    DynamicHomo(Type),
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
        Type::Array(Box::new(at))
    }
}
