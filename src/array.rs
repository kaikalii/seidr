use std::{
    borrow::Cow,
    cell::RefCell,
    cmp::Ordering,
    collections::HashMap,
    iter::{self, once},
    rc::Rc,
};

use crate::{
    error::RuntimeResult,
    eval::{eval_bin, eval_un, index_array, replicator_int, rt_error},
    format::{Format, Formatter},
    lex::Span,
    num::{modulus, Num},
    pervade::PervadedArray,
    rcview::{RcView, RcViewIntoIter},
    value::{Atom, Val},
};

type Items = RcView<Val>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Array {
    Concrete(Items),
    AsciiString(Rc<str>),
    Cached(Rc<CachedArray>),
    Rotate(Box<Self>, i64),
    Reverse(Box<Self>),
    Range(Num),
    Product(RcView<Self>, Items),
    JoinTo(Box<Self>, Box<Self>),
    Pervaded(Box<PervadedArray>),
    Take(Box<Self>, i64),
    Drop(Box<Self>, i64),
    Each(Box<EachArray>),
    Select(Box<SelectArray>),
    Windows(Box<Self>, usize),
    Chunks(Box<Self>, usize),
    Replicate(Rc<ReplicateArray>),
    Scan(Rc<ScanArray>),
    Table(Rc<TableArray>),
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
    pub fn string<S>(s: S) -> Self
    where
        S: Into<Rc<str>>,
    {
        let s = s.into();
        if s.is_ascii() {
            Array::AsciiString(s)
        } else {
            Array::concrete(s.chars())
        }
    }
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
            Array::AsciiString(s) => s.len(),
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
            Array::Each(each, ..) => each.zip.len()?,
            Array::Select(sel) => min_len(sel.indices.len(), sel.array.len())?,
            Array::Windows(arr, size) => arr.len()?.saturating_sub(size.saturating_sub(1)).max(1),
            Array::Chunks(arr, size) => {
                let len = arr.len()?;
                if len % *size == 0 {
                    len / *size
                } else {
                    len / *size + 1
                }
            }
            Array::Replicate(_) => return None,
            Array::Scan(scan) => scan.len()?,
            Array::Table(table) => table.len()?,
        })
    }
    pub fn get(&self, index: usize) -> RuntimeResult<Option<Cow<Val>>> {
        Ok(match self {
            Array::Concrete(items) => items.get(index).map(Cow::Borrowed),
            Array::AsciiString(s) => s
                .as_bytes()
                .get(index)
                .copied()
                .map(Val::from)
                .map(Cow::Owned),
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
            Array::Each(each) => each
                .zip
                .index_apply(
                    index,
                    |x| eval_un(each.f.clone(), x, &each.span),
                    |w, x| eval_bin(each.f.clone(), w, x, &each.span),
                )?
                .map(Cow::Owned),
            Array::Select(sel) => {
                let w = if let Some(w) = sel.indices.get(index)? {
                    w.into_owned()
                } else {
                    return Ok(None);
                };
                Some(Cow::Owned(index_array(w, &sel.array, &sel.span)?))
            }
            Array::Windows(arr, size) => {
                if let Some(len) = self.len() {
                    if index >= len {
                        return Ok(None);
                    }
                }
                Some(Cow::Owned(
                    Array::Take(Array::Drop(arr.clone(), index as i64).into(), *size as i64).into(),
                ))
            }
            Array::Chunks(arr, size) => {
                if let Some(len) = self.len() {
                    if index >= len {
                        return Ok(None);
                    }
                }
                Some(Cow::Owned(
                    Array::Take(
                        Array::Drop(arr.clone(), (index * *size) as i64).into(),
                        *size as i64,
                    )
                    .into(),
                ))
            }
            Array::Replicate(rep) => rep.get(index)?,
            Array::Scan(scan) => scan.get(index)?.map(Cow::Owned),
            Array::Table(table) => table.get(index)?.map(Cow::Owned),
        })
    }
    pub fn iter(&self) -> impl Iterator<Item = RuntimeResult<Cow<Val>>> {
        let mut i = 0;
        iter::from_fn(move || {
            i += 1;
            self.get(i - 1).transpose()
        })
    }
    pub fn matches(&self, other: &Self) -> RuntimeResult<bool> {
        let len = self.len();
        Ok(if len == other.len() {
            if len.is_some() {
                if self == other {
                    true
                } else {
                    for (a, b) in self.iter().zip(other.iter()) {
                        if !a?.matches(b?.as_ref())? {
                            return Ok(false);
                        }
                    }
                    true
                }
            } else {
                self == other
            }
        } else {
            false
        })
    }
    pub fn depth(&self, span: &Span) -> RuntimeResult<usize> {
        let of_items = match self {
            Array::Range(_) => 0,
            arr if arr.len().is_none() => {
                return rt_error("Unbounded arrays do not have a depth", span)
            }
            arr => arr
                .iter()
                .fold(Ok(0), |acc, item| -> RuntimeResult<usize> {
                    Ok(acc?.max(item?.depth(span)?))
                })?,
        };
        Ok(1 + of_items)
    }
}

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

