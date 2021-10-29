use std::{fmt, rc::Rc};

use crate::value::Val;

#[derive(Clone)]
pub struct Array {
    mask: Mask,
    items: Rc<[Val]>,
}

#[derive(Clone)]
pub struct Mask {
    ty: MaskTy,
    shape: Vec<usize>,
}

#[derive(Clone)]
pub enum MaskTy {
    Identity,
}

impl Mask {
    pub fn len(&self) -> usize {
        self.shape[0]
    }
    pub fn rank(&self) -> usize {
        self.shape.len()
    }
}

impl FromIterator<Val> for Array {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = Val>,
    {
        let items: Rc<[Val]> = iter.into_iter().collect();
        Array {
            mask: Mask {
                shape: vec![items.len()],
                ty: MaskTy::Identity,
            },
            items,
        }
    }
}
