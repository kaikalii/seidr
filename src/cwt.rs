//! Types for and conversion into the Concrete Walkable Tree

use std::{fmt, rc::Rc};

use crate::{
    array::Array,
    ast::*,
    error::{CompileResult, Problem, SpannedCompileWarning, WarnedCompileResult},
    lex::Sp,
    op::Op,
    value::Val,
};

#[derive(Clone)]
pub enum ValNode {
    Val(Val),
    Un(Rc<UnVal>),
    Bin(Rc<BinVal>),
    Array(Rc<[Self]>),
}

pub type UnVal = Un<Sp<ValNode>, ValNode>;
pub type BinVal = Bin<Sp<ValNode>, ValNode, ValNode>;

impl From<UnVal> for ValNode {
    fn from(un: UnVal) -> Self {
        ValNode::Un(un.into())
    }
}

impl From<BinVal> for ValNode {
    fn from(bin: BinVal) -> Self {
        ValNode::Bin(bin.into())
    }
}

impl<T> From<T> for ValNode
where
    T: Into<Val>,
{
    fn from(val: T) -> Self {
        ValNode::Val(val.into())
    }
}

impl<T> FromIterator<T> for ValNode
where
    Val: FromIterator<T>,
{
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        ValNode::Val(Val::from_iter(iter))
    }
}

impl FromIterator<ValNode> for ValNode {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = ValNode>,
    {
        ValNode::Array(iter.into_iter().collect())
    }
}

impl fmt::Debug for ValNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValNode::Val(val) => val.fmt(f),
            ValNode::Un(un) => un.fmt(f),
            ValNode::Bin(bin) => bin.fmt(f),
            ValNode::Array(arr) => f.debug_list().entries(arr.iter()).finish(),
        }
    }
}

#[derive(Default)]
pub struct TreeBuilder {
    problems: Vec<Problem>,
}

pub type TreeBuildResult = Result<(ValNode, Vec<SpannedCompileWarning>), Vec<Problem>>;

impl TreeBuilder {
    pub fn build<V>(&mut self, node: &V) -> TreeBuildResult
    where
        V: ToValNode + ?Sized,
    {
        let node = node.to_val(self);
        let problems: Vec<Problem> = self.problems.drain(..).collect();
        if problems.iter().any(Problem::prevents_compilation) {
            Err(problems)
        } else {
            Ok((
                node,
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

pub trait ToValNode {
    fn to_val(&self, builder: &mut TreeBuilder) -> ValNode;
    fn build_val_tree(&self) -> TreeBuildResult {
        TreeBuilder::default().build(self)
    }
}

impl ToValNode for ValExpr {
    fn to_val(&self, builder: &mut TreeBuilder) -> ValNode {
        match self {
            ValExpr::Num(num) => (**num).into(),
            ValExpr::Char(c) => (**c).into(),
            ValExpr::String(string) => string.chars().collect(),
            ValExpr::Array(expr) => expr.items.iter().map(|expr| expr.to_val(builder)).collect(),
            ValExpr::Parened(expr) => expr.to_val(builder),
        }
    }
}

impl ToValNode for OpTreeExpr {
    fn to_val(&self, builder: &mut TreeBuilder) -> ValNode {
        match self {
            OpTreeExpr::Val(expr) => expr.to_val(builder),
            OpTreeExpr::Un(expr) => expr.to_val(builder),
            OpTreeExpr::Bin(expr) => expr.to_val(builder),
        }
    }
}

impl<X> ToValNode for Un<OpExpr, X>
where
    X: ToValNode,
{
    fn to_val(&self, builder: &mut TreeBuilder) -> ValNode {
        let op = self.op.span().clone().sp(self.op.to_val(builder));
        let x = self.x.to_val(builder);
        Un { op, x }.into()
    }
}

impl<W, X> ToValNode for Bin<OpExpr, W, X>
where
    W: ToValNode,
    X: ToValNode,
{
    fn to_val(&self, builder: &mut TreeBuilder) -> ValNode {
        let op = self.op.span().clone().sp(self.op.to_val(builder));
        let x = self.x.to_val(builder);
        let w = self.w.to_val(builder);
        Bin { op, w, x }.into()
    }
}

impl ToValNode for OpExpr {
    fn to_val(&self, builder: &mut TreeBuilder) -> ValNode {
        match self {
            OpExpr::Op(op) => (**op).into(),
        }
    }
}
