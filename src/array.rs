use std::{
    borrow::{Borrow, Cow},
    cmp::Ordering,
    fmt,
    iter::{self, once},
    ops::{Bound, Deref, RangeBounds},
    rc::Rc,
};

use crate::{
    error::{RuntimeError, RuntimeResult},
    lex::Span,
    num::modulus,
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
}

impl Array {
    pub fn concrete<I>(items: I) -> Array
    where
        I: IntoIterator,
        I::Item: Into<Val>,
    {
        Array::Concrete(items.into_iter().map(Into::into).collect())
    }
    pub fn len(&self) -> usize {
        match self {
            Array::Concrete(items) => items.len(),
            Array::Rotate(arr, _) | Array::Reverse(arr) => arr.len(),
            Array::Range(n) => *n,
            Array::Product(arrs, _) => arrs[0].len(),
        }
    }
    pub fn get(&self, index: usize) -> Option<Cow<Val>> {
        match self {
            Array::Concrete(items) => items.get(index).map(Cow::Borrowed),
            Array::Rotate(arr, r) => {
                if index >= arr.len() {
                    None
                } else {
                    let index = modulus(index as i64 + *r, arr.len() as i64) as usize;
                    arr.get(index)
                }
            }
            Array::Reverse(arr) => {
                if index >= arr.len() {
                    None
                } else {
                    arr.get(arr.len() - 1 - index)
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
        }
    }
    pub fn cow_iter(&self) -> impl Iterator<Item = Cow<Val>> {
        let mut i = 0;
        iter::from_fn(move || {
            i += 1;
            self.get(i - 1)
        })
    }
    pub fn iter(&self) -> impl Iterator<Item = Val> + '_ {
        self.cow_iter().map(Cow::into_owned)
    }
    pub fn pervade<F, V>(&self, f: F) -> RuntimeResult<Self>
    where
        F: Fn(Val) -> RuntimeResult<V>,
        V: Into<Val>,
    {
        let mut items = Vec::new();
        for item in self.cow_iter().map(Cow::into_owned) {
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
        for (a, b) in self.cow_iter().zip(other.cow_iter()) {
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
        if self.len() > 0
            && self
                .cow_iter()
                .all(|val| matches!(val.as_ref(), Val::Atom(Atom::Char(_))))
        {
            let mut s = String::new();
            for val in self.cow_iter() {
                if let Val::Atom(Atom::Char(c)) = val.as_ref() {
                    s.push(*c);
                }
            }
            write!(f, "{:?}", s)
        } else if self.len() >= 2
            && self
                .cow_iter()
                .all(|val| matches!(val.as_ref(), Val::Atom(_)))
        {
            for (i, val) in self.cow_iter().enumerate() {
                if i > 0 {
                    write!(f, "‿")?;
                }
                val.fmt(f)?;
            }
            Ok(())
        } else {
            write!(f, "〈")?;
            for (i, val) in self.cow_iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                val.fmt(f)?;
            }
            write!(f, "〉")
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

#[derive(Clone)]
pub struct RcView<T> {
    items: Rc<[T]>,
    start: usize,
    end: usize,
}

impl<T> RcView<T> {
    pub fn new<I>(items: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        Self::from_iter(items)
    }
    pub fn sub<R>(&self, range: R) -> Self
    where
        R: RangeBounds<usize>,
    {
        let len = self.end - self.start;
        let start = match range.start_bound() {
            Bound::Unbounded => self.start,
            Bound::Included(i) => self.start + *i,
            Bound::Excluded(i) => self.start + *i + 1,
        };
        let end = match range.end_bound() {
            Bound::Unbounded => self.end,
            Bound::Included(i) => *i + 2 - (start - self.start),
            Bound::Excluded(i) => *i + 1 - (start - self.start),
        };
        RcView {
            items: self.items.clone(),
            start,
            end,
        }
    }
}

impl<T> From<Rc<[T]>> for RcView<T> {
    fn from(items: Rc<[T]>) -> Self {
        let start = 0;
        let end = items.len();
        RcView { items, start, end }
    }
}

impl<T> From<Vec<T>> for RcView<T> {
    fn from(items: Vec<T>) -> Self {
        Self::from(Rc::from(items))
    }
}

impl<T> FromIterator<T> for RcView<T> {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        let items: Rc<[T]> = iter.into_iter().collect();
        items.into()
    }
}

impl<T> Deref for RcView<T> {
    type Target = [T];
    fn deref(&self) -> &Self::Target {
        &self.items[self.start..self.end]
    }
}

impl<T> AsRef<[T]> for RcView<T> {
    fn as_ref(&self) -> &[T] {
        self
    }
}

impl<T> Borrow<[T]> for RcView<T> {
    fn borrow(&self) -> &[T] {
        self
    }
}

impl<T> IntoIterator for RcView<T>
where
    T: Clone,
{
    type Item = T;
    type IntoIter = RcViewIntoIter<T>;
    fn into_iter(self) -> Self::IntoIter {
        if Rc::strong_count(&self.items) + Rc::weak_count(&self.items) == 1 {
            RcViewIntoIter::Raw {
                len: self.items.len(),
                index: 0,
                ptr: Rc::into_raw(self.items) as *const T,
            }
        } else {
            RcViewIntoIter::Cloned {
                index: 0,
                rcv: self,
            }
        }
    }
}

pub enum RcViewIntoIter<T>
where
    T: Clone,
{
    Cloned {
        index: usize,
        rcv: RcView<T>,
    },
    Raw {
        ptr: *const T,
        index: usize,
        len: usize,
    },
}

impl<T> Iterator for RcViewIntoIter<T>
where
    T: Clone,
{
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            RcViewIntoIter::Cloned { index, rcv } => {
                let item = rcv.get(*index)?.clone();
                *index += 1;
                Some(item)
            }
            RcViewIntoIter::Raw { ptr, index, len } => {
                if index < len {
                    unsafe {
                        let item = std::ptr::read(ptr.add(*index));
                        *index += 1;
                        Some(item)
                    }
                } else {
                    None
                }
            }
        }
    }
}

impl<T> Drop for RcViewIntoIter<T>
where
    T: Clone,
{
    fn drop(&mut self) {
        if let RcViewIntoIter::Raw { .. } = self {
            for item in self {
                drop(item)
            }
        }
    }
}

#[test]
fn rc_view_into_iter() {
    let items = RcView::new(0..10);
    let clone = items.clone();
    for (i, j) in items.into_iter().enumerate() {
        assert_eq!(i, j);
    }
    for (i, j) in clone.into_iter().enumerate() {
        assert_eq!(i, j);
    }
}
