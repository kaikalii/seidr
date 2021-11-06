//! Types for and conversion into the Concrete Walkable Tree

use std::rc::Rc;

use crate::{
    ast::*,
    error::{Problem, SpannedCompileWarning},
    lex::Span,
    value::Val,
};

#[derive(Clone)]
pub enum ValNode {
    Val(Val),
    Un(Rc<UnValNode>),
    Bin(Rc<BinValNode>),
    Array(Rc<[Self]>),
}

pub struct UnValNode {
    pub op: ValNode,
    pub span: Span,
    pub x: ValNode,
}

pub struct BinValNode {
    pub op: ValNode,
    pub span: Span,
    pub w: ValNode,
    pub x: ValNode,
}

impl From<UnValNode> for ValNode {
    fn from(un: UnValNode) -> Self {
        ValNode::Un(un.into())
    }
}

impl From<BinValNode> for ValNode {
    fn from(bin: BinValNode) -> Self {
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

impl ToValNode for ExprItem {
    fn to_val(&self, builder: &mut TreeBuilder) -> ValNode {
        self.expr.to_val(builder)
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

impl ToValNode for OpExpr {
    fn to_val(&self, builder: &mut TreeBuilder) -> ValNode {
        match self {
            OpExpr::Val(expr) => expr.to_val(builder),
            OpExpr::Op(op) => todo!("{:?}", op),
            OpExpr::Un(expr) => expr.to_val(builder),
            OpExpr::Bin(expr) => expr.to_val(builder),
            OpExpr::Atop(expr) => expr.to_val(builder),
            OpExpr::Fork(expr) => expr.to_val(builder),
        }
    }
}

impl ToValNode for UnOpExpr {
    fn to_val(&self, builder: &mut TreeBuilder) -> ValNode {
        let op = self.op.to_val(builder);
        let x = self.x.to_val(builder);
        UnValNode {
            op,
            span: self.op.span().clone(),
            x,
        }
        .into()
    }
}

impl ToValNode for BinOpExpr {
    fn to_val(&self, builder: &mut TreeBuilder) -> ValNode {
        let op = self.op.to_val(builder);
        let x = self.x.to_val(builder);
        let w = self.w.to_val(builder);
        BinValNode {
            op,
            span: self.op.span().clone(),
            w,
            x,
        }
        .into()
    }
}

impl ToValNode for AtopExpr {
    fn to_val(&self, builder: &mut TreeBuilder) -> ValNode {
        todo!("{:?}", self)
    }
}

impl ToValNode for ForkExpr {
    fn to_val(&self, builder: &mut TreeBuilder) -> ValNode {
        todo!("{:?}", self)
    }
}

impl ToValNode for ModExpr {
    fn to_val(&self, builder: &mut TreeBuilder) -> ValNode {
        match self {
            ModExpr::Op(op) => ValNode::Val(op.data.into()),
            ModExpr::Parened(expr) => expr.to_val(builder),
            ModExpr::Un(expr) => todo!(),
            ModExpr::Bin(expr) => todo!(),
        }
    }
}
