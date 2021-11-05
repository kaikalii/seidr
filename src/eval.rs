use std::iter::repeat;

use crate::{
    array::Array,
    cwt::{BinVal, UnVal, ValNode},
    error::{RuntimeError, RuntimeResult},
    lex::Span,
    num::Num,
    op::*,
    rcview::RcView,
    value::{Atom, Val},
};

#[derive(Default)]
pub struct Runtime {}

impl Runtime {}

pub trait Eval {
    fn eval(&self, rt: &mut Runtime) -> RuntimeResult;
}

impl Eval for Val {
    fn eval(&self, _: &mut Runtime) -> RuntimeResult {
        Ok(self.clone())
    }
}

impl Eval for ValNode {
    fn eval(&self, rt: &mut Runtime) -> RuntimeResult {
        match self {
            ValNode::Val(val) => val.eval(rt),
            ValNode::Un(un) => un.eval(rt),
            ValNode::Bin(bin) => bin.eval(rt),
            ValNode::Array(arr) => {
                let vals: Vec<Val> = arr
                    .iter()
                    .map(|node| node.eval(rt))
                    .collect::<RuntimeResult<_>>()?;
                Ok(Val::from_iter(vals))
            }
        }
    }
}

impl Eval for UnVal {
    fn eval(&self, rt: &mut Runtime) -> RuntimeResult {
        let op = self.op.eval(rt)?;
        let span = self.op.span.clone();
        let x = self.x.eval(rt)?;
        match op {
            Val::Atom(Atom::Op(Op::Pervasive(per))) => un_pervade_val(per, x, &span),
            Val::Atom(Atom::Op(Op::Rune(rune))) => match rune {
                RuneOp::Jera => Ok(reverse(x, &span)),
                RuneOp::Algiz => range(x, &span).map(Val::Array),
                rune => error(format!("{} has no unary form", rune), &span),
            },
            val => Ok(val),
        }
    }
}

impl Eval for BinVal {
    fn eval(&self, rt: &mut Runtime) -> RuntimeResult {
        let op = self.op.eval(rt)?;
        let span = self.op.span.clone();
        let w = self.w.eval(rt)?;
        let x = self.x.eval(rt)?;
        match op {
            Val::Atom(Atom::Op(Op::Pervasive(per))) => bin_pervade_val(per, w, x, &span),
            Val::Atom(Atom::Op(Op::Rune(rune))) => match rune {
                RuneOp::Fehu => replicate(w, x, &span).map(Val::Array),
                RuneOp::Jera => rotate(w, x, &span),
                RuneOp::Laguz => {
                    Ok(Array::JoinTo(w.into_array().into(), x.into_array().into()).into())
                }
                rune => error(format!("{} has no binary form", rune), &span),
            },
            val => Ok(val),
        }
    }
}

pub fn un_pervade_val(per: Pervasive, x: Val, span: &Span) -> RuntimeResult {
    Ok(match (per, x) {
        (per, Val::Atom(x)) => un_pervade_atom(per, x, span)?,
        (Pervasive::Comparison(cmp), Val::Array(arr)) => match cmp {
            ComparisonOp::Equal => arr.len().map(Num::from).unwrap_or(Num::INFINIFY).into(),
            cmp => todo!("{}", cmp),
        },
        (per @ Pervasive::Math(_), Val::Array(x)) => {
            x.pervade(|x| un_pervade_val(per, x, span))?.into()
        }
    })
}

pub fn bin_pervade_val(per: Pervasive, w: Val, x: Val, span: &Span) -> RuntimeResult {
    Ok(match (w, x) {
        (Val::Atom(w), Val::Atom(x)) => bin_pervade_atom(per, w, x, span)?,
        (Val::Array(w), Val::Array(x)) => w
            .pervade_with(&x, span, |w, x| bin_pervade_val(per, w, x, span))?
            .into(),
        (w, Val::Array(x)) => x
            .pervade(|x| bin_pervade_val(per, w.clone(), x, span))?
            .into(),
        (Val::Array(w), x) => w
            .pervade(|w| bin_pervade_val(per, w, x.clone(), span))?
            .into(),
    })
}

pub fn un_pervade_atom(per: Pervasive, x: Atom, span: &Span) -> RuntimeResult {
    match (per, x) {
        (Pervasive::Math(MathOp::Add), atom) => Ok(atom.into()),
        (Pervasive::Math(MathOp::Sub), Atom::Num(n)) => Ok((-n).into()),
        (Pervasive::Math(MathOp::Sub), atom) => {
            error(format!("{} cannot be negated", atom.type_name()), span)
        }
        (Pervasive::Math(MathOp::Mul), Atom::Num(n)) => Ok(n.sign().into()),
        (Pervasive::Math(MathOp::Div), Atom::Num(n)) => Ok((Num::Int(1) / n).into()),
        _ => error(format!("{} {} is invalid", per, x.type_name()), span),
    }
}

