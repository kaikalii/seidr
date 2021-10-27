use std::{fmt, ops::*};

use crate::{
    ast::*,
    error::{CompileErrorKind, CompileResult, Problem},
    lex::Span,
    num::Num,
    op::{Op, Visit},
    types::{ArrayType, AtomType, Type},
    value::{Array, Atom, Value},
};

#[derive(Debug, PartialEq, Eq)]
pub enum Const {
    Type(Type),
    Value(Value),
}

impl Const {
    pub fn ty(&self) -> Type {
        match self {
            Const::Type(ty) => ty.clone(),
            Const::Value(val) => val.ty(),
        }
    }
}

impl fmt::Display for Const {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Const::Type(ty) => ty.fmt(f),
            Const::Value(val) => val.fmt(f),
        }
    }
}

impl From<Type> for Const {
    fn from(ty: Type) -> Self {
        Const::Type(ty)
    }
}

impl From<ArrayType> for Const {
    fn from(at: ArrayType) -> Self {
        Const::Type(at.into())
    }
}

impl From<AtomType> for Const {
    fn from(at: AtomType) -> Self {
        Const::Type(at.into())
    }
}

impl From<Value> for Const {
    fn from(val: Value) -> Self {
        Const::Value(val)
    }
}

impl From<Atom> for Const {
    fn from(atom: Atom) -> Self {
        Const::Value(atom.into())
    }
}

impl From<Num> for Const {
    fn from(num: Num) -> Self {
        Const::Value(num.into())
    }
}

impl From<Array> for Const {
    fn from(arr: Array) -> Self {
        Const::Value(arr.into())
    }
}

pub struct Evaler {
    span: Span,
}

impl Default for Evaler {
    fn default() -> Self {
        Evaler { span: Span::dud() }
    }
}

impl Evaler {
    pub fn items(&mut self, items: Vec<Item>) -> CompileResult {
        for item in items {
            self.item(item)?;
        }
        Ok(())
    }
    pub fn item(&mut self, item: Item) -> CompileResult {
        match item {
            Item::Expr(expr) => {
                let ty = self.expr(expr)?;
                println!("{}", ty);
            }
        }
        Ok(())
    }
    fn expr(&mut self, expr: Expr) -> CompileResult<Const> {
        match expr {
            Expr::Ident(..) => todo!(),
            Expr::Num(num, _) => Ok(num.into()),
            Expr::Un(expr) => {
                let inner = self.expr(expr.inner)?;
                expr.op.visit_un(inner, self)
            }
            Expr::Bin(expr) => {
                let left = self.expr(expr.left)?;
                let right = self.expr(expr.right)?;
                expr.op.visit_bin(left, right, self)
            }
        }
    }
}

impl Visit<Evaler> for Op {
    type Input = Const;
    type Output = Const;
    type Error = Problem;
    fn visit_bin(
        &self,
        left: Self::Input,
        right: Self::Input,
        state: &mut Evaler,
    ) -> Result<Self::Output, Self::Error> {
        match self {
            Op::Add => bin_math(*self, left, right, &state.span, Atom::add),
            Op::Sub => bin_math(*self, left, right, &state.span, Atom::sub),
            Op::Mul => bin_math(*self, left, right, &state.span, Atom::mul),
            Op::Div => bin_math(*self, left, right, &state.span, Atom::div),
            op => todo!("{}", op),
        }
    }
    fn visit_un(
        &self,
        inner: Self::Input,
        state: &mut Evaler,
    ) -> Result<Self::Output, Self::Error> {
        todo!()
    }
}

fn bin_math(
    op: Op,
    left: Const,
    right: Const,
    span: &Span,
    f: fn(Atom, Atom, &Span) -> CompileResult<Atom>,
) -> CompileResult<Const> {
    Ok(match (left, right) {
        (Const::Value(a), Const::Value(b)) => bin_math_value(op, a, b, span, f)?.into(),
        (left, right) => {
            return Err(CompileErrorKind::IncompatibleBinTypes(op, left, right).at(span.clone()))
        }
    })
}

fn bin_math_value(
    op: Op,
    left: Value,
    right: Value,
    span: &Span,
    f: fn(Atom, Atom, &Span) -> CompileResult<Atom>,
) -> CompileResult<Value> {
    Ok(match (left, right) {
        (Value::Atom(a), Value::Atom(b)) => Value::Atom(f(a, b, span)?),
        (Value::Atom(a), Value::Array(b)) => Value::Array(Array::from_iter(
            b.iter()
                .map(|b| bin_math_value(op, Value::Atom(a), b, span, f)),
        )?),
        (left, right) => {
            return Err(
                CompileErrorKind::IncompatibleBinTypes(op, left.into(), right.into())
                    .at(span.clone()),
            )
        }
    })
}
