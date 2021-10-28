use std::{fmt, rc::Rc};

use crate::atom::Atom;

#[derive(Clone)]
pub struct Shape {
    dims: Vec<usize>,
}

#[derive(Clone)]
pub struct Array {
    offset: usize,
    skip: usize,
    shape: Shape,
    items: Rc<[Atom]>,
}

impl Shape {
    pub fn len(&self) -> usize {
        self.dims[0]
    }
    pub fn rank(&self) -> usize {
        self.dims.len()
    }
}

impl Array {
    pub fn atoms(&self) -> Atoms {
        Atoms { i: 0, array: self }
    }
    pub fn pervasive<F>(&self, f: F) -> Array
    where
        F: Fn(&Atom) -> Atom,
    {
        Array {
            offset: 0,
            skip: 1,
            shape: self.shape.clone(),
            items: self.atoms().map(f).collect(),
        }
    }
}

struct Atoms<'a> {
    i: usize,
    array: &'a Array,
}

impl<'a> Iterator for Atoms<'a> {
    type Item = &'a Atom;
    fn next(&mut self) -> Option<Self::Item> {
        let i = self.i * self.array.skip + self.array.offset;
        self.i += 1;
        todo!()
    }
}
