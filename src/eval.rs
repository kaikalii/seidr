use std::{fmt, ops::*};

use crate::{
    ast::*,
    checked::Checked,
    error::{CompileError, CompileResult, Problem},
    lex::Span,
    num::Num,
    op::{Op, Visit},
    types::{ArrayType, AtomType, Type},
    value::{Array, Atom, Value},
};

pub type EvalResult<T> = Result<T, CompileError>;

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
    fn expr(&mut self, expr: Expr) -> CompileResult<Checked> {
        self.spans.push(expr.span().clone());
        let res = match expr {
            Expr::Ident(..) => todo!(),
            Expr::Num(num, _) => Ok(num.into()),
            Expr::Char(c, _) => Ok(c.into()),
            Expr::String(s, _) => Ok(Array::String(s).into()),
            Expr::Array(expr) => {
                Checked::from_try_iter(expr.items.into_iter().map(|expr| self.expr(expr)))
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
    type Input = Checked;
    type Output = Checked;
    type Error = Problem;
    fn visit_bin(
        &self,
        left: Self::Input,
        right: Self::Input,
        state: &mut Evaler,
    ) -> Result<Self::Output, Self::Error> {
        match self {
            Op::Add => pervasize_bin(*self, left, right, Atom::add),
            Op::Sub => pervasize_bin(*self, left, right, Atom::sub),
            Op::Mul => pervasize_bin(*self, left, right, Atom::mul),
            Op::Div => pervasize_bin(*self, left, right, Atom::div),
            Op::Equal => pervasize_bin(*self, left, right, |a, b| Ok((a == b).into())),
            Op::NotEqual => pervasize_bin(*self, left, right, |a, b| Ok((a != b).into())),
            Op::Less => pervasize_bin(*self, left, right, |a, b| Ok((a < b).into())),
            Op::LessOrEqual => pervasize_bin(*self, left, right, |a, b| Ok((a <= b).into())),
            Op::Greater => pervasize_bin(*self, left, right, |a, b| Ok((a > b).into())),
            Op::GreaterOrEqual => pervasize_bin(*self, left, right, |a, b| Ok((a >= b).into())),
            op => Err(CompileError::IncompatibleBinTypes(*op, left, right)),
        }
        .map_err(|e| e.at(state.span().clone()))
    }
    fn visit_un(
        &self,
        inner: Self::Input,
        state: &mut Evaler,
    ) -> Result<Self::Output, Self::Error> {
        match self {
            Op::Add => pervasive_un(*self, inner, Ok),
            Op::Sub => pervasive_un(*self, inner, |atom| match atom {
                Atom::Num(n) => Ok(Atom::Num(-n)),
                Atom::Char(_) => Err(CompileError::IncompatibleUnType(Op::Sub, atom.into())),
            }),
            Op::Mul => pervasive_un(*self, inner, |atom| match atom {
                Atom::Num(n) => Ok(Atom::Num(n.sign())),
                Atom::Char(_) => Err(CompileError::IncompatibleUnType(Op::Mul, atom.into())),
            }),
            Op::Div => pervasive_un(*self, inner, |atom| match atom {
                Atom::Num(n) => Ok(Atom::Num(Num::Int(1) / n)),
                Atom::Char(_) => Err(CompileError::IncompatibleUnType(Op::Div, atom.into())),
            }),
            Op::Equal => Ok(Num::from(match inner {
                Checked::Value(Value::Atom(_)) => 0,
                Checked::Value(Value::Array(arr)) => arr.len(),
                Checked::Type(Type::Atom(_)) => 0,
                Checked::Type(Type::Array(arr)) => match *arr {
                    ArrayType::Empty => 0,
                    ArrayType::StaticHomo(_, len) => len,
                    ArrayType::StaticHetero(tys) => tys.len(),
                    ArrayType::DynamicHomo(_) => return Ok(AtomType::Num.into()),
                },
            })
            .into()),
            Op::Jera => Ok(match inner {
                Checked::Value(Value::Atom(_)) | Checked::Type(Type::Atom(_)) => inner,
                Checked::Value(Value::Array(array)) => {
                    let mut values: Vec<Value> = array.into_iter().collect();
                    values.reverse();
                    Array::from_iter(values).into()
                }
                Checked::Type(Type::Array(arr)) => match *arr {
                    ArrayType::StaticHetero(mut tys) => {
                        tys.reverse();
                        ArrayType::StaticHetero(tys)
                    }
                    arr => arr,
                }
                .into(),
            }),
            op => Err(CompileError::IncompatibleUnType(*op, inner)),
        }
        .map_err(|e| e.at(state.span().clone()))
    }
}

fn pervasize_bin(
    op: Op,
    left: Checked,
    right: Checked,
    f: fn(Atom, Atom) -> EvalResult<Atom>,
) -> EvalResult<Checked> {
    match (left, right) {
        (Checked::Value(a), Checked::Value(b)) => pervasize_bin_value(op, a, b, f).map(Into::into),
        (left, right) => Err(CompileError::IncompatibleBinTypes(op, left, right)),
    }
}

fn pervasize_bin_value(
    op: Op,
    left: Value,
    right: Value,
    f: fn(Atom, Atom) -> EvalResult<Atom>,
) -> EvalResult<Value> {
    Ok(match (left, right) {
        (Value::Atom(a), Value::Atom(b)) => Value::Atom(f(a, b)?),
        (Value::Atom(a), Value::Array(b)) => Value::Array(Array::from_try_iter(
            b.iter()
                .map(|b| pervasize_bin_value(op, Value::Atom(a), b, f)),
        )?),
        (Value::Array(a), Value::Atom(b)) => Value::Array(Array::from_try_iter(
            a.iter()
                .map(|a| pervasize_bin_value(op, a, Value::Atom(b), f)),
        )?),
        (Value::Array(a), Value::Array(b)) => {
            if a.len() == b.len() {
                Value::Array(Array::from_try_iter(
                    a.into_iter()
                        .zip(b.into_iter())
                        .map(|(a, b)| pervasize_bin_value(op, a, b, f)),
                )?)
            } else {
                return Err(CompileError::DifferentArraySizes(op, a.into(), b.into()));
            }
        }
        (left, right) => {
            return Err(CompileError::IncompatibleBinTypes(
                op,
                left.into(),
                right.into(),
            ))
        }
    })
}

fn pervasive_un(op: Op, inner: Checked, f: fn(Atom) -> EvalResult<Atom>) -> EvalResult<Checked> {
    match inner {
        Checked::Value(val) => pervasive_un_value(op, val, f).map(Into::into),
        inner => Err(CompileError::IncompatibleUnType(op, inner)),
    }
}

fn pervasive_un_value(op: Op, inner: Value, f: fn(Atom) -> EvalResult<Atom>) -> EvalResult<Value> {
    Ok(match inner {
        Value::Atom(atom) => f(atom)?.into(),
        Value::Array(arr) => {
            Array::from_try_iter(arr.into_iter().map(|val| pervasive_un_value(op, val, f)))?.into()
        }
    })
}
