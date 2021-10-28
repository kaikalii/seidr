use std::{fmt, ops::*};

use crate::{
    ast::*,
    check::Check,
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
    fn expr(&mut self, expr: Expr) -> CompileResult<Check> {
        self.spans.push(expr.span().clone());
        let res = match expr {
            Expr::Ident(..) => todo!(),
            Expr::Num(num, _) => Ok(num.into()),
            Expr::Char(c, _) => Ok(c.into()),
            Expr::String(s, _) => Ok(Array::String(s).into()),
            Expr::Array(expr) => {
                Check::from_try_iter(expr.items.into_iter().map(|expr| self.expr(expr)))
            }
            Expr::Un(expr) => {
                let x = self.expr(expr.inner)?;
                expr.op.visit_un(x, self)
            }
            Expr::Bin(expr) => {
                let w = self.expr(expr.left)?;
                let x = self.expr(expr.right)?;
                expr.op.visit_bin(w, x, self)
            }
        };
        self.spans.pop().unwrap();
        res
    }
}

impl Visit<Evaler> for Op {
    type Input = Check;
    type Output = Check;
    type Error = Problem;
    fn visit_bin(
        &self,
        w: Self::Input,
        x: Self::Input,
        state: &mut Evaler,
    ) -> Result<Self::Output, Self::Error> {
        match self {
            Op::Add => pervasize_bin(*self, w, x, Atom::add),
            Op::Sub => pervasize_bin(*self, w, x, Atom::sub),
            Op::Mul => pervasize_bin(*self, w, x, Atom::mul),
            Op::Div => pervasize_bin(*self, w, x, Atom::div),
            Op::Equal => pervasize_bin(*self, w, x, |w, x| Ok((w == x).into())),
            Op::NotEqual => pervasize_bin(*self, w, x, |w, x| Ok((w != x).into())),
            Op::Less => pervasize_bin(*self, w, x, |w, x| Ok((w < x).into())),
            Op::LessOrEqual => pervasize_bin(*self, w, x, |w, x| Ok((w <= x).into())),
            Op::Greater => pervasize_bin(*self, w, x, |w, x| Ok((w > x).into())),
            Op::GreaterOrEqual => pervasize_bin(*self, w, x, |w, x| Ok((w >= x).into())),
            Op::Jera => bin_jera(w, x),
            op => Err(CompileError::IncompatibleBinTypes(*op, w, x)),
        }
        .map_err(|e| e.at(state.span().clone()))
    }
    fn visit_un(&self, x: Self::Input, state: &mut Evaler) -> Result<Self::Output, Self::Error> {
        match self {
            Op::Add => pervasive_un(*self, x, Ok),
            Op::Sub => pervasive_un(*self, x, |atom| match atom {
                Atom::Num(n) => Ok(Atom::Num(-n)),
                Atom::Char(_) => Err(CompileError::IncompatibleUnType(Op::Sub, atom.into())),
            }),
            Op::Mul => pervasive_un(*self, x, |atom| match atom {
                Atom::Num(n) => Ok(Atom::Num(n.sign())),
                Atom::Char(_) => Err(CompileError::IncompatibleUnType(Op::Mul, atom.into())),
            }),
            Op::Div => pervasive_un(*self, x, |atom| match atom {
                Atom::Num(n) => Ok(Atom::Num(Num::Int(1) / n)),
                Atom::Char(_) => Err(CompileError::IncompatibleUnType(Op::Div, atom.into())),
            }),
            Op::Equal => Ok(Num::from(match x {
                Check::Value(Value::Atom(_)) => 0,
                Check::Value(Value::Array(arr)) => arr.len(),
                Check::Type(Type::Atom(_)) => 0,
                Check::Type(Type::Array(arr)) => match arr {
                    ArrayType::Empty => 0,
                    ArrayType::StaticHomo(_, len) => len,
                    ArrayType::StaticHetero(tys) => tys.len(),
                    ArrayType::DynamicHomo(_) => return Ok(AtomType::Num.into()),
                },
            })
            .into()),
            Op::Jera => Ok(un_jera(x)),
            op => op.err_un(x),
        }
        .map_err(|e| e.at(state.span().clone()))
    }
}

