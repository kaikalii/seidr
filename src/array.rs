use std::{
    borrow::Cow,
    cell::RefCell,
    cmp::Ordering,
    collections::HashMap,
    fmt,
    iter::{self, once},
    rc::Rc,
};

use crate::{
    error::RuntimeResult,
    eval::{eval_bin, eval_un, index_array},
    lex::Span,
    num::{modulus, Num},
    pervade::PervadedArray,
    rcview::{RcView, RcViewIntoIter},
    value::{Atom, Val},
};

type Items = RcView<Val>;

#[derive(Clone)]
pub enum Array {
    Concrete(Items),
    Cached(Rc<CachedArray>),
    Rotate(Box<Self>, i64),
    Reverse(Box<Self>),
    Range(Num),
    Product(RcView<Self>, Items),
    JoinTo(Box<Self>, Box<Self>),
    Pervaded(Box<PervadedArray>),
    Take(Box<Self>, i64),
    Drop(Box<Self>, i64),
    Each(Box<ZipForm>, Box<Val>, Span),
    Select(Box<Self>, Box<Self>, Span),
}

fn min_len(a: Option<usize>, b: Option<usize>) -> Option<usize> {
    Some(match (a, b) {
        (Some(a), Some(b)) => a.min(b),
        (Some(a), None) => a,
        (None, Some(b)) => b,
        (None, None) => return None,
    })
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
    pub fn into_vec(self) -> RuntimeResult<Vec<Val>> {
        match self {
            Array::Concrete(items) => Ok(items.into_iter().collect()),
            arr => arr.into_iter().collect(),
        }
    }
    pub fn cache(self) -> Self {
        Array::Cached(Rc::new(CachedArray {
            arr: self,
            cache: Default::default(),
        }))
    }
    pub fn len(&self) -> Option<usize> {
        Some(match self {
            Array::Concrete(items) => items.len(),
            Array::Cached(arr) => arr.len()?,
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
            Array::Drop(arr, n) => {
                if *n >= 0 {
                    arr.len()?.saturating_sub(*n as usize)
                } else if let Some(len) = arr.len() {
                    len.saturating_sub(n.abs() as usize)
                } else {
                    return None;
                }
            }
            Array::Each(zip, ..) => zip.len()?,
            Array::Select(a, b, _) => min_len(a.len(), b.len())?,
        })
    }
    pub fn get(&self, index: usize) -> RuntimeResult<Option<Cow<Val>>> {
        Ok(match self {
            Array::Concrete(items) => items.get(index).map(Cow::Borrowed),
            Array::Cached(arr) => arr.get(index)?.map(Cow::Owned),
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
            Array::Drop(arr, n) => {
                if *n >= 0 {
                    let n = *n as usize;
                    arr.get(index + n)?
                } else if let Some(len) = arr.len() {
                    let n = n.abs() as usize;
                    if n >= len {
                        None
                    } else {
                        arr.get(len + index - n)?
                    }
                } else {
                    None
                }
            }
            Array::Each(zip, f, span) => zip
                .index_apply(
                    index,
                    |x| eval_un(Val::clone(f), x, span),
                    |w, x| eval_bin(Val::clone(f), w, x, span),
                )?
                .map(Cow::Owned),
            Array::Select(w, x, span) => {
                let w = if let Some(w) = w.get(index)? {
                    w.into_owned()
                } else {
                    return Ok(None);
                };
                Some(Cow::Owned(index_array(w, x, span)?))
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
        if self.len().is_none() || other.len().is_none() || self.len() != other.len() {
            false
        } else {
            self.iter().zip(other.iter()).all(|(a, b)| {
                if let (Ok(a), Ok(b)) = (a, b) {
                    a == b
                } else {
                    false
                }
            })
        }
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
        if self.len().is_none() {
            if other.len().is_none() {
                Ordering::Equal
            } else {
                Ordering::Less
            }
        } else if other.len().is_none() {
            Ordering::Greater
        } else {
            let mut a = self.iter();
            let mut b = other.iter();
            loop {
                match (a.next(), b.next()) {
                    (Some(a), Some(b)) => {
                        let ordering = if let (Ok(a), Ok(b)) = (a, b) {
                            a.cmp(&b)
                        } else {
                            Ordering::Equal
                        };
                        if ordering != Ordering::Equal {
                            break ordering;
                        }
                    }
                    (Some(_), None) => break Ordering::Greater,
                    (None, Some(_)) => break Ordering::Less,
                    (None, None) => break Ordering::Equal,
                }
            }
        }
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
                    Err(e) => write!(f, "<error: {}>", e.message)?,
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

#[derive(Clone)]
pub enum ZipForm {
    Un(Array),
    BinLeft(Val, Array),
    BinRight(Array, Val),
    Bin(Array, Array),
}

impl ZipForm {
    pub fn bin(w: Val, x: Val) -> Result<Self, (Atom, Atom)> {
        match (w, x) {
            (Val::Array(w), Val::Array(x)) => Ok(ZipForm::Bin(w, x)),
            (w, Val::Array(x)) => Ok(ZipForm::BinLeft(w, x)),
            (Val::Array(w), x) => Ok(ZipForm::BinRight(w, x)),
            (Val::Atom(w), Val::Atom(x)) => Err((w, x)),
        }
    }
    pub fn len(&self) -> Option<usize> {
        match self {
            ZipForm::Un(arr) | ZipForm::BinLeft(_, arr) | ZipForm::BinRight(arr, _) => arr.len(),
            ZipForm::Bin(a, b) => min_len(a.len(), b.len()),
        }
    }
    pub fn index_apply<U, B>(&self, index: usize, un: U, bin: B) -> RuntimeResult<Option<Val>>
    where
        U: FnOnce(Val) -> RuntimeResult,
        B: FnOnce(Val, Val) -> RuntimeResult,
    {
        Ok(Some(match self {
            ZipForm::Un(arr) => {
                let x = if let Some(x) = arr.get(index)? {
                    x.into_owned()
                } else {
                    return Ok(None);
                };
                un(x)?
            }
            ZipForm::BinLeft(w, arr) => {
                let x = if let Some(x) = arr.get(index)? {
                    x.into_owned()
                } else {
                    return Ok(None);
                };
                bin(w.clone(), x)?
            }
            ZipForm::BinRight(arr, x) => {
                let w = if let Some(w) = arr.get(index)? {
                    w.into_owned()
                } else {
                    return Ok(None);
                };
                bin(w, x.clone())?
            }
            ZipForm::Bin(w, x) => {
                let w = if let Some(w) = w.get(index)? {
                    w.into_owned()
                } else {
                    return Ok(None);
                };
                let x = if let Some(x) = x.get(index)? {
                    x.into_owned()
                } else {
                    return Ok(None);
                };
                bin(w, x)?
            }
        }))
    }
}

pub struct CachedArray {
    arr: Array,
    cache: RefCell<HashMap<usize, Val>>,
}

impl CachedArray {
    pub fn len(&self) -> Option<usize> {
        self.arr.len()
    }
    pub fn get(&self, index: usize) -> RuntimeResult<Option<Val>> {
        Ok(if let Some(val) = self.cache.borrow().get(&index) {
            Some(val.clone())
        } else if let Some(val) = self.arr.get(index)? {
            let val = val.into_owned();
            self.cache.borrow_mut().insert(index, val.clone());
            Some(val)
        } else {
            None
        })
    }
}
