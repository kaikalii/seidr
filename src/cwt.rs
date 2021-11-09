//! Types for and conversion into the Concrete Walkable Tree

use std::rc::Rc;

use crate::{
    array::Array,
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
    Atop(Box<Self>, Box<Self>),
    Fork(Box<Self>, Box<Self>, Box<Self>),
}

pub struct UnValNode {
    pub op: ValNode,
    pub x: ValNode,
    pub span: Span,
}

pub struct BinValNode {
    pub op: ValNode,
    pub w: ValNode,
    pub x: ValNode,
    pub span: Span,
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
            ValExpr::String(string) => Array::string(string.data.clone()).into(),
            ValExpr::Array(expr) => expr
                .items
                .iter()
                .map(|(expr, _)| match expr {
                    ArrayItemExpr::Val(expr) => expr.to_val(builder),
                    ArrayItemExpr::Function(expr) => expr.to_val(builder),
                })
                .collect(),
            ValExpr::Parened(expr) => expr.to_val(builder),
            ValExpr::Mod(expr) => expr.to_val(builder),
        }
    }
}

impl ToValNode for OpExpr {
    fn to_val(&self, builder: &mut TreeBuilder) -> ValNode {
        match self {
            OpExpr::Val(expr) => expr.to_val(builder),
            OpExpr::Un(expr) => expr.to_val(builder),
            OpExpr::Bin(expr) => expr.to_val(builder),
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

impl ToValNode for ModExpr {
    fn to_val(&self, builder: &mut TreeBuilder) -> ValNode {
        match self {
            ModExpr::Op(op) => (**op).into(),
            ModExpr::Un(expr) => expr.to_val(builder),
            ModExpr::Bin(expr) => expr.to_val(builder),
            ModExpr::Parened(expr) => expr.to_val(builder),
        }
    }
}

impl ToValNode for UnModExpr {
    fn to_val(&self, builder: &mut TreeBuilder) -> ValNode {
        ValNode::Un(
            UnValNode {
                op: self.m.into(),
                x: self.f.to_val(builder),
                span: self.span.clone(),
            }
            .into(),
        )
    }
}

impl ToValNode for BinModExpr {
    fn to_val(&self, builder: &mut TreeBuilder) -> ValNode {
        ValNode::Bin(
            BinValNode {
                op: self.m.into(),
                w: self.f.to_val(builder),
                x: self.g.to_val(builder),
                span: self.span.clone(),
            }
            .into(),
        )
    }
}

impl ToValNode for TrainExpr {
    fn to_val(&self, builder: &mut TreeBuilder) -> ValNode {
        match self {
            TrainExpr::Single(expr) => expr.to_val(builder),
            TrainExpr::Atop(expr) => expr.to_val(builder),
            TrainExpr::Fork(expr) => expr.to_val(builder),
        }
    }
}

impl ToValNode for AtopExpr {
    fn to_val(&self, builder: &mut TreeBuilder) -> ValNode {
        ValNode::Atop(self.f.to_val(builder).into(), self.g.to_val(builder).into())
    }
}

impl ToValNode for ForkExpr {
    fn to_val(&self, builder: &mut TreeBuilder) -> ValNode {
        ValNode::Fork(
            self.left.to_val(builder).into(),
            self.center.to_val(builder).into(),
            self.right.to_val(builder).into(),
        )
    }
}
