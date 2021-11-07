use std::iter::repeat;

use crate::{
    array::{Array, ZipForm},
    cwt::{BinValNode, UnValNode, ValNode},
    error::{RuntimeError, RuntimeResult},
    function::{Atop, BinModded, Fork, Function, UnModded},
    lex::Span,
    num::Num,
    op::*,
    pervade::{bin_pervade_val, un_pervade_val},
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
            ValNode::Atop(f, g) => Ok(Function::Atop(
                Atop {
                    f: f.eval(rt)?,
                    g: g.eval(rt)?,
                }
                .into(),
            )
            .into()),
            ValNode::Fork(left, center, right) => Ok(Function::Fork(
                Fork {
                    left: left.eval(rt)?,
                    center: center.eval(rt)?,
                    right: right.eval(rt)?,
                }
                .into(),
            )
            .into()),
        }
    }
}

impl Eval for UnValNode {
    fn eval(&self, rt: &mut Runtime) -> RuntimeResult {
        let op = self.op.eval(rt)?;
        let x = self.x.eval(rt)?;
        eval_un(op, x, &self.span)
    }
}

pub fn eval_un(op: Val, x: Val, span: &Span) -> RuntimeResult {
    match op {
        Val::Atom(Atom::Function(function)) => match function {
            Function::Op(Op::Pervasive(per)) => un_pervade_val(per, x, span),
            Function::Op(Op::Rune(rune)) => match rune {
                RuneOp::Jera => Ok(reverse(x, span)),
                RuneOp::Algiz => range(x, span).map(Val::Array),
                RuneOp::Tiwaz => sort(x, span).map(Val::Array),
                rune => rt_error(format!("{} has no unary form", rune), span),
            },
            Function::Atop(atop) => {
                let lower = eval_un(atop.g, x, span)?;
                eval_un(atop.f, lower, span)
            }
            Function::Fork(fork) => {
                let left = eval_un(fork.left, x.clone(), span)?;
                let right = eval_un(fork.right, x, span)?;
                eval_bin(fork.center, left, right, span)
            }
            Function::UnMod(un_mod) => match un_mod.m {
                RuneUnMod::Raido => fold(un_mod.f, None, x, span),
                RuneUnMod::Othala => eval_bin(un_mod.f, x.clone(), x, span),
                RuneUnMod::Berkanan => each_un(un_mod.f, x, span).map(Into::into),
                m => todo!("{:?}", m),
            },
            Function::BinMod(bin_mod) => match bin_mod.m {
                m => todo!("{:?}", m),
            },
        },
        Val::Atom(Atom::UnMod(m)) => Ok(UnModded { m, f: x }.into()),
        val => Ok(val),
    }
}

impl Eval for BinValNode {
    fn eval(&self, rt: &mut Runtime) -> RuntimeResult {
        let op = self.op.eval(rt)?;
        let w = self.w.eval(rt)?;
        let x = self.x.eval(rt)?;
        eval_bin(op, w, x, &self.span)
    }
}

pub fn eval_bin(op: Val, w: Val, x: Val, span: &Span) -> RuntimeResult {
    match op {
        Val::Atom(Atom::Function(function)) => match function {
            Function::Op(Op::Pervasive(per)) => bin_pervade_val(per, w, x, span),
            Function::Op(Op::Rune(rune)) => match rune {
                RuneOp::Fehu => replicate(w, x, span).map(Val::Array),
                RuneOp::Jera => rotate(w, x, span),
                RuneOp::Iwaz => {
                    Ok(Array::JoinTo(w.into_array().into(), x.into_array().into()).into())
                }
                RuneOp::Naudiz => Ok(take(w, x, span)?.into()),
                rune => rt_error(format!("{} has no binary form", rune), span),
            },
            Function::Atop(atop) => {
                let lower = eval_bin(atop.g, w, x, span)?;
                eval_un(atop.f, lower, span)
            }
            Function::Fork(fork) => {
                let left = eval_bin(fork.left, w.clone(), x.clone(), span)?;
                let right = eval_bin(fork.right, w, x, span)?;
                eval_bin(fork.center, left, right, span)
            }
            Function::UnMod(un_mod) => match un_mod.m {
                RuneUnMod::Raido => fold(un_mod.f, Some(w), x, span),
                RuneUnMod::Othala => eval_bin(un_mod.f, x, w, span),
                RuneUnMod::Berkanan => each_bin(un_mod.f, w, x, span).map(Into::into),
                m => todo!("{:?}", m),
            },
            Function::BinMod(bin_mod) => match bin_mod.m {
                m => todo!("{:?}", m),
            },
        },
        Val::Atom(Atom::BinMod(m)) => Ok(BinModded { m, f: w, g: x }.into()),
        val => Ok(val),
    }
}

fn rotate(w: Val, x: Val, span: &Span) -> RuntimeResult {
    match (w, x) {
        (Val::Atom(Atom::Num(n)), Val::Array(arr)) => {
            Ok(Array::Rotate(arr.into(), i64::from(n)).into())
        }
        (Val::Array(ns), x) if ns.len() == Some(1) => {
            rotate(ns.get(0)?.unwrap().into_owned(), x, span)
        }
        (Val::Array(ns), Val::Array(arr)) => {
            let mut ns = ns.into_iter();
            let first = ns.next().unwrap()?;
            let sub_ns: Array = ns.skip(1).collect::<RuntimeResult<_>>()?;
            rotate(
                first,
                Array::try_concrete(
                    arr.into_iter()
                        .map(|sub| sub.and_then(|sub| rotate(sub_ns.clone().into(), sub, span))),
                )?
                .into(),
                span,
            )
        }
        (Val::Atom(atom), _) => rt_error(
            format!("Attempted to rotate with {}", atom.type_name()),
            span,
        ),
        (_, Val::Atom(_)) => rt_error("x must be an array", span),
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
            if n < 0 {
                rt_error("x must be natural numbers", span)
            } else {
                Ok(Array::Range(n))
            }
        }
        Val::Atom(atom) => rt_error(
            format!("A range cannot be built from {}", atom.type_name()),
            span,
        ),
        Val::Array(arr) => {
            if arr.len().map_or(true, |len| len == 0) {
                rt_error("Range array must have a positive, finite size", span)
            } else {
                Ok(Array::Product(
                    arr.into_iter()
                        .map(|val| val.and_then(|val| range(val, span)))
                        .collect::<RuntimeResult<_>>()?,
                    RcView::new([]),
                ))
            }
        }
    }
}

