//! Types for and conversion into the Concrete Walkable Tree

use std::rc::Rc;

use crate::{
    array::Array,
    ast::*,
    error::{CompileResult, Problem, SpannedCompileWarning, WarnedCompileResult},
    op::Op,
    value::Val,
};

#[derive(Default)]
pub struct TreeBuilder {
    problems: Vec<Problem>,
}

pub type TreeBuildResult = Result<(Val, Vec<SpannedCompileWarning>), Vec<Problem>>;

impl TreeBuilder {
    pub fn build<V>(&mut self, node: &V) -> TreeBuildResult
    where
        V: ToVal + ?Sized,
    {
        let val = node.to_val(self);
        let problems: Vec<Problem> = self.problems.drain(..).collect();
        if problems.iter().any(Problem::prevents_compilation) {
            Err(problems)
        } else {
            Ok((
                val,
                problems
                    .into_iter()
                    .filter_map(|p| {
                        if let Problem::Warning(w) = p {
                            Some(w)
                        } else {
                            None
                        }
                    })
                    .collect(),
            ))
        }
    }
    pub fn error<P>(&mut self, error: P)
    where
        P: Into<Problem>,
    {
        self.problems.push(error.into())
    }
}

pub trait ToVal {
    fn to_val(&self, builder: &mut TreeBuilder) -> Val;
    fn build_val_tree(&self) -> Result<(Val, Vec<SpannedCompileWarning>), Vec<Problem>> {
        TreeBuilder::default().build(self)
    }
}

impl ToVal for ValExpr {
    fn to_val(&self, builder: &mut TreeBuilder) -> Val {
        match self {
            ValExpr::Num(num) => Val::Num(**num),
            ValExpr::Char(c) => Val::Char(**c),
            ValExpr::String(string) => string.chars().collect(),
            ValExpr::Array(expr) => expr.items.iter().map(|expr| expr.to_val(builder)).collect(),
            ValExpr::Parened(expr) => expr.to_val(builder),
        }
    }
}

impl ToVal for OpTreeExpr {
    fn to_val(&self, builder: &mut TreeBuilder) -> Val {
        match self {
            OpTreeExpr::Val(expr) => expr.to_val(builder),
            OpTreeExpr::Un(expr) => expr.to_val(builder),
            OpTreeExpr::Bin(expr) => expr.to_val(builder),
        }
    }
}

impl<X> ToVal for Un<OpExpr, X>
where
    X: ToVal,
{
    fn to_val(&self, builder: &mut TreeBuilder) -> Val {
        let op = self.op.span().clone().sp(self.op.to_val(builder));
        let x = self.x.to_val(builder);
        Un { op, x }.into()
    }
}

impl<W, X> ToVal for Bin<OpExpr, W, X>
where
    W: ToVal,
    X: ToVal,
{
    fn to_val(&self, builder: &mut TreeBuilder) -> Val {
        let op = self.op.span().clone().sp(self.op.to_val(builder));
        let x = self.x.to_val(builder);
        let w = self.w.to_val(builder);
        Bin { op, w, x }.into()
    }
}

impl ToVal for OpExpr {
    fn to_val(&self, builder: &mut TreeBuilder) -> Val {
        match self {
            OpExpr::Op(op) => match **op {
                Op::Pervasive(per) => todo!(),
                Op::Rune(rune) => todo!(),
            },
        }
    }
}