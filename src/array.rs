use std::{cmp::Ordering, fmt};

use crate::value::Val;

#[derive(Clone)]
pub enum Array {}

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
        todo!()
    }
}