fn replicate(w: Val, x: Val, span: &Span) -> RuntimeResult<Array> {
    match (w, x) {
        (Val::Atom(Atom::Num(w)), Val::Atom(x)) => {
            let n = i64::from(w);
            if n < 0 {
                rt_error("Replicator must be natural numbers", span)
            } else {
                let n = n as usize;
                Ok(Array::concrete(repeat(x).take(n)))
            }
        }
        (w @ Val::Atom(Atom::Num(_)), Val::Array(x)) => {
            let arrays: Vec<Array> = x
                .into_iter()
                .map(|x| x.and_then(|x| replicate(w.clone(), x, span)))
                .collect::<RuntimeResult<_>>()?;
            Ok(Array::Concrete(
                arrays.into_iter().flatten().collect::<RuntimeResult<_>>()?,
            ))
        }
        (Val::Array(w), Val::Array(x)) => {
            if w.len() == x.len() {
                let arrays: Vec<Array> = w
                    .into_iter()
                    .zip(x)
                    .map(|(w, x)| w.and_then(|w| x.and_then(|x| replicate(w, x, span))))
                    .collect::<RuntimeResult<_>>()?;
                Ok(Array::Concrete(
                    arrays.into_iter().flatten().collect::<RuntimeResult<_>>()?,
                ))
            } else {
                rt_error("Arrays must have matching lengths", span)
            }
        }
        (Val::Array(_), Val::Atom(x)) => rt_error(
            format!("{} cannot be replicated with array", x.type_name()),
            span,
        ),
        (Val::Atom(w), x) => rt_error(
            format!("{} cannot be used to replicate", w.type_name()),
            span,
        ),
    }
}

pub fn take(w: Val, x: Val, span: &Span) -> RuntimeResult<Array> {
    match (w, x) {
        (Val::Atom(Atom::Num(n)), Val::Array(arr)) => Ok(Array::Take(arr.into(), i64::from(n))),
        (w, x) => rt_error(
            format!(
                "Attempted to take {} items from {}",
                w.type_name(),
                x.type_name()
            ),
            span,
        ),
    }
}

pub fn fold(op: Val, w: Option<Val>, x: Val, span: &Span) -> RuntimeResult {
    match x {
        Val::Array(arr) => {
            if let Some(w) = w {
                arr.into_iter()
                    .fold(Ok(w), |acc, val| eval_bin(op.clone(), acc?, val?, span))
            } else {
                let val = arr
                    .into_iter()
                    .reduce(|acc, val| eval_bin(op.clone(), acc?, val?, span))
                    .transpose()?;
                if let Some(val) = val.or(w) {
                    Ok(val)
                } else {
                    fold_identity(&op, span)
                }
            }
        }
        Val::Atom(atom) => rt_error(format!("Attempted to fold over {}", atom.type_name()), span),
    }
}

pub fn fold_identity(op: &Val, span: &Span) -> RuntimeResult {
    Ok(match op {
        Val::Atom(Atom::Function(function)) => match function {
            Function::Op(Op::Pervasive(Pervasive::Math(math))) => match math {
                MathOp::Add | MathOp::Sub => 0i64.into(),
                MathOp::Mul | MathOp::Div => 1i64.into(),
                MathOp::Max => (-Num::INFINIFY).into(),
                MathOp::Min => Num::INFINIFY.into(),
                op => return rt_error(format!("{} has no fold identity", op), span),
            },
            function => return rt_error(format!("{} has no fold identity", function), span),
        },
        val => val.clone(),
    })
}

pub fn each_un(op: Val, x: Val, span: &Span) -> RuntimeResult<Array> {
    match x {
        Val::Array(arr) => Ok(Array::Each(
            ZipForm::Un(arr).into(),
            op.into(),
            span.clone(),
        )),
        Val::Atom(atom) => rt_error(format!("Each cannot be used on {}", atom.type_name()), span),
    }
}

pub fn each_bin(op: Val, w: Val, x: Val, span: &Span) -> RuntimeResult<Array> {
    match ZipForm::bin(w, x) {
        Ok(zip) => Ok(Array::Each(zip.into(), op.into(), span.clone())),
        Err((w, x)) => rt_error(
            format!(
                "Each cannot be used on {} and {}",
                w.type_name(),
                x.type_name()
            ),
            span,
        ),
    }
}

pub fn sort(x: Val, span: &Span) -> RuntimeResult<Array> {
    match x {
        Val::Array(arr) => {
            let mut items = arr.into_vec()?;
            items.sort_unstable();
            Ok(Array::concrete(items))
        }
        Val::Atom(atom) => rt_error(format!("{} cannot be sorted", atom.type_name()), span),
    }
}

pub fn rt_error<T>(message: impl Into<String>, span: &Span) -> RuntimeResult<T> {
    Err(RuntimeError::new(message, span.clone()))
}
