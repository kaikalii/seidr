use std::{
    cell::{Ref, RefCell, RefMut},
    cmp::Ordering,
    collections::HashMap,
    fmt,
    mem::swap,
    rc::Rc,
};

use crate::{
    lex::{Ident, ParamPlace},
    value::Val,
};

#[derive(Clone)]
pub struct Runtime {
    scope: Rc<RefCell<Scope>>,
}

impl Default for Runtime {
    fn default() -> Self {
        Runtime {
            scope: Default::default(),
        }
    }
}

impl Runtime {
    pub fn push(&self) -> Self {
        let rt = self.clone();
        let mut new = Scope::default();
        let mut scope = rt.scope.borrow_mut();
        swap(&mut new, &mut scope);
        scope.parent = Some(Rc::new(new));
        drop(scope);
        rt
    }
    pub fn bind_param(&self, place: ParamPlace, val: Val) {
        let mut scope = self.scope.borrow_mut();
        match place {
            ParamPlace::X => scope.params.x = Some(val),
            ParamPlace::W => scope.params.w = Some(val),
            ParamPlace::F => scope.params.f = Some(val),
            ParamPlace::G => scope.params.g = Some(val),
        }
    }
    pub fn get_param(&self, place: ParamPlace) -> Option<Val> {
        let scope = self.scope.borrow();
        match place {
            ParamPlace::X => scope.params.x.clone(),
            ParamPlace::W => scope.params.w.clone(),
            ParamPlace::F => scope.params.f.clone(),
            ParamPlace::G => scope.params.g.clone(),
        }
    }
    pub fn bind(&self, name: Ident, val: Val) {
        self.scope.borrow_mut().bindings.insert(name, val.into());
    }
    pub fn get(&self, name: &Ident) -> Option<Val> {
        self.scope.borrow().get(name).map(|val| val.clone())
    }
    pub fn get_mut<F, R>(&self, name: &Ident, f: F) -> Option<R>
    where
        F: FnOnce(&mut Val) -> R,
    {
        self.scope.borrow().get_mut(name).map(|mut val| f(&mut val))
    }
}

impl fmt::Debug for Runtime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<runtime>")
    }
}

impl PartialEq for Runtime {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl Eq for Runtime {}

impl PartialOrd for Runtime {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Runtime {
    fn cmp(&self, _: &Self) -> Ordering {
        Ordering::Equal
    }
}

#[derive(Default)]
pub struct Params {
    x: Option<Val>,
    w: Option<Val>,
    f: Option<Val>,
    g: Option<Val>,
}

#[derive(Default)]
struct Scope {
    parent: Option<Rc<Self>>,
    params: Params,
    bindings: HashMap<Ident, RefCell<Val>>,
}

impl Scope {
    fn get(&self, name: &Ident) -> Option<Ref<Val>> {
        if let Some(val) = self.bindings.get(name) {
            Some(val.borrow())
        } else if let Some(parent) = &self.parent {
            parent.get(name)
        } else {
            None
        }
    }
    fn get_mut(&self, name: &Ident) -> Option<RefMut<Val>> {
        if let Some(val) = self.bindings.get(name) {
            Some(val.borrow_mut())
        } else if let Some(parent) = &self.parent {
            parent.get_mut(name)
        } else {
            None
        }
    }
}