fn pervasize_bin(
    op: Op,
    w: Check,
    x: Check,
    f: fn(Atom, Atom) -> EvalResult<Atom>,
) -> EvalResult<Check> {
    match (w, x) {
        (Check::Value(w), Check::Value(x)) => pervasize_bin_value(op, w, x, f).map(Into::into),
        (w, x) => op.err_bin(w, x),
    }
}

fn pervasize_bin_value(
    op: Op,
    w: Value,
    x: Value,
    f: fn(Atom, Atom) -> EvalResult<Atom>,
) -> EvalResult<Value> {
    Ok(match (w, x) {
        (Value::Atom(w), Value::Atom(x)) => Value::Atom(f(w, x)?),
        (Value::Atom(w), Value::Array(x)) => Value::Array(Array::from_try_iter(
            x.iter()
                .map(|b| pervasize_bin_value(op, Value::Atom(w), b, f)),
        )?),
        (Value::Array(w), Value::Atom(x)) => Value::Array(Array::from_try_iter(
            w.iter()
                .map(|a| pervasize_bin_value(op, a, Value::Atom(x), f)),
        )?),
        (Value::Array(w), Value::Array(x)) => {
            if w.len() == x.len() {
                Value::Array(Array::from_try_iter(
                    w.into_iter()
                        .zip(x.into_iter())
                        .map(|(w, x)| pervasize_bin_value(op, w, x, f)),
                )?)
            } else {
                return Err(CompileError::DifferentArraySizes(op, w.into(), x.into()));
            }
        }
        (w, x) => return op.err_bin(w, x),
    })
}

fn pervasive_un(op: Op, x: Check, f: fn(Atom) -> EvalResult<Atom>) -> EvalResult<Check> {
    match x {
        Check::Value(val) => pervasive_un_value(op, val, f).map(Into::into),
        x => op.err_un(x),
    }
}

fn pervasive_un_value(op: Op, x: Value, f: fn(Atom) -> EvalResult<Atom>) -> EvalResult<Value> {
    Ok(match x {
        Value::Atom(atom) => f(atom)?.into(),
        Value::Array(arr) => {
            Array::from_try_iter(arr.into_iter().map(|val| pervasive_un_value(op, val, f)))?.into()
        }
    })
}

fn un_jera(x: Check) -> Check {
    match x {
        Check::Type(Type::Array(ArrayType::StaticHetero(mut tys))) => {
            tys.reverse();
            ArrayType::StaticHetero(tys).into()
        }
        Check::Value(Value::Atom(_)) | Check::Type(Type::Atom(_)) => x,
        Check::Value(Value::Array(array)) => {
            let mut values: Vec<Value> = array.into_iter().collect();
            values.reverse();
            Array::from_iter(values).into()
        }
        Check::Type(Type::Array(arr)) => arr.into(),
    }
}

fn bin_jera(w: Check, x: Check) -> EvalResult<Check> {
    match (w, x) {
        (Check::Value(Value::Atom(Atom::Num(n))), Check::Value(Value::Array(arr))) => {
            let mut values: Vec<Value> = arr.into_iter().collect();
            if n >= 0 {
                values.rotate_left(n.into())
            } else {
                values.rotate_right(-i64::from(n) as usize)
            }
            Ok(Array::from_iter(values).into())
        }
        (
            Check::Value(Value::Atom(Atom::Num(n))),
            Check::Type(Type::Array(ArrayType::StaticHetero(mut tys))),
        ) => {
            if n >= 0 {
                tys.rotate_left(n.into())
            } else {
                tys.rotate_right(-i64::from(n) as usize)
            }
            Ok(Type::from_iter(tys).into())
        }
        (Check::Value(Value::Atom(Atom::Num(_))), Check::Type(Type::Array(arr))) => Ok(arr.into()),
        (w, x) => Op::Jera.err_bin(w, x),
    }
}