impl Format for Array {
    fn format(&self, f: &mut Formatter) -> RuntimeResult<()> {
        let len = if let Some(len) = self.len() { len } else { 5 };
        if len > 0
            && self
                .iter()
                .take(len)
                .all(|val| matches!(val.as_deref(), Ok(Val::Atom(Atom::Char(_)))))
        {
            let mut s = String::new();
            for val in self.iter().take(len) {
                if let Val::Atom(Atom::Char(c)) = val?.as_ref() {
                    s.push(*c);
                }
            }
            let s = format!("{:?}", s);
            f.display(&s[..s.len() - 1]);
            if self.len().is_none() {
                f.display("...");
            }
            f.display("\"");
        } else {
            f.display("⟨");
            for (i, val) in self.iter().take(len).enumerate() {
                let val = val?;
                if i > 0 {
                    f.display(" ");
                }
                val.format(f)?;
            }
            if self.len().is_none() {
                f.display(" ...");
            }
            f.display("⟩");
        }
        Ok(())
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

#[derive(Debug)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
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

type ArrayCache = RefCell<HashMap<usize, Val>>;

#[derive(Debug)]
pub struct CachedArray {
    arr: Array,
    cache: ArrayCache,
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

impl PartialEq for CachedArray {
    fn eq(&self, other: &Self) -> bool {
        self.arr == other.arr
    }
}

impl Eq for CachedArray {}

#[derive(Debug, Clone)]
pub struct EachArray {
    pub zip: ZipForm,
    pub f: Val,
    pub span: Span,
}

impl PartialEq for EachArray {
    fn eq(&self, other: &Self) -> bool {
        self.f == other.f && self.zip == other.zip
    }
}

impl Eq for EachArray {}

#[derive(Debug, Clone)]
pub struct SelectArray {
    pub indices: Array,
    pub array: Array,
    pub span: Span,
}

impl PartialEq for SelectArray {
    fn eq(&self, other: &Self) -> bool {
        self.indices == other.indices && self.array == other.array
    }
}

impl Eq for SelectArray {}

#[derive(Debug, PartialEq, Eq)]
pub enum ReplicateArray {
    Int { n: usize, array: Array },
    Counts(Box<CountsReplicateArray>),
}

#[derive(Debug)]
pub struct CountsReplicateArray {
    counts: RefCell<ArrayIntoIter>,
    array: RefCell<ArrayIntoIter>,
    cache: RefCell<Vec<Val>>,
    span: Span,
}

impl PartialEq for CountsReplicateArray {
    fn eq(&self, _: &Self) -> bool {
        false
    }
}

impl Eq for CountsReplicateArray {}

impl ReplicateArray {
    pub fn int(n: usize, array: Array) -> Self {
        ReplicateArray::Int { n, array }
    }
    pub fn counts(counts: Array, array: Array, span: Span) -> Self {
        ReplicateArray::Counts(
            CountsReplicateArray {
                counts: counts.into_iter().into(),
                array: array.into_iter().into(),
                cache: Vec::new().into(),
                span,
            }
            .into(),
        )
    }
    pub fn get(&self, index: usize) -> RuntimeResult<Option<Cow<Val>>> {
        Ok(match self {
            ReplicateArray::Counts(cra) => {
                let mut cache = cra.cache.borrow_mut();
                while cache.len() <= index {
                    let count = cra.counts.borrow_mut().next().transpose()?;
                    let val = cra.array.borrow_mut().next().transpose()?;
                    if let Some((count, val)) = count.zip(val) {
                        let n = replicator_int(count, &cra.span)?;
                        for _ in 0..n {
                            cache.push(val.clone());
                        }
                    } else {
                        break;
                    }
                }
                cache.get(index).cloned().map(Cow::Owned)
            }
            ReplicateArray::Int { n, array } => array.get(index / *n)?,
        })
    }
}

#[derive(Debug)]
pub struct ScanArray {
    f: Val,
    array: Array,
    init: Option<Val>,
    cache: RefCell<Vec<Val>>,
    span: Span,
}

impl PartialEq for ScanArray {
    fn eq(&self, other: &Self) -> bool {
        self.f == other.f && self.array == other.array && self.init == other.init
    }
}

impl Eq for ScanArray {}

impl ScanArray {
    pub fn new(f: Val, array: Array, init: Option<Val>, span: Span) -> Self {
        ScanArray {
            f,
            array,
            init,
            cache: Default::default(),
            span,
        }
    }
    pub fn len(&self) -> Option<usize> {
        self.array.len()
    }
    pub fn get(&self, index: usize) -> RuntimeResult<Option<Val>> {
        let mut cache = self.cache.borrow_mut();
        while cache.len() <= index {
            let i = cache.len();
            if i == 0 {
                if let Some(init) = &self.init {
                    if let Some(val) = self.array.get(i)? {
                        cache.push(eval_bin(
                            self.f.clone(),
                            init.clone(),
                            val.into_owned(),
                            &self.span,
                        )?);
                    } else {
                        break;
                    }
                } else if let Some(val) = self.array.get(i)? {
                    cache.push(val.into_owned())
                } else {
                    break;
                }
            } else {
                let w = self.array.get(i - 1)?.unwrap();
                let x = self.array.get(i)?;
                if let Some(x) = self.array.get(i)? {
                    cache.push(eval_bin(
                        self.f.clone(),
                        w.into_owned(),
                        x.into_owned(),
                        &self.span,
                    )?);
                } else {
                    break;
                }
            }
        }
        Ok(cache.get(index).cloned())
    }
}

#[derive(Debug)]
pub struct TableArray {
    f: Val,
    w: Array,
    x: Array,
    span: Span,
}

impl PartialEq for TableArray {
    fn eq(&self, other: &Self) -> bool {
        self.f == other.f && self.w == other.w && self.x == other.x
    }
}

impl Eq for TableArray {}

impl TableArray {
    pub fn new(f: Val, w: Array, x: Array, span: Span) -> Self {
        TableArray { f, w, x, span }
    }
    pub fn len(&self) -> Option<usize> {
        self.w.len()
    }
    pub fn get(&self, index: usize) -> RuntimeResult<Option<Val>> {
        let val = if let Some(val) = self.w.get(index)? {
            val.into_owned()
        } else {
            return Ok(None);
        };
        Ok(Some(Val::Array(Array::Each(
            EachArray {
                zip: ZipForm::BinLeft(val, self.x.clone()),
                f: self.f.clone(),
                span: self.span.clone(),
            }
            .into(),
        ))))
    }
}
