use std::{cmp::Ordering, fmt, rc::Rc};

use crate::value::Val;

#[derive(Debug, Clone)]
pub struct Array {
    shape: Rc<[usize]>,
    items: Rc<[Val]>,
}

impl Array {
    pub fn len(&self) -> usize {
        self.shape[0]
    }
    pub fn rank(&self) -> usize {
        self.shape.len()
    }
    pub fn item_len(&self) -> usize {
        self.items.len()
    }
}

impl FromIterator<Val> for Array {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = Val>,
    {
        let items: Rc<[Val]> = iter.into_iter().collect();
        Array {
            shape: Rc::new([items.len()]),
            items,
        }
    }
}

impl PartialEq for Array {
    fn eq(&self, other: &Self) -> bool {
        self.items == other.items
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
        self.items.cmp(&self.items)
    }
}

impl fmt::Display for Array {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let columns = *self.shape.last().unwrap();
        let mut indices = vec![0usize; self.shape.len()];
        'l: for item in self.items.iter() {
            for &index in &indices {
                if index == 0 {
                    write!(f, "〈")?;
                }
            }
            write!(f, "{} ", item)?;
            for i in 0..indices.len() {
                if indices[i] == self.shape[i] - 1 {
                    write!(f, "〉")?;
                    indices[i] = 0;
                    if i == 0 {
                        break 'l;
                    } else {
                        indices[i - 1] += 1;
                    }
                }
            }
        }
        Ok(())
    }
}
