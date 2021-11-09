//! Types for and conversion into the Concrete Walkable Tree

use std::{collections::HashSet, rc::Rc};

use crate::{
    array::Array,
    ast::*,
    error::{CompileError, Problem, SpannedCompileWarning},
    lex::{Ident, Sp, Span},
    op::AssignOp,
    value::Val,
};

#[derive(Clone)]
pub enum ValNode {
    Ident(Ident),
    Val(Val),
    Un(Rc<UnValNode>),
    Bin(Rc<BinValNode>),
    Array(Rc<[Self]>),
    Atop(Box<Self>, Box<Self>),
    Fork(Box<Self>, Box<Self>, Box<Self>),
    Assign(Rc<AssignValNode>),
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

pub struct AssignValNode {
    pub name: Ident,
    pub body: ValNode,
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

pub struct TreeBuilder {
    problems: Vec<Problem>,
    scopes: Vec<Scope>,
}

#[derive(Default)]
struct Scope {
    bindings: HashSet<Ident>,
}

pub type TreeBuildResult = Result<(ValNode, Vec<SpannedCompileWarning>), Vec<Problem>>;

impl Default for TreeBuilder {
    fn default() -> Self {
        TreeBuilder {
            problems: Vec::new(),
            scopes: vec![Scope::default()],
        }
    }
}

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
    fn scope(&mut self) -> &mut Scope {
        self.scopes.last_mut().expect("scopes is empty")
    }
    pub fn lookup(&self, name: &Ident) -> bool {
        self.scopes
            .iter()
            .rev()
            .any(|scope| scope.bindings.contains(name))
    }
}

pub trait ToValNode {
    fn to_val(&self, builder: &mut TreeBuilder) -> ValNode;
    fn build_val_tree(&self) -> TreeBuildResult {
        TreeBuilder::default().build(self)
    }
}

impl ToValNode for Item {
    fn to_val(&self, builder: &mut TreeBuilder) -> ValNode {
        match self {
            Item::Newline | Item::Comment(_) => todo!(),
            Item::Expr(expr) => expr.to_val(builder),
            Item::Function(expr) => expr.to_val(builder),
        }
    }
}

impl<T> ToValNode for ExprItem<T>
where
    T: ToValNode,
{
    fn to_val(&self, builder: &mut TreeBuilder) -> ValNode {
        self.expr.to_val(builder)
    }
}

impl ToValNode for ValExpr {
    fn to_val(&self, builder: &mut TreeBuilder) -> ValNode {
        match self {
            ValExpr::Ident(ident) => ident.to_val(builder),
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
            OpExpr::Assign(expr) => expr.to_val(builder),
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

impl ToValNode for Sp<Ident> {
    fn to_val(&self, builder: &mut TreeBuilder) -> ValNode {
        if !builder.lookup(self) {
            builder.error(CompileError::UnknownBinding(self.data.clone()).at(self.span.clone()));
        }
        ValNode::Ident(self.data.clone())
    }
}

impl<T> ToValNode for AssignExpr<T>
where
    T: ToValNode,
{
    fn to_val(&self, builder: &mut TreeBuilder) -> ValNode {
        match self.op {
            AssignOp::Assign => {
                builder.scope().bindings.insert(self.name.clone());
            }
            AssignOp::Reassign => {
                if !builder.lookup(&self.name) {
                    builder.error(
                        CompileError::UnknownBinding(self.name.clone()).at(self.span.clone()),
                    );
                }
            }
        }
        ValNode::Assign(
            AssignValNode {
                name: self.name.clone(),
                body: self.body.to_val(builder),
            }
            .into(),
        )
    }
}

impl ToValNode for ModExpr {
    fn to_val(&self, builder: &mut TreeBuilder) -> ValNode {
        match self {
            ModExpr::Ident(ident) => ident.to_val(builder),
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
            TrainExpr::Assign(expr) => expr.to_val(builder),
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
