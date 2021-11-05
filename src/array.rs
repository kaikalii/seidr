use std::{
    borrow::Cow,
    cmp::Ordering,
    fmt,
    iter::{self, once},
};

use crate::{
    error::{RuntimeError, RuntimeResult},
    lex::Span,
    num::modulus,
    rcview::{RcView, RcViewIntoIter},
    value::{Atom, Val},
};

type Items = RcView<Val>;

#[derive(Clone)]
pub enum Array {
    Concrete(Items),
    Rotate(Box<Self>, i64),
    Reverse(Box<Self>),
    Range(usize),
    Product(RcView<Self>, Items),
    JoinTo(Box<Self>, Box<Self>),
}

impl Array {
    pub fn concrete<I>(items: I) -> Array
    where
        I: IntoIterator,
        I::Item: Into<Val>,
    {
        Array::Concrete(items.into_iter().map(Into::into).collect())
    }
    pub fn len(&self) -> Option<usize> {
        Some(match self {
            Array::Concrete(items) => items.len(),
            Array::Rotate(arr, _) | Array::Reverse(arr) => arr.len()?,
            Array::Range(n) => *n,
            Array::Product(arrs, _) => arrs[0].len()?,
            Array::JoinTo(a, b) => a.len().zip(b.len()).map(|(a, b)| a + b)?,
        })
    }
    pub fn get(&self, index: usize) -> Option<Cow<Val>> {
        match self {
            Array::Concrete(items) => items.get(index).map(Cow::Borrowed),
            Array::Rotate(arr, r) => {
                if let Some(len) = arr.len() {
                    if index >= len {
                        None
                    } else {
                        let index = modulus(index as i64 + *r, len as i64) as usize;
                        arr.get(index)
                    }
                } else if *r >= 0 {
                    let index = index + *r as usize;
                    arr.get(index)
                } else {
                    None
                }
            }
            Array::Reverse(arr) => {
                let len = arr.len()?;
                if index >= len {
                    None
                } else {
                    arr.get(len - 1 - index)
                }
            }
            Array::Range(n) => {
                if index >= *n {
                    None
                } else {
                    Some(Cow::Owned(index.into()))
                }
            }
            Array::Product(arrs, items) => {
                let first = arrs[0].get(index)?;
                Some(if arrs.len() == 1 {
                    if items.is_empty() {
                        first
                    } else {
                        Cow::Owned(
                            Array::concrete(items.iter().cloned().chain(once(first.into_owned())))
                                .into(),
                        )
                    }
                } else {
                    let val = first.into_owned();
                    Cow::Owned(
                        Array::Product(
                            arrs.sub(1..),
                            items.iter().cloned().chain(once(val)).collect(),
                        )
                        .into(),
                    )
                })
            }
            Array::JoinTo(a, b) => {
                if let Some(val) = a.get(index) {
                    Some(val)
                } else if let Some(len) = a.len() {
                    b.get(index - len)
                } else {
                    None
                }
            }
        }
    }
    pub fn iter(&self) -> impl Iterator<Item = Cow<Val>> {
        let mut i = 0;
        iter::from_fn(move || {
            i += 1;
            self.get(i - 1)
        })
    }
    pub fn pervade<F, V>(&self, f: F) -> RuntimeResult<Self>
    where
        F: Fn(Val) -> RuntimeResult<V>,
        V: Into<Val>,
    {
        let mut items = Vec::new();
        for item in self.iter().map(Cow::into_owned) {
            items.push(f(item)?.into());
        }
        Ok(Array::Concrete(items.into()))
    }
    pub fn pervade_with<F, V>(&self, other: &Self, span: &Span, f: F) -> RuntimeResult<Self>
    where
        F: Fn(Val, Val) -> RuntimeResult<V>,
        V: Into<Val>,
    {
        if self.len() != other.len() {
            return Err(RuntimeError::new(
                "Array lengths do not match",
                span.clone(),
            ));
        }
        let mut items = Vec::new();
        for (a, b) in self.iter().zip(other.iter()) {
            items.push(f(a.into_owned(), b.into_owned())?.into());
        }
        Ok(Array::Concrete(items.into()))
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
        let len = if let Some(len) = self.len() {
            len
        } else {
            return write!(f, "..");
        };
        if len > 0
            && self
                .iter()
                .all(|val| matches!(val.as_ref(), Val::Atom(Atom::Char(_))))
        {
            let mut s = String::new();
            for val in self.iter() {
                if let Val::Atom(Atom::Char(c)) = val.as_ref() {
                    s.push(*c);
                }
            }
            write!(f, "{:?}", s)
        } else if len >= 2 && self.iter().all(|val| matches!(val.as_ref(), Val::Atom(_))) {
            for (i, val) in self.iter().enumerate() {
                if i > 0 {
                    write!(f, "‿")?;
                }
                val.fmt(f)?;
            }
            Ok(())
        } else {
            write!(f, "〈")?;
            for (i, val) in self.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                val.fmt(f)?;
            }
            write!(f, "〉")
        }
    }
}

impl IntoIterator for Array {
    type Item = Val;
    type IntoIter = ArrayIntoIter;
    fn into_iter(self) -> Self::IntoIter {
        match self {
            Array::Concrete(rcv) => ArrayIntoIter::RcView(rcv.into_iter()),
            Array::JoinTo(a, b) => {
                ArrayIntoIter::JoinTo(a.into_iter().into(), b.into_iter().into())
            }
            array => ArrayIntoIter::Get { index: 0, array },
        }
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

pub enum ArrayIntoIter {
    RcView(RcViewIntoIter<Val>),
    Get { index: usize, array: Array },
    JoinTo(Box<Self>, Box<Self>),
}

impl Iterator for ArrayIntoIter {
    type Item = Val;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            ArrayIntoIter::RcView(iter) => iter.next(),
            ArrayIntoIter::Get { index, array } => {
                let item = array.get(*index)?;
                *index += 1;
                Some(item.into_owned())
            }
            ArrayIntoIter::JoinTo(a, b) => a.next().or_else(|| b.next()),
        }
    }
}
