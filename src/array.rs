use std::{
    borrow::Cow,
    cmp::Ordering,
    fmt,
    iter::{self, once},
};

use crate::{
    error::RuntimeResult,
    num::{modulus, Num},
    pervade::PervadedArray,
    rcview::{RcView, RcViewIntoIter},
    value::{Atom, Val},
};

type Items = RcView<Val>;

#[derive(Clone)]
pub enum Array {
    Concrete(Items),
    Rotate(Box<Self>, i64),
    Reverse(Box<Self>),
    Range(Num),
    Product(RcView<Self>, Items),
    JoinTo(Box<Self>, Box<Self>),
    Pervaded(Box<PervadedArray>),
    Take(Box<Self>, i64),
}

impl Array {
    pub fn try_concrete<I>(items: I) -> RuntimeResult<Array>
    where
        I: IntoIterator,
        I::Item: Into<RuntimeResult>,
    {
        Ok(Array::Concrete(
            items
                .into_iter()
                .map(Into::into)
                .collect::<RuntimeResult<_>>()?,
        ))
    }
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
            Array::Range(n) => {
                if n.is_infinite() {
                    return None;
                } else {
                    i64::from(*n) as usize
                }
            }
            Array::Product(arrs, _) => arrs[0].len()?,
            Array::JoinTo(a, b) => a.len().zip(b.len()).map(|(a, b)| a + b)?,
            Array::Pervaded(pa) => pa.len()?,
            Array::Take(arr, n) => match (arr.len(), *n >= 0) {
                (Some(len), true) => len.min(*n as usize),
                (Some(len), false) => len.min(n.abs() as usize),
                (None, true) => *n as usize,
                (None, false) => 0,
            },
        })
    }
    pub fn get(&self, index: usize) -> RuntimeResult<Option<Cow<Val>>> {
        Ok(match self {
            Array::Concrete(items) => items.get(index).map(Cow::Borrowed),
            Array::Rotate(arr, r) => {
                if let Some(len) = arr.len() {
                    if index >= len {
                        None
                    } else {
                        let index = modulus(index as i64 + *r, len as i64) as usize;
                        arr.get(index)?
                    }
                } else if *r >= 0 {
                    let index = index + *r as usize;
                    arr.get(index)?
                } else {
                    None
                }
            }
            Array::Reverse(arr) => {
                if let Some(len) = arr.len() {
                    if index >= len {
                        None
                    } else {
                        arr.get(len - 1 - index)?
                    }
                } else {
                    None
                }
            }
            Array::Range(n) => {
                let n = i64::from(*n) as usize;
                if index >= n {
                    None
                } else {
                    Some(Cow::Owned(index.into()))
                }
            }
            Array::Product(arrs, items) => arrs[0].get(index)?.map(|first| {
                if arrs.len() == 1 {
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
                }
            }),
            Array::JoinTo(a, b) => {
                if let Some(val) = a.get(index)? {
                    Some(val)
                } else if let Some(len) = a.len() {
                    b.get(index - len)?
                } else {
                    None
                }
            }
            Array::Pervaded(pa) => pa.get(index)?.map(Cow::Owned),
            Array::Take(arr, n) => {
                if *n >= 0 {
                    let n = *n as usize;
                    if index < n {
                        arr.get(index)?
                    } else {
                        None
                    }
                } else if let Some(len) = arr.len() {
                    let n = n.abs() as usize;
                    arr.get(len - n + index)?
                } else {
                    None
                }
            }
        })
    }
    pub fn iter(&self) -> impl Iterator<Item = RuntimeResult<Cow<Val>>> {
        let mut i = 0;
        iter::from_fn(move || {
            i += 1;
            self.get(i - 1).transpose()
        })
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
        let len = if let Some(len) = self.len() { len } else { 5 };
        if len > 0
            && self
                .iter()
                .take(len)
                .all(|val| matches!(val.as_deref(), Ok(Val::Atom(Atom::Char(_)))))
        {
            let mut s = String::new();
            for val in self.iter().take(len) {
                if let Ok(Val::Atom(Atom::Char(c))) = val.as_deref() {
                    s.push(*c);
                } else {
                    s.push('?');
                }
            }
            let s = format!("{:?}", s);
            write!(f, "{}", &s[..s.len() - 1])?;
            if self.len().is_none() {
                write!(f, "...")?;
            }
            write!(f, "\"")
        } else {
            write!(f, "⟨")?;
            for (i, val) in self.iter().take(len).enumerate() {
                if i > 0 {
                    write!(f, " ")?;
                }
                match val {
                    Ok(val) => val.fmt(f)?,
                    Err(_) => write!(f, "<error>")?,
                }
            }
            if self.len().is_none() {
                write!(f, "...")?;
            }
            write!(f, "⟩")
        }
    }
}

impl From<PervadedArray> for Array {
    fn from(pa: PervadedArray) -> Self {
        Array::Pervaded(pa.into())
    }
}

impl IntoIterator for Array {
    type Item = RuntimeResult;
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
    type Item = RuntimeResult;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            ArrayIntoIter::RcView(iter) => iter.next().map(Ok),
            ArrayIntoIter::Get { index, array } => {
                let item = array.get(*index).transpose()?;
                *index += 1;
                Some(item.map(Cow::into_owned))
            }
            ArrayIntoIter::JoinTo(a, b) => a.next().or_else(|| b.next()),
        }
    }
}
