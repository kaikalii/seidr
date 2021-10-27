use std::fmt;

use crate::{
    error::CompileResult,
    eval::Const,
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
            ArrayType::Empty => write!(f, "[]"),
            ArrayType::StaticHomo(ty, n) => write!(f, "[{}; {}]", ty, n),
            ArrayType::DynamicHomo(ty) => write!(f, "[{}]", ty),
            ArrayType::StaticHetero(tys) => f.debug_list().entries(tys).finish(),
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

// impl Const {
//     fn from_iter<I>(iter: I) -> CompileResult<Self>
//     where
//         I: IntoIterator<Item = CompileResult<Const>>,
//     {
//         let mut consts: Vec<Const> = iter.into_iter().collect::<CompileResult<_>>()?;
//         Ok(if consts.is_empty() {
//             Array::List(Vec::new()).into()
//         } else if consts.iter().all(|ty| matches!(ty, Const::Value(_))) {
//             Value::Array(Array::from_iter(consts.into_iter().map(|ty| {
//                 Ok(if let Const::Value(val) = ty {
//                     val
//                 } else {
//                     unreachable!()
//                 })
//             }))?)
//             .into()
//         } else {
//             let all_same = consts.windows(2).all(|win| win[0] == win[1]);
//             if all_same {
//                 let len = consts.len();
//                 ArrayType::StaticHomo(consts.pop().unwrap(), len)
//             } else {
//                 ArrayType::StaticHetero(consts)
//             }
//             .into()
//         })
//     }
// }
