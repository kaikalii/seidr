use std::iter::repeat;

use crate::{
    array::{Array, EachArray, ReplicateArray, SelectArray, ZipForm},
    ast::Format,
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
            Function::Op(Op::Pervasive(Pervasive::Comparison(ComparisonOp::Equal))) => match x {
                Val::Array(arr) => Ok(arr.len().map(Num::from).unwrap_or(Num::INFINIFY).into()),
                Val::Atom(_) => Ok(1i64.into()),
            },
            Function::Op(Op::Pervasive(per)) => un_pervade_val(per, x, span),
            Function::Op(Op::Rune(rune)) => match rune {
                RuneOp::Kaunan | RuneOp::Laguz => Ok(x),
                RuneOp::Jera => Ok(reverse(x, span)),
                RuneOp::Algiz => range(x, span).map(Val::Array),
                RuneOp::Tiwaz => sort(x, span).map(Val::Array),
                RuneOp::Sowilo => grade(x, span).map(Val::Array),
                rune => rt_error(format!("{} has no unary form", rune), span),
            },
            Function::Op(Op::Other(other)) => match other {
                OtherOp::Match => x.depth().map(Into::into),
                other => todo!("{:?}", other),
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
                RuneUnMod::Ingwaz => eval_un(un_mod.f, x, span),
                RuneUnMod::Raido => fold(un_mod.f, None, x, span),
                RuneUnMod::Othala => eval_bin(un_mod.f, x.clone(), x, span),
                RuneUnMod::Berkanan => each_un(un_mod.f, x, span).map(Into::into),
                RuneUnMod::Ing => undo_un(un_mod.f, x, span),
                m => todo!("{:?}", m),
            },
            Function::BinMod(bin_mod) => match bin_mod.m {
                RuneBinMod::Ehwaz => {
                    let right = eval_un(bin_mod.g, x.clone(), span)?;
                    eval_bin(bin_mod.f, x, right, span)
                }
                RuneBinMod::Haglaz => eval_un(bin_mod.f, eval_un(bin_mod.g, x, span)?, span),
                RuneBinMod::Dagaz => {
                    let condition = eval_un(bin_mod.f, x.clone(), span)?;
                    let branches = eval_un(bin_mod.g, x.clone(), span)?;
                    let chosen = index(condition, branches, span)?;
                    eval_un(chosen, x, span)
                }
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
                RuneOp::Kaunan => Ok(w),
                RuneOp::Laguz => Ok(x),
                RuneOp::Fehu => replicate(w, x, span).map(Val::Array),
                RuneOp::Jera => rotate(w, x, span),
                RuneOp::Iwaz => {
                    Ok(Array::JoinTo(w.into_array().into(), x.into_array().into()).into())
                }
                RuneOp::Naudiz => take(w, x, span).map(Val::from),
                RuneOp::Gebo => drop(w, x, span).map(Val::from),
                RuneOp::Perth => index(w, x, span),
                RuneOp::Ansuz => select(w, x, span),
                RuneOp::Algiz => windows(w, x, span).map(Val::from),
                RuneOp::Uruz => chunks(w, x, span).map(Val::from),
                rune => rt_error(format!("{} has no binary form", rune), span),
            },
            Function::Op(Op::Other(other)) => match other {
                OtherOp::Match => w.matches(&x).map(Val::from),
                OtherOp::DoNotMatch => w.matches(&x).map(|matches| (!matches).into()),
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
                RuneUnMod::Ingwaz => eval_bin(un_mod.f, w, x, span),
                RuneUnMod::Raido => fold(un_mod.f, Some(w), x, span),
                RuneUnMod::Othala => eval_bin(un_mod.f, x, w, span),
                RuneUnMod::Berkanan => each_bin(un_mod.f, w, x, span).map(Into::into),
                RuneUnMod::Ing => undo_bin(un_mod.f, w, x, span),
                m => todo!("{:?}", m),
            },
            Function::BinMod(bin_mod) => match bin_mod.m {
                RuneBinMod::Ehwaz => {
                    let right = eval_un(bin_mod.g, x, span)?;
                    eval_bin(bin_mod.f, w, right, span)
                }
                RuneBinMod::Haglaz => {
                    let w = eval_un(bin_mod.g.clone(), w, span)?;
                    let x = eval_un(bin_mod.g, x, span)?;
                    eval_bin(bin_mod.f, w, x, span)
                }
                RuneBinMod::Dagaz => {
                    let condition = eval_bin(bin_mod.f, w.clone(), x.clone(), span)?;
                    let branches = eval_bin(bin_mod.g, w.clone(), x.clone(), span)?;
                    let chosen = index(condition, branches, span)?;
                    eval_bin(chosen, w, x, span)
                }
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

pub fn replicator_int(n: Val, span: &Span) -> RuntimeResult<usize> {
    match n {
        Val::Atom(Atom::Num(n)) if n >= 0 => Ok(i64::from(n) as usize),
        Val::Atom(Atom::Num(_)) => rt_error("Replicator cannot be negative", span),
        val => rt_error(
            format!("{} cannot be used to replicate", val.type_name()),
            span,
        ),
    }
}

fn replicate(w: Val, x: Val, span: &Span) -> RuntimeResult<Array> {
    match (w, x) {
        (Val::Array(w), Val::Array(x)) => Ok(if w.len().is_some() && x.len().is_some() {
            let arrays: Vec<Array> = w
                .into_iter()
                .zip(x)
                .map(|(w, x)| w.and_then(|w| x.and_then(|x| replicate(w, x, span))))
                .collect::<RuntimeResult<_>>()?;
            Array::Concrete(arrays.into_iter().flatten().collect::<RuntimeResult<_>>()?)
        } else {
            Array::Replicate(ReplicateArray::counts(w, x, span.clone()).into())
        }),
        (w, x) => {
            let n = replicator_int(w, span)?;
            Ok(match x {
                Val::Atom(x) => Array::concrete(repeat(x).take(n)),
                Val::Array(x) => {
                    let arrays: Vec<Array> = x
                        .into_iter()
                        .map(|x| x.and_then(|x| replicate(n.into(), x, span)))
                        .collect::<RuntimeResult<_>>()?;
                    Array::Concrete(arrays.into_iter().flatten().collect::<RuntimeResult<_>>()?)
                }
            })
        }
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

pub fn drop(w: Val, x: Val, span: &Span) -> RuntimeResult<Array> {
    match (w, x) {
        (Val::Atom(Atom::Num(n)), Val::Array(arr)) => Ok(Array::Drop(arr.into(), i64::from(n))),
        (w, x) => rt_error(
            format!(
                "Attempted to drop {} items from {}",
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
            function => {
                return rt_error(
                    format!("{} has no fold identity", function.as_string()?),
                    span,
                )
            }
        },
        val => val.clone(),
    })
}

pub fn each_un(op: Val, x: Val, span: &Span) -> RuntimeResult<Array> {
    match x {
        Val::Array(arr) => Ok(Array::Each(
            EachArray {
                zip: ZipForm::Un(arr),
                f: op,
                span: span.clone(),
            }
            .into(),
        )),
        Val::Atom(atom) => rt_error(format!("Each cannot be used on {}", atom.type_name()), span),
    }
}

pub fn each_bin(op: Val, w: Val, x: Val, span: &Span) -> RuntimeResult<Array> {
    match ZipForm::bin(w, x) {
        Ok(zip) => Ok(Array::Each(
            EachArray {
                zip,
                f: op,
                span: span.clone(),
            }
            .into(),
        )),
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

pub fn grade(x: Val, span: &Span) -> RuntimeResult<Array> {
    match x {
        Val::Array(arr) => {
            let mut items: Vec<(usize, Val)> = arr.into_vec()?.into_iter().enumerate().collect();
            items.sort_unstable_by(|(_, a), (_, b)| a.cmp(b));
            Ok(Array::concrete(items.into_iter().map(|(i, _)| i)))
        }
        Val::Atom(atom) => rt_error(format!("{} cannot be graded", atom.type_name()), span),
    }
}

pub fn index(w: Val, x: Val, span: &Span) -> RuntimeResult {
    match x {
        Val::Array(arr) => index_array(w, &arr, span),
        Val::Atom(atom) => rt_error(format!("{} cannot be indexed", atom.type_name()), span),
    }
}

pub fn index_array(w: Val, x: &Array, span: &Span) -> RuntimeResult {
    match w {
        Val::Atom(Atom::Num(i)) => {
            let i = i64::from(i);
            let val = if i >= 0 {
                x.get(i as usize)?
            } else if let Some(len) = x.len() {
                let i = i.abs() as usize;
                if i <= len {
                    x.get(len - i)?
                } else {
                    None
                }
            } else {
                return rt_error(
                    "Attempted to index unbounded array with a negative index",
                    span,
                );
            };
            if let Some(val) = val {
                Ok(val.into_owned())
            } else {
                rt_error(
                    format!(
                        "Index {} is out of bounds of array length {}",
                        i,
                        x.len().unwrap_or(0)
                    ),
                    span,
                )
            }
        }
        Val::Atom(atom) => rt_error(
            format!("{} cannot be used as an index", atom.type_name()),
            span,
        ),
        Val::Array(indices) => {
            let mut indices = indices.into_iter();
            Ok(if let Some(i) = indices.next().transpose()? {
                let mut item = index_array(i, x, span)?;
                for i in indices {
                    let i = i?;
                    item = index(i, item, span)?
                }
                item
            } else {
                x.clone().into()
            })
        }
    }
}

pub fn select(w: Val, x: Val, span: &Span) -> RuntimeResult {
    match w {
        w @ Val::Atom(_) => index(w, x, span),
        Val::Array(w) => match x {
            Val::Array(x) => Ok(Array::Select(
                SelectArray {
                    indices: w,
                    array: x,
                    span: span.clone(),
                }
                .into(),
            )
            .into()),
            Val::Atom(atom) => rt_error(
                format!("{} cannot be selected from", atom.type_name()),
                span,
            ),
        },
    }
}

pub fn undo_un(op: Val, x: Val, span: &Span) -> RuntimeResult {
    match &op {
        Val::Atom(Atom::Function(function)) => match function {
            Function::Op(Op::Pervasive(
                per @ Pervasive::Math(MathOp::Add | MathOp::Sub | MathOp::Div),
            )) => un_pervade_val(*per, x, span),
            Function::Op(Op::Pervasive(Pervasive::Math(MathOp::Pow))) => {
                un_pervade_val(Pervasive::Math(MathOp::Log), x, span)
            }
            Function::Op(Op::Pervasive(Pervasive::Math(MathOp::Log))) => {
                un_pervade_val(Pervasive::Math(MathOp::Pow), x, span)
            }
            Function::UnMod(un_mod) => match un_mod.m {
                RuneUnMod::Ing => eval_un(op, x, span),
                m => rt_error(
                    format!("Undoing unary {} is not supported", function.as_string()?),
                    span,
                ),
            },
            function => rt_error(
                format!("Undoing unary {} is not supported", function.as_string()?),
                span,
            ),
        },
        _ => Ok(x),
    }
}

pub fn undo_bin(op: Val, w: Val, x: Val, span: &Span) -> RuntimeResult {
    match &op {
        Val::Atom(Atom::Function(function)) => match function {
            Function::Op(Op::Pervasive(Pervasive::Math(MathOp::Add))) => {
                bin_pervade_val(Pervasive::Math(MathOp::Sub), x, w, span)
            }
            Function::Op(Op::Pervasive(Pervasive::Math(MathOp::Sub))) => {
                bin_pervade_val(Pervasive::Math(MathOp::Add), x, w, span)
            }
            Function::Op(Op::Pervasive(Pervasive::Math(MathOp::Mul))) => {
                bin_pervade_val(Pervasive::Math(MathOp::Div), x, w, span)
            }
            Function::Op(Op::Pervasive(Pervasive::Math(MathOp::Div))) => {
                bin_pervade_val(Pervasive::Math(MathOp::Mul), x, w, span)
            }
            Function::Op(Op::Pervasive(Pervasive::Math(MathOp::Pow))) => {
                bin_pervade_val(Pervasive::Math(MathOp::Log), x, w, span)
            }
            Function::Op(Op::Pervasive(Pervasive::Math(MathOp::Log))) => {
                bin_pervade_val(Pervasive::Math(MathOp::Pow), x, w, span)
            }
            Function::UnMod(un_mod) => match un_mod.m {
                RuneUnMod::Ing => eval_bin(op, w, x, span),
                m => rt_error(
                    format!("Undoing binary {} is not supported", function.as_string()?),
                    span,
                ),
            },
            function => rt_error(
                format!("Undoing binary {} is not supported", function.as_string()?),
                span,
            ),
        },
        _ => Ok(x),
    }
}

pub fn windows(w: Val, x: Val, span: &Span) -> RuntimeResult<Array> {
    match (w, x) {
        (Val::Atom(Atom::Num(n)), Val::Array(arr)) => {
            let n = i64::from(n);
            if n < 0 {
                rt_error("Windows size cannot be negative", span)
            } else {
                Ok(Array::Windows(arr.into(), n as usize))
            }
        }
        (w, Val::Array(_)) => rt_error(
            format!(
                "Windows size must be non-zero number, but it is {}",
                w.type_name()
            ),
            span,
        ),
        (_, x) => rt_error(format!("Cannot get windows from {}", x.type_name()), span),
    }
}

pub fn chunks(w: Val, x: Val, span: &Span) -> RuntimeResult<Array> {
    match (w, x) {
        (Val::Atom(Atom::Num(n)), Val::Array(arr)) => {
            let n = i64::from(n);
            if n > 0 {
                Ok(Array::Chunks(arr.into(), n as usize))
            } else {
                rt_error("Chunks size must be positive", span)
            }
        }
        (w, Val::Array(_)) => rt_error(
            format!(
                "Chunks size must be positive number, but it is {}",
                w.type_name()
            ),
            span,
        ),
        (_, x) => rt_error(format!("Cannot get chunks from {}", x.type_name()), span),
    }
}

pub fn rt_error<T>(message: impl Into<String>, span: &Span) -> RuntimeResult<T> {
    Err(RuntimeError::new(message, span.clone()))
}
