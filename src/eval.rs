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

impl From<Const> for Type {
    fn from(c: Const) -> Self {
        match c {
            Const::Value(val) => val.ty(),
            Const::Type(ty) => ty,
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

impl From<char> for Const {
    fn from(c: char) -> Self {
        Const::Value(c.into())
    }
}

impl From<Array> for Const {
    fn from(arr: Array) -> Self {
        Const::Value(arr.into())
    }
}

impl Const {
    fn from_iter<I>(iter: I) -> CompileResult<Self>
    where
        I: IntoIterator<Item = CompileResult<Const>>,
    {
        let mut consts: Vec<Const> = iter.into_iter().collect::<CompileResult<_>>()?;
        Ok(if consts.is_empty() {
            Array::List(Vec::new()).into()
        } else if consts.iter().all(|ty| matches!(ty, Const::Value(_))) {
            Value::Array(Array::from_iter(consts.into_iter().map(|ty| {
                Ok(if let Const::Value(val) = ty {
                    val
                } else {
                    unreachable!()
                })
            }))?)
            .into()
        } else {
            let mut types: Vec<Type> = consts.into_iter().map(Type::from).collect();
            let all_same = types.windows(2).all(|win| win[0] == win[1]);
            if all_same {
                let len = types.len();
                ArrayType::StaticHomo(types.pop().unwrap(), len)
            } else {
                ArrayType::StaticHetero(types)
            }
            .into()
        })
    }
}

pub struct Evaler {
    spans: Vec<Span>,
}

impl Default for Evaler {
    fn default() -> Self {
        Evaler { spans: Vec::new() }
    }
}

impl Evaler {
    fn span(&self) -> &Span {
        self.spans.last().unwrap()
    }
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
        self.spans.push(expr.span().clone());
        let res = match expr {
            Expr::Ident(..) => todo!(),
            Expr::Num(num, _) => Ok(num.into()),
            Expr::Char(c, _) => Ok(c.into()),
            Expr::String(s, _) => Ok(Array::String(s).into()),
            Expr::Array(expr) => {
                Const::from_iter(expr.items.into_iter().map(|expr| self.expr(expr)))
            }
            Expr::Un(expr) => {
                let inner = self.expr(expr.inner)?;
                expr.op.visit_un(inner, self)
            }
            Expr::Bin(expr) => {
                let left = self.expr(expr.left)?;
                let right = self.expr(expr.right)?;
                expr.op.visit_bin(left, right, self)
            }
        };
        self.spans.pop().unwrap();
        res
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
            Op::Add => pervasize_bin(*self, left, right, state.span(), Atom::add),
            Op::Sub => pervasize_bin(*self, left, right, state.span(), Atom::sub),
            Op::Mul => pervasize_bin(*self, left, right, state.span(), Atom::mul),
            Op::Div => pervasize_bin(*self, left, right, state.span(), Atom::div),
            Op::Equal => pervasize_bin(*self, left, right, state.span(), |a, b, _| {
                Ok((a == b).into())
            }),
            Op::NotEqual => pervasize_bin(*self, left, right, state.span(), |a, b, _| {
                Ok((a != b).into())
            }),
            Op::Less => pervasize_bin(*self, left, right, state.span(), |a, b, _| {
                Ok((a < b).into())
            }),
            Op::LessOrEqual => pervasize_bin(*self, left, right, state.span(), |a, b, _| {
                Ok((a <= b).into())
            }),
            Op::Greater => pervasize_bin(*self, left, right, state.span(), |a, b, _| {
                Ok((a > b).into())
            }),
            Op::GreaterOrEqual => pervasize_bin(*self, left, right, state.span(), |a, b, _| {
                Ok((a >= b).into())
            }),
            op => todo!("{}", op),
        }
    }
    fn visit_un(
        &self,
        inner: Self::Input,
        state: &mut Evaler,
    ) -> Result<Self::Output, Self::Error> {
        match self {
            Op::Sub => pervasive_un(*self, inner, state.span(), |atom, span| match atom {
                Atom::Num(n) => Ok(Atom::Num(-n)),
                Atom::Char(_) => {
                    Err(CompileErrorKind::IncompatibleUnType(Op::Sub, atom.into()).at(span.clone()))
                }
            }),
            op => todo!("{}", op),
        }
    }
}

fn pervasize_bin(
    op: Op,
    left: Const,
    right: Const,
    span: &Span,
    f: fn(Atom, Atom, &Span) -> CompileResult<Atom>,
) -> CompileResult<Const> {
    match (left, right) {
        (Const::Value(a), Const::Value(b)) => {
            pervasize_bin_value(op, a, b, span, f).map(Into::into)
        }
        (left, right) => {
            Err(CompileErrorKind::IncompatibleBinTypes(op, left, right).at(span.clone()))
        }
    }
}

fn pervasize_bin_value(
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
                .map(|b| pervasize_bin_value(op, Value::Atom(a), b, span, f)),
        )?),
        (Value::Array(a), Value::Atom(b)) => Value::Array(Array::from_iter(
            a.iter()
                .map(|a| pervasize_bin_value(op, a, Value::Atom(b), span, f)),
        )?),
        (Value::Array(a), Value::Array(b)) => {
            if a.len() == b.len() {
                Value::Array(Array::from_iter(
                    a.into_iter()
                        .zip(b.into_iter())
                        .map(|(a, b)| pervasize_bin_value(op, a, b, span, f)),
                )?)
            } else {
                return Err(
                    CompileErrorKind::DifferentArraySizes(op, a.into(), b.into()).at(span.clone()),
                );
            }
        }
        (left, right) => {
            return Err(
                CompileErrorKind::IncompatibleBinTypes(op, left.into(), right.into())
                    .at(span.clone()),
            )
        }
    })
}

fn pervasive_un(
    op: Op,
    inner: Const,
    span: &Span,
    f: fn(Atom, &Span) -> CompileResult<Atom>,
) -> CompileResult<Const> {
    match inner {
        Const::Value(val) => pervasive_un_value(op, val, span, f).map(Into::into),
        inner => Err(CompileErrorKind::IncompatibleUnType(op, inner).at(span.clone())),
    }
}

fn pervasive_un_value(
    op: Op,
    inner: Value,
    span: &Span,
    f: fn(Atom, &Span) -> CompileResult<Atom>,
) -> CompileResult<Value> {
    Ok(match inner {
        Value::Atom(atom) => f(atom, span)?.into(),
        Value::Array(arr) => Array::from_iter(
            arr.into_iter()
                .map(|val| pervasive_un_value(op, val, span, f)),
        )?
        .into(),
    })
}
