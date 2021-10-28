use std::fmt;

use crate::{
    error::CompileResult,
    num::Num,
    types::{ArrayType, AtomType, Type},
    value::{Array, Atom, Value},
};

#[derive(Debug, PartialEq, Eq)]
pub enum Checked {
    Type(Type),
    Value(Value),
}

impl Checked {
    pub fn ty(&self) -> Type {
        match self {
            Checked::Type(ty) => ty.clone(),
            Checked::Value(val) => val.ty(),
        }
    }
}

impl fmt::Display for Checked {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Checked::Type(ty) => ty.fmt(f),
            Checked::Value(val) => val.fmt(f),
        }
    }
}

impl From<Checked> for Type {
    fn from(c: Checked) -> Self {
        match c {
            Checked::Value(val) => val.ty(),
            Checked::Type(ty) => ty,
        }
    }
}

impl From<Type> for Checked {
    fn from(ty: Type) -> Self {
        Checked::Type(ty)
    }
}

impl From<ArrayType> for Checked {
    fn from(at: ArrayType) -> Self {
        Checked::Type(at.into())
    }
}

impl From<AtomType> for Checked {
    fn from(at: AtomType) -> Self {
        Checked::Type(at.into())
    }
}

impl From<Value> for Checked {
    fn from(val: Value) -> Self {
        Checked::Value(val)
    }
}

impl From<Atom> for Checked {
    fn from(atom: Atom) -> Self {
        Checked::Value(atom.into())
    }
}

impl From<Num> for Checked {
    fn from(num: Num) -> Self {
        Checked::Value(num.into())
    }
}

impl From<char> for Checked {
    fn from(c: char) -> Self {
        Checked::Value(c.into())
    }
}

impl From<Array> for Checked {
    fn from(arr: Array) -> Self {
        Checked::Value(arr.into())
    }
}

impl Checked {
    pub fn from_try_iter<I>(iter: I) -> CompileResult<Self>
    where
        I: IntoIterator<Item = CompileResult<Checked>>,
    {
        let mut consts: Vec<Checked> = iter.into_iter().collect::<CompileResult<_>>()?;
        Ok(if consts.is_empty() {
            Array::List(Vec::new()).into()
        } else if consts.iter().all(|ty| matches!(ty, Checked::Value(_))) {
            Value::Array(Array::from_try_iter(consts.into_iter().map(|ty| {
                Ok(if let Checked::Value(val) = ty {
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
