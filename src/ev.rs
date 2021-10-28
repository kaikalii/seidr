use std::fmt;

use crate::{
    error::CompileResult,
    num::Num,
    op::Op,
    types::{ArrayType, AtomType, Ty},
    value::{Array, Atom, Val},
};

#[derive(Debug, PartialEq, Eq)]
pub enum Ev {
    Type(Ty),
    Value(Val),
}

impl Ev {
    pub fn ty(&self) -> Ty {
        match self {
            Ev::Type(ty) => ty.clone(),
            Ev::Value(val) => val.ty(),
        }
    }
}

impl fmt::Display for Ev {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Ev::Type(ty) => ty.fmt(f),
            Ev::Value(val) => val.fmt(f),
        }
    }
}

impl From<Ev> for Ty {
    fn from(c: Ev) -> Self {
        match c {
            Ev::Value(val) => val.ty(),
            Ev::Type(ty) => ty,
        }
    }
}

impl From<Ty> for Ev {
    fn from(ty: Ty) -> Self {
        Ev::Type(ty)
    }
}

impl From<ArrayType> for Ev {
    fn from(at: ArrayType) -> Self {
        Ev::Type(at.into())
    }
}

impl From<AtomType> for Ev {
    fn from(at: AtomType) -> Self {
        Ev::Type(at.into())
    }
}

impl From<Val> for Ev {
    fn from(val: Val) -> Self {
        Ev::Value(val)
    }
}

impl From<Atom> for Ev {
    fn from(atom: Atom) -> Self {
        Ev::Value(atom.into())
    }
}

impl From<Num> for Ev {
    fn from(num: Num) -> Self {
        Ev::Value(num.into())
    }
}

impl From<char> for Ev {
    fn from(c: char) -> Self {
        Ev::Value(c.into())
    }
}

impl From<Op> for Ev {
    fn from(op: Op) -> Self {
        Ev::Value(op.into())
    }
}

impl From<Array> for Ev {
    fn from(arr: Array) -> Self {
        Ev::Value(arr.into())
    }
}

impl Ev {
    pub fn from_try_iter<I>(iter: I) -> CompileResult<Self>
    where
        I: IntoIterator<Item = CompileResult<Ev>>,
    {
        let mut consts: Vec<Ev> = iter.into_iter().collect::<CompileResult<_>>()?;
        Ok(if consts.is_empty() {
            Array::List(Vec::new()).into()
        } else if consts.iter().all(|ty| matches!(ty, Ev::Value(_))) {
            Val::Array(Array::from_try_iter(consts.into_iter().map(|ty| {
                Ok(if let Ev::Value(val) = ty {
                    val
                } else {
                    unreachable!()
                })
            }))?)
            .into()
        } else {
            let mut types: Vec<Ty> = consts.into_iter().map(Ty::from).collect();
            let all_same = types.windows(2).all(|win| win[0] == win[1]);
            if all_same {
                let len = types.len();
                ArrayType::StaticHomo(types.pop().unwrap().into(), len)
            } else {
                ArrayType::StaticHetero(types)
            }
            .into()
        })
    }
}
