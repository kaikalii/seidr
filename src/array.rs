use std::{cmp::Ordering, fmt, rc::Rc};

use crate::{
    ast::{Bin, Un},
    error::{RuntimeError, RuntimeResult},
    lex::{Sp, Span},
    op::Pervasive,
    value::{Atom, Val},
};

pub type Shape = Rc<[usize]>;

#[derive(Clone)]
pub struct Array {
    shape: Shape,
    items: Rc<[Val]>,
}

impl Array {
    pub fn shape(&self) -> &[usize] {
        &self.shape
    }
    pub fn len(&self) -> usize {
        self.shape[0]
    }
    pub fn rank(&self) -> usize {
        self.shape.len()
    }
    pub fn iter(&self) -> Box<dyn Iterator<Item = Val> + '_> {
        if self.shape.len() == 1 {
            Box::new(self.items.iter().cloned())
        } else {
            let sub_shape: Shape = self.shape.iter().skip(1).copied().collect();
            let item_size: usize = sub_shape.iter().product();
            Box::new(self.items.chunks(item_size).map(move |chunk| {
                Val::from(Array {
                    shape: sub_shape.clone(),
                    items: Rc::from(chunk),
                })
            }))
        }
    }
    pub fn pervade<F, V>(&self, f: F) -> RuntimeResult<Self>
    where
        F: Fn(Val) -> RuntimeResult<V>,
        V: Into<Val>,
    {
        let mut items = Vec::new();
        for item in self.items.iter().cloned() {
            items.push(f(item)?.into());
        }
        Ok(Array {
            shape: self.shape.clone(),
            items: items.into(),
        })
    }
    pub fn pervade_with<F, V>(&self, other: &Self, span: &Span, f: F) -> RuntimeResult<Self>
    where
        F: Fn(Val, Val) -> RuntimeResult<V>,
        V: Into<Val>,
    {
        if self.shape != other.shape {
            return Err(RuntimeError::new("Array shapes do not match", span.clone()));
        }
        let mut items = Vec::new();
        for (a, b) in self.items.iter().cloned().zip(other.items.iter().cloned()) {
            items.push(f(a, b)?.into());
        }
        Ok(Array {
            shape: self.shape.clone(),
            items: items.into(),
        })
    }
    pub fn product<F, V>(&self, other: &Self, f: F) -> Self
    where
        F: Fn(Val, Val) -> V,
        V: Into<Val>,
    {
        let mut items = Vec::new();
        for a in self.iter() {
            for b in other.iter() {
                let v = f(a.clone(), b).into();
                items.push(v);
            }
        }
        Array {
            shape: self
                .shape
                .iter()
                .copied()
                .chain(other.shape.iter().copied())
                .collect(),
            items: items.into(),
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
        if self.rank() == 1
            && self
                .items
                .iter()
                .all(|val| matches!(val, Val::Atom(Atom::Char(_))))
        {
            let mut s = String::new();
            for val in self.items.iter() {
                if let Val::Atom(Atom::Char(c)) = val {
                    s.push(*c);
                }
            }
            write!(f, "{:?}", s)
        } else if self.rank() == 1 && self.items.iter().all(|val| matches!(val, Val::Atom(_))) {
            for (i, val) in self.items.iter().enumerate() {
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

impl<V> FromIterator<V> for Array
where
    V: Into<Val>,
{
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = V>,
    {
        let items: Rc<[Val]> = iter.into_iter().map(Into::into).collect();
        Array {
            shape: Rc::new([items.len()]),
            items,
        }
    }
}
