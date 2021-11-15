//! Types for and conversion into the Concrete Walkable Tree

use std::{collections::HashSet, rc::Rc};

use crate::{
    array::Array,
    ast::*,
    error::{CompileError, Problem, SpannedCompileWarning},
    function::*,
    lex::{Ident, Param, ParamPlace, Span},
    op::AssignOp,
    rcview::RcView,
    value::{Atom, Val},
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ValNode {
    Param(Param),
    Ident(Ident),
    Val(Val),
    Un(Rc<UnValNode>),
    Bin(Rc<BinValNode>),
    Array(Rc<[Self]>),
    Assign(Rc<AssignValNode>),
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct UnValNode {
    pub op: ValNode,
    pub inner: ValNode,
    pub span: Span,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BinValNode {
    pub op: ValNode,
    pub left: ValNode,
    pub right: ValNode,
    pub span: Span,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct AssignValNode {
    pub name: Ident,
    pub op: AssignOp,
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

impl ToValNode for ExprItem {
    fn to_val(&self, builder: &mut TreeBuilder) -> ValNode {
        if let Some(param) = self.expr.max_param() {
            builder.error(CompileError::ParameterOutsideFunction.at(param.span.clone()));
        }
        self.expr.to_val(builder)
    }
}

impl ToValNode for Expr {
    fn to_val(&self, builder: &mut TreeBuilder) -> ValNode {
        match self {
            Expr::Param(param) => ValNode::Param(param.data),
            Expr::Op(op) => (**op).into(),
            Expr::UnMod(m) => UnMod::from(**m).into(),
            Expr::BinMod(m) => BinMod::from(**m).into(),
            Expr::Ident(ident) => {
                if !builder.lookup(ident) {
                    builder.error(
                        CompileError::UnknownBinding(ident.data.clone()).at(ident.span.clone()),
                    )
                }
                ValNode::Ident(ident.data.clone())
            }
            Expr::Num(n) => (**n).into(),
            Expr::Char(c) => (**c).into(),
            Expr::String(s) => Array::string(s.data.clone()).into(),
            Expr::Array(arr) => ValNode::Array(
                arr.items
                    .iter()
                    .map(|(item, _)| item.to_val(builder))
                    .collect(),
            ),
            Expr::Parened(expr) => expr.to_val(builder),
            Expr::Un(expr) => expr.to_val(builder),
            Expr::Bin(expr) => expr.to_val(builder),
            Expr::Assign(expr) => expr.to_val(builder),
            Expr::Function(func) => func.to_val(builder),
        }
    }
}

impl ToValNode for UnExpr {
    fn to_val(&self, builder: &mut TreeBuilder) -> ValNode {
        ValNode::Un(
            UnValNode {
                op: self.op.to_val(builder),
                inner: self.inner.to_val(builder),
                span: self.op.span().clone(),
            }
            .into(),
        )
    }
}

impl ToValNode for BinExpr {
    fn to_val(&self, builder: &mut TreeBuilder) -> ValNode {
        ValNode::Bin(
            BinValNode {
                op: self.op.to_val(builder),
                left: self.left.to_val(builder),
                right: self.right.to_val(builder),
                span: self.op.span().clone(),
            }
            .into(),
        )
    }
}

impl ToValNode for AssignExpr {
    fn to_val(&self, builder: &mut TreeBuilder) -> ValNode {
        if self.name.role() != self.body.role() {
            builder.error(
                CompileError::MismatchedRoles(self.name.clone(), self.body.role())
                    .at(self.span.clone()),
            );
        }
        builder.scope().bindings.insert(self.name.clone());
        ValNode::Assign(
            AssignValNode {
                name: self.name.clone(),
                op: self.op,
                body: self.body.to_val(builder),
            }
            .into(),
        )
    }
}

impl ToValNode for FunctionLiteral {
    fn to_val(&self, builder: &mut TreeBuilder) -> ValNode {
        let nodes: RcView<ValNode> = self
            .expressions()
            .map(|expr| expr.to_val(builder))
            .collect();
        match self.max_param().map(|param| param.place) {
            Some(ParamPlace::W | ParamPlace::X) | None => Function::Nodes(nodes).into(),
            Some(ParamPlace::F) => Atom::UnMod(UnMod::Nodes(nodes)).into(),
            Some(ParamPlace::G) => Atom::BinMod(BinMod::Nodes(nodes)).into(),
        }
    }
}
