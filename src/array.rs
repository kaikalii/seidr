use std::{cmp::Ordering, fmt, rc::Rc};

use crate::{
    ast::{Bin, Un},
    lex::Sp,
    op::Pervasive,
    value::Val,
};

#[derive(Clone)]
pub enum Array {
    Concrete(Rc<[Val]>),
    PervasiveUn(Rc<PervasiveUn>),
    PervasiveBin(Rc<PervasiveBin>),
}

pub type PervasiveUn = Un<Sp<Pervasive>, Array>;
pub type PervasiveBin = Bin<Sp<Pervasive>, Array, Array>;

impl Array {
    pub fn len(&self) -> usize {
        match self {
            Array::Concrete(arr) => arr.len(),
            Array::PervasiveUn(un) => un.x.len(),
            Array::PervasiveBin(bin) => bin.x.len(),
        }
    }
    pub fn index(&self, index: usize) -> Val {
        match self {
            Array::Concrete(arr) => arr[index].clone(),
            Array::PervasiveUn(un) => todo!(),
            Array::PervasiveBin(bin) => todo!(),
        }
    }
}

macro_rules! array_from {
    ($variant:ident) => {
        array_from!($variant, $variant);
    };
    ($variant:ident, $type:ty) => {
        impl From<$type> for Array {
            fn from(inner: $type) -> Self {
                Array::$variant(inner.into())
            }
        }
    };
}

array_from!(PervasiveUn);
array_from!(PervasiveBin);

impl PartialEq for Array {
    fn eq(&self, other: &Self) -> bool {
        todo!()
    }
}

impl Eq for Array {}

impl PartialOrd for Array {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Array {
    fn cmp(&self, other: &Self) -> Ordering {
        todo!()
    }
}

impl fmt::Debug for Array {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl fmt::Display for Array {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl<V> FromIterator<V> for Array
where
    V: Into<Val>,
{
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = V>,
    {
        Array::Concrete(iter.into_iter().map(Into::into).collect())
    }
}
