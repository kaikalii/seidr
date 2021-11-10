//! Types for and conversion into the Concrete Walkable Tree

use std::{collections::HashSet, rc::Rc};

use crate::{
    ast::*,
    error::{Problem, SpannedCompileWarning},
    lex::{Ident, Span},
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
        }
    }
}

impl ToValNode for ExprItem {
    fn to_val(&self, builder: &mut TreeBuilder) -> ValNode {
        todo!()
    }
}
