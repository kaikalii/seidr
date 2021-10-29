use crate::{ast::OpTreeExpr, error::RuntimeResult, value::Val};

#[derive(Default)]
pub struct Runtime {}

impl Runtime {}

trait Eval {
    fn eval(self, rt: &mut Runtime) -> RuntimeResult<Val>;
}