pub fn bin_pervade_atom(per: Pervasive, w: Atom, x: Atom, span: &Span) -> RuntimeResult {
    match per {
        Pervasive::Math(math) => match (w, x) {
            (Atom::Num(w), Atom::Num(x)) => {
                return Ok((match math {
                    MathOp::Add => w + x,
                    MathOp::Sub => w - x,
                    MathOp::Mul => w * x,
                    MathOp::Div => w / x,
                })
                .into())
            }
            (Atom::Char(w), Atom::Num(x)) => {
                let w = w as u32;
                let x = u32::from(x);
                match math {
                    MathOp::Add => {
                        return Ok(char::from_u32(w.saturating_add(x))
                            .unwrap_or_default()
                            .into())
                    }
                    MathOp::Sub => {
                        return Ok(char::from_u32(w.saturating_sub(x))
                            .unwrap_or_default()
                            .into())
                    }
                    _ => {}
                }
            }
            (Atom::Num(w), Atom::Char(x)) if math == MathOp::Add => {
                return Ok(char::from_u32((i64::from(w) + x as u32 as i64) as u32)
                    .unwrap_or_default()
                    .into())
            }
            (Atom::Char(w), Atom::Char(x)) if math == MathOp::Sub => {
                return Ok((Num::from(w as u32) - Num::from(x as u32)).into())
            }
            _ => {}
        },
        Pervasive::Comparison(comp) => {
            return Ok(match comp {
                ComparisonOp::Equal => w == x,
                ComparisonOp::NotEqual => w != x,
                ComparisonOp::Less => w < x,
                ComparisonOp::LessOrEqual => w <= x,
                ComparisonOp::Greater => w > x,
                ComparisonOp::GreaterOrEqual => w >= x,
            }
            .into())
        }
    }
    error(
        format!("{} {} {} is invalid", w.type_name(), per, x.type_name()),
        span,
    )
}

fn rotate(w: Val, x: Val, span: &Span) -> RuntimeResult {
    match (w, x) {
        (Val::Atom(Atom::Num(n)), Val::Array(arr)) => {
            Ok(Array::Rotate(arr.into(), i64::from(n)).into())
        }
        (Val::Array(ns), x) if ns.len() == Some(1) => {
            rotate(ns.get(0).unwrap().into_owned(), x, span)
        }
        (Val::Array(ns), Val::Array(arr)) => {
            let mut ns = ns.into_iter();
            let first = ns.next().unwrap();
            let sub_ns: Array = ns.skip(1).collect();
            let mut items: Vec<Val> = arr
                .into_iter()
                .map(|sub| rotate(sub_ns.clone().into(), sub, span))
                .collect::<RuntimeResult<_>>()?;
            rotate(first, Array::concrete(items).into(), span)
        }
        (Val::Atom(atom), _) => error(
            format!("Attempted to rotate with {}", atom.type_name()),
            span,
        ),
        (_, Val::Atom(_)) => error("x must be an array", span),
    }
}

fn reverse(x: Val, span: &Span) -> Val {
    match x {
        Val::Atom(_) => x,
        Val::Array(arr) => Array::Reverse(arr.into()).into(),
    }
}

fn range(x: Val, span: &Span) -> RuntimeResult<Array> {
    match x {
        Val::Atom(Atom::Num(n)) => {
            let n = i64::from(n);
            if n < 0 {
                error("x must be natural numbers", span)
            } else {
                Ok(Array::Range(n as usize))
            }
        }
        Val::Atom(atom) => error(
            format!("A range cannot be built from {}", atom.type_name()),
            span,
        ),
        Val::Array(arr) => {
            if arr.len().map_or(true, |len| len == 0) {
                error("Range array must have a positive, finite size", span)
            } else {
                let arrays: Vec<Array> = arr
                    .into_iter()
                    .map(|val| range(val, span))
                    .collect::<RuntimeResult<_>>()?;
                Ok(Array::Product(arrays.into(), RcView::new([])))
            }
        }
    }
}

fn replicate(w: Val, x: Val, span: &Span) -> RuntimeResult<Array> {
    match (w, x) {
        (Val::Atom(Atom::Num(w)), Val::Atom(x)) => {
            let n = i64::from(w);
            if n < 0 {
                error("Replicator must be natural numbers", span)
            } else {
                let n = n as usize;
                Ok(Array::concrete(repeat(x).take(n)))
            }
        }
        (w @ Val::Atom(Atom::Num(_)), Val::Array(x)) => {
            let mut arrays: Vec<Array> = x
                .into_iter()
                .map(|x| replicate(w.clone(), x, span))
                .collect::<RuntimeResult<_>>()?;
            Ok(arrays.into_iter().flatten().collect())
        }
        (Val::Array(w), Val::Array(x)) => {
            if w.len() == x.len() {
                let mut arrays: Vec<Array> = w
                    .into_iter()
                    .zip(x)
                    .map(|(w, x)| replicate(w, x, span))
                    .collect::<RuntimeResult<_>>()?;
                Ok(arrays.into_iter().flatten().collect())
            } else {
                error("Arrays must have matching lengths", span)
            }
        }
        (Val::Array(_), Val::Atom(x)) => error(
            format!("{} cannot be replicated with array", x.type_name()),
            span,
        ),
        (Val::Atom(w), x) => error(
            format!("{} cannot be used to replicate", w.type_name()),
            span,
        ),
    }
}

fn error<T>(message: impl Into<String>, span: &Span) -> RuntimeResult<T> {
    Err(RuntimeError::new(message, span.clone()))
}
