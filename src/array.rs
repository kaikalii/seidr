use std::{
    borrow::Cow,
    cell::{Cell, RefCell},
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet, HashMap},
    iter,
    rc::Rc,
};

use crate::{
    error::RuntimeResult,
    eval::{replicator_num, rt_error},
    format::{Format, Formatter},
    lex::Span,
    num::Num,
    pervade::LazyPervade,
    rcview::{RcView, RcViewIntoIter},
    runtime::Runtime,
    value::{Atom, Val},
};

type Items = RcView<Val>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Array {
    Concrete(Items),
    AsciiString(Rc<str>),
    Cached(Rc<CachedArray>),
    Reverse(Box<Self>),
    Range(Num),
    JoinTo(Box<Self>, Box<Self>),
    Pervaded(Box<LazyPervade>),
    Take(Box<Self>, i64),
    Drop(Box<Self>, i64),
    Each(Box<LazyEach>),
    Select(Box<LazySelect>),
    Windows(Box<Self>, usize),
    Chunks(Box<Self>, usize),
    Replicate(Rc<LazyReplicate>),
    Deduplicate(Rc<LazyDeduplicate>),
    Scan(Rc<LazyScan>),
    Table(Rc<LazyTable>),
    Classify(Rc<LazyClassify>),
}

fn _array_size() {
    use std::mem::transmute;
    let _: [u8; 8] = unsafe { transmute(Box::new(0)) };
    let _: [u8; 8] = unsafe { transmute(Rc::new(0)) };
    let _: [u8; 32] = unsafe { transmute(RcView::new(Some(1))) };
    let _: [u8; 40] = unsafe { transmute(Array::string("")) };
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
    pub fn empty() -> Self {
        Array::concrete(<[Val; 0]>::default())
    }
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
    pub fn bounded(&self) -> Cow<Self> {
        if self.len().is_none() {
            Cow::Owned(Array::Take(self.clone().into(), 5))
        } else {
            Cow::Borrowed(self)
        }
    }
    pub fn len(&self) -> Option<usize> {
        Some(match self {
            Array::Concrete(items) => items.len(),
            Array::AsciiString(s) => s.len(),
            Array::Cached(arr) => arr.len()?,
            Array::Reverse(arr) => arr.len()?,
            Array::Range(n) => {
                if n.is_infinite() {
                    return None;
                } else {
                    i64::from(*n) as usize
                }
            }
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
            Array::Replicate(rep) => rep.len()?,
            Array::Scan(scan) => scan.len()?,
            Array::Table(table) => table.len()?,
            Array::Classify(_) => return None,
            Array::Deduplicate(_) => return None,
        })
    }
    pub fn get(&self, index: usize) -> RuntimeResult<Option<Cow<Val>>> {
        Ok(match self {
            Array::Concrete(items) => items.get(index).map(Cow::Borrowed),
            Array::AsciiString(s) => s
                .as_bytes()
                .get(index)
                .copied()
                .map(char::from)
                .map(Val::from)
                .map(Cow::Owned),
            Array::Cached(arr) => arr.get(index)?.map(Cow::Owned),
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
                    |x| each.rt.eval_un(each.f.clone(), x, &each.span),
                    |w, x| each.rt.eval_bin(each.f.clone(), w, x, &each.span),
                )?
                .map(Cow::Owned),
            Array::Select(sel) => {
                let w = if let Some(w) = sel.indices.get(index)? {
                    w.into_owned()
                } else {
                    return Ok(None);
                };
                Some(Cow::Owned(sel.rt.index_array(w, &sel.array, &sel.span)?))
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
            Array::Classify(class) => class.get(index)?.map(Cow::Owned),
            Array::Deduplicate(dedup) => dedup.get(index)?.map(Cow::Owned),
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
    pub fn limited_depth(&self) -> RuntimeResult<usize> {
        let of_items = match self {
            Array::Range(_) => 0,
            arr => arr
                .bounded()
                .iter()
                .fold(Ok(0), |acc, item| -> RuntimeResult<usize> {
                    Ok(acc?.max(item?.limited_depth()?))
                })?,
        };
        Ok(1 + of_items)
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
        f.array(self)
    }
}

impl From<LazyPervade> for Array {
    fn from(pa: LazyPervade) -> Self {
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
        let mut cache = self.cache.borrow_mut();
        Ok(if let Some(val) = cache.get(&index) {
            Some(val.clone())
        } else if let Some(val) = self.arr.get(index)? {
            let val = val.into_owned();
            cache.insert(index, val.clone());
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
pub struct LazyEach {
    pub zip: ZipForm,
    pub f: Val,
    pub span: Span,
    pub rt: Runtime,
}

impl PartialEq for LazyEach {
    fn eq(&self, other: &Self) -> bool {
        self.f == other.f && self.zip == other.zip
    }
}

impl Eq for LazyEach {}

#[derive(Debug, Clone)]
pub struct LazySelect {
    pub indices: Array,
    pub array: Array,
    pub span: Span,
    pub rt: Runtime,
}

impl PartialEq for LazySelect {
    fn eq(&self, other: &Self) -> bool {
        self.indices == other.indices && self.array == other.array
    }
}

impl Eq for LazySelect {}

#[derive(Debug, PartialEq, Eq)]
pub enum LazyReplicate {
    Repeat { n: Num, val: Val },
    Num { n: Num, array: Array },
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

impl LazyReplicate {
    pub fn repeat(n: Num, val: Val) -> Self {
        LazyReplicate::Repeat { n, val }
    }
    pub fn num(n: Num, array: Array) -> Self {
        LazyReplicate::Num { n, array }
    }
    pub fn counts(counts: Array, array: Array, span: Span) -> Self {
        LazyReplicate::Counts(
            CountsReplicateArray {
                counts: counts.into_iter().into(),
                array: array.into_iter().into(),
                cache: Vec::new().into(),
                span,
            }
            .into(),
        )
    }
    pub fn len(&self) -> Option<usize> {
        match self {
            LazyReplicate::Repeat { n, .. } => {
                if n.is_infinite() {
                    None
                } else {
                    Some(i64::from(*n) as usize)
                }
            }
            LazyReplicate::Counts(_) => None,
            LazyReplicate::Num { n, array } => {
                if n.is_infinite() {
                    None
                } else {
                    Some(i64::from(*n * Num::from(array.len()?)) as usize)
                }
            }
        }
    }
    pub fn get(&self, index: usize) -> RuntimeResult<Option<Cow<Val>>> {
        Ok(match self {
            LazyReplicate::Repeat { n, val } => {
                if n > &Num::from(index) {
                    Some(Cow::Borrowed(val))
                } else {
                    None
                }
            }
            LazyReplicate::Counts(cra) => {
                let mut cache = cra.cache.borrow_mut();
                while cache.len() <= index {
                    let count = cra.counts.borrow_mut().next().transpose()?;
                    let val = cra.array.borrow_mut().next().transpose()?;
                    if let Some((count, val)) = count.zip(val) {
                        let n = replicator_num(count, &cra.span)?;
                        let mut i: i64 = 0;
                        while n > i {
                            cache.push(val.clone());
                            i += 0;
                        }
                    } else {
                        break;
                    }
                }
                cache.get(index).cloned().map(Cow::Owned)
            }
            LazyReplicate::Num { n, array } if n.is_infinite() => array.get(0)?,
            LazyReplicate::Num { n, array } => array.get(index / i64::from(*n) as usize)?,
        })
    }
}

#[derive(Debug)]
pub struct LazyScan {
    f: Val,
    array: Array,
    init: Option<Val>,
    cache: RefCell<Vec<Val>>,
    span: Span,
    rt: Runtime,
}

impl PartialEq for LazyScan {
    fn eq(&self, other: &Self) -> bool {
        self.f == other.f && self.array == other.array && self.init == other.init
    }
}

impl Eq for LazyScan {}

impl LazyScan {
    pub fn new(f: Val, array: Array, init: Option<Val>, span: Span, rt: Runtime) -> Self {
        LazyScan {
            f,
            array,
            init,
            cache: Default::default(),
            span,
            rt,
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
                        cache.push(self.rt.eval_bin(
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
                let w = cache.get(i - 1).unwrap().clone();
                let x = self.array.get(i)?;
                if let Some(x) = x {
                    cache.push(
                        self.rt
                            .eval_bin(self.f.clone(), w, x.into_owned(), &self.span)?,
                    );
                } else {
                    break;
                }
            }
        }
        Ok(cache.get(index).cloned())
    }
}

#[derive(Debug)]
pub struct LazyTable {
    f: Val,
    w: Array,
    x: Array,
    span: Span,
    rt: Runtime,
}

impl PartialEq for LazyTable {
    fn eq(&self, other: &Self) -> bool {
        self.f == other.f && self.w == other.w && self.x == other.x
    }
}

impl Eq for LazyTable {}

impl LazyTable {
    pub fn new(f: Val, w: Array, x: Array, span: Span, rt: Runtime) -> Self {
        LazyTable { f, w, x, span, rt }
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
            LazyEach {
                zip: ZipForm::BinLeft(val, self.x.clone()),
                f: self.f.clone(),
                span: self.span.clone(),
                rt: self.rt.clone(),
            }
            .into(),
        ))))
    }
}

#[derive(Debug)]
pub struct LazyClassify {
    arr: Array,
    next_index: Cell<usize>,
    resolved: Cell<usize>,
    indices: RefCell<BTreeMap<Val, usize>>,
}

impl LazyClassify {
    pub fn new(arr: Array) -> Self {
        LazyClassify {
            arr,
            next_index: Cell::new(0),
            resolved: Cell::new(0),
            indices: Default::default(),
        }
    }
    pub fn get(&self, index: usize) -> RuntimeResult<Option<Val>> {
        while self.resolved.get() <= index {
            let resolved = self.resolved.get();
            let val = self.arr.get(resolved)?;
            if let Some(val) = val {
                let mut indices = self.indices.borrow_mut();
                if !indices.contains_key(&val) {
                    let next_index = self.next_index.get();
                    indices.insert(val.into_owned(), next_index);
                    self.next_index.set(next_index + 1);
                }
                self.resolved.set(resolved + 1);
            } else {
                return Ok(None);
            }
        }
        Ok(self.arr.get(index)?.map(|val| {
            (*self
                .indices
                .borrow()
                .get(&val)
                .expect("No index for classified value"))
            .into()
        }))
    }
}

impl PartialEq for LazyClassify {
    fn eq(&self, other: &Self) -> bool {
        self.arr == other.arr
    }
}

impl Eq for LazyClassify {}

impl PartialOrd for LazyClassify {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for LazyClassify {
    fn cmp(&self, other: &Self) -> Ordering {
        self.arr.cmp(&other.arr)
    }
}

#[derive(Debug)]
pub struct LazyDeduplicate {
    arr: Array,
    resolved: Cell<usize>,
    cache: RefCell<Vec<Val>>,
    seen: RefCell<BTreeSet<Val>>,
}

impl LazyDeduplicate {
    pub fn new(arr: Array) -> Self {
        LazyDeduplicate {
            arr,
            resolved: Cell::new(0),
            cache: Default::default(),
            seen: Default::default(),
        }
    }
    pub fn get(&self, index: usize) -> RuntimeResult<Option<Val>> {
        let mut cache = self.cache.borrow_mut();
        let mut seen = self.seen.borrow_mut();
        while cache.len() <= index {
            let resolved = self.resolved.get();
            if let Some(val) = self.arr.get(resolved)? {
                if !seen.contains(&val) {
                    let val = val.into_owned();
                    seen.insert(val.clone());
                    cache.push(val);
                }
                self.resolved.set(resolved + 1);
            } else {
                break;
            }
        }
        Ok(cache.get(index).cloned())
    }
}

impl PartialEq for LazyDeduplicate {
    fn eq(&self, other: &Self) -> bool {
        self.arr == other.arr
    }
}

impl Eq for LazyDeduplicate {}

impl PartialOrd for LazyDeduplicate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for LazyDeduplicate {
    fn cmp(&self, other: &Self) -> Ordering {
        self.arr.cmp(&other.arr)
    }
}
