use std::{cmp::Ordering, fmt, rc::Rc};

use crate::{
    ast::{Bin, Un},
    value::Val,
};

#[derive(Clone)]
pub enum Array {
    Concrete(Rc<[Val]>),
}

impl Array {
    pub fn len(&self) -> usize {
        match self {
            Array::Concrete(arr) => arr.len(),
        }
    }
    pub fn index(&self, index: usize) -> Val {
        match self {
            Array::Concrete(arr) => arr[index].clone(),
        }
    }
}

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
