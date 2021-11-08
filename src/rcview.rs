use std::{
    borrow::Borrow,
    fmt,
    ops::{Bound, Deref, RangeBounds},
    rc::Rc,
};

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

impl<T> fmt::Debug for RcViewIntoIter<T>
where
    T: Clone,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "iter")
    }
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

impl<T> PartialEq for RcView<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.as_ref() == other.as_ref()
    }
}

impl<T> Eq for RcView<T> where T: Eq {}

impl<T> fmt::Debug for RcView<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
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
