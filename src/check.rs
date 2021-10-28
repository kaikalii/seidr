use std::fmt;

use crate::{
    error::CompileResult,
    num::Num,
    types::{ArrayType, AtomType, Type},
    value::{Array, Atom, Value},
};

#[derive(Debug, PartialEq, Eq)]
pub enum Check {
    Type(Type),
    Value(Value),
}

impl Check {
    pub fn ty(&self) -> Type {
        match self {
            Check::Type(ty) => ty.clone(),
            Check::Value(val) => val.ty(),
        }
    }
}

impl fmt::Display for Check {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Check::Type(ty) => ty.fmt(f),
            Check::Value(val) => val.fmt(f),
        }
    }
}

impl From<Check> for Type {
    fn from(c: Check) -> Self {
        match c {
            Check::Value(val) => val.ty(),
            Check::Type(ty) => ty,
        }
    }
}

impl From<Type> for Check {
    fn from(ty: Type) -> Self {
        Check::Type(ty)
    }
}

impl From<ArrayType> for Check {
    fn from(at: ArrayType) -> Self {
        Check::Type(at.into())
    }
}

impl From<AtomType> for Check {
    fn from(at: AtomType) -> Self {
        Check::Type(at.into())
    }
}

impl From<Value> for Check {
    fn from(val: Value) -> Self {
        Check::Value(val)
    }
}

impl From<Atom> for Check {
    fn from(atom: Atom) -> Self {
        Check::Value(atom.into())
    }
}

impl From<Num> for Check {
    fn from(num: Num) -> Self {
        Check::Value(num.into())
    }
}

impl From<char> for Check {
    fn from(c: char) -> Self {
        Check::Value(c.into())
    }
}

impl From<Array> for Check {
    fn from(arr: Array) -> Self {
        Check::Value(arr.into())
    }
}

impl Check {
    pub fn from_try_iter<I>(iter: I) -> CompileResult<Self>
    where
        I: IntoIterator<Item = CompileResult<Check>>,
    {
        let mut consts: Vec<Check> = iter.into_iter().collect::<CompileResult<_>>()?;
        Ok(if consts.is_empty() {
            Array::List(Vec::new()).into()
        } else if consts.iter().all(|ty| matches!(ty, Check::Value(_))) {
            Value::Array(Array::from_try_iter(consts.into_iter().map(|ty| {
                Ok(if let Check::Value(val) = ty {
                    val
                } else {
                    unreachable!()
                })
            }))?)
            .into()
        } else {
            let mut types: Vec<Type> = consts.into_iter().map(Type::from).collect();
            let all_same = types.windows(2).all(|win| win[0] == win[1]);
            if all_same {
                let len = types.len();
                ArrayType::StaticHomo(types.pop().unwrap(), len)
            } else {
                ArrayType::StaticHetero(types)
            }
            .into()
        })
    }
}
