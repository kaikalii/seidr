use std::{collections::HashMap, iter::repeat};

use crate::{
    array::{Array, EachArray, ReplicateArray, ScanArray, SelectArray, TableArray, ZipForm},
    cwt::{AssignValNode, BinValNode, UnValNode, ValNode},
    error::{RuntimeError, RuntimeResult},
    format::Format,
    function::{Atop, BinModded, Fork, Function, UnModded},
    lex::{Ident, Span},
    num::Num,
    op::*,
    pervade::{bin_pervade_val, un_pervade_val},
    value::{Atom, Val},
};

pub struct Runtime {
    scopes: Vec<Scope>,
}

impl Default for Runtime {
    fn default() -> Self {
        Runtime {
            scopes: vec![Scope::default()],
        }
    }
}

impl Runtime {
    fn scope(&mut self) -> &mut Scope {
        self.scopes.last_mut().expect("scopes are empty")
    }
    fn set(&mut self, name: Ident, val: Val) {
        self.scope().bindings.insert(name, val);
    }
}

#[derive(Default)]
pub struct Params {
    x: Option<Val>,
    w: Option<Val>,
    f: Option<Val>,
    g: Option<Val>,
}

#[derive(Default)]
struct Scope {
    params: Params,
    bindings: HashMap<Ident, Val>,
}

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
            ValNode::Param(param) => todo!(),
            ValNode::Ident(ident) => Ok(rt
                .scope()
                .bindings
                .get(ident)
                .unwrap_or_else(|| panic!("No value stored for `{}`", ident))
                .clone()),
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
            ValNode::Assign(assign) => assign.eval(rt),
        }
    }
}

impl Eval for AssignValNode {
    fn eval(&self, rt: &mut Runtime) -> RuntimeResult {
        let val = self.body.eval(rt)?;
        rt.set(self.name.clone(), val.clone());
        Ok(val)
    }
}

impl Eval for UnValNode {
    fn eval(&self, rt: &mut Runtime) -> RuntimeResult {
        let op = self.op.eval(rt)?;
        let x = self.inner.eval(rt)?;
        eval_un(op, x, &self.span)
    }
}

pub fn eval_un(op: Val, x: Val, span: &Span) -> RuntimeResult {
    match op {
        Val::Atom(Atom::Function(function)) => eval_un_function(function, x, span),
        Val::Atom(Atom::UnMod(m)) => Ok(UnModded { m, f: x }.into()),
        val => Ok(val),
    }
}

fn eval_un_function(function: Function, x: Val, span: &Span) -> RuntimeResult {
    if let Val::Atom(Atom::Function(g)) = x {
        return Ok(Function::Atop(Atop { f: function, g }.into()).into());
    }
    match function {
        Function::Op(Op::Pervasive(Pervasive::Comparison(ComparisonOp::Equal))) => match x {
            Val::Array(arr) => Ok(arr.len().map(Num::from).unwrap_or(Num::INFINIFY).into()),
            Val::Atom(_) => Ok(1i64.into()),
        },
        Function::Op(Op::Pervasive(per)) => un_pervade_val(per, x, span),
        Function::Op(Op::Rune(rune)) => match rune {
            RuneOp::Laguz => Ok(x),
            RuneOp::Jera => reverse(x, span),
            RuneOp::Algiz => range(x, span).map(Val::from),
            RuneOp::Tiwaz => sort(x, span).map(Val::from),
            RuneOp::Sowilo => grade(x, span).map(Val::from),
            RuneOp::Perth => first(x, span),
            rune => rt_error(format!("{} has no unary form", rune), span),
        },
        Function::Op(Op::Other(other)) => match other {
            OtherOp::Match => x.depth(span).map(Into::into),
            other => todo!("{:?}", other),
        },
        Function::Atop(atop) => {
            let lower = eval_un_function(atop.g, x, span)?;
            eval_un_function(atop.f, lower, span)
        }
        Function::Fork(fork) => {
            let left = eval_un(fork.left, x.clone(), span)?;
            let right = eval_un_function(fork.right, x, span)?;
            eval_bin_function(fork.center, left, right, span)
        }
        Function::UnMod(un_mod) => match un_mod.m {
            RuneUnMod::Ingwaz => eval_un(un_mod.f, x, span),
            RuneUnMod::Raido => fold(un_mod.f, None, x, span),
            RuneUnMod::Thurisaz => scan(un_mod.f, None, x, span).map(Val::from),
            RuneUnMod::Othala => eval_bin(un_mod.f, x.clone(), x, span),
            RuneUnMod::Berkanan | RuneUnMod::Wunjo => each_un(un_mod.f, x, span).map(Val::from),
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
    }
}

impl Eval for BinValNode {
    fn eval(&self, rt: &mut Runtime) -> RuntimeResult {
        let op = self.op.eval(rt)?;
        let w = self.left.eval(rt)?;
        let x = self.right.eval(rt)?;
        eval_bin(op, w, x, &self.span)
    }
}

pub fn eval_bin(op: Val, w: Val, x: Val, span: &Span) -> RuntimeResult {
    match op {
        Val::Atom(Atom::Function(function)) => eval_bin_function(function, w, x, span),
        Val::Atom(Atom::BinMod(m)) => Ok(BinModded { m, f: w, g: x }.into()),
        val => Ok(val),
    }
}

fn eval_bin_function(function: Function, w: Val, x: Val, span: &Span) -> RuntimeResult {
    if let Val::Atom(Atom::Function(right)) = x {
        return Ok(Function::Fork(
            Fork {
                left: w,
                center: function,
                right,
            }
            .into(),
        )
        .into());
    }
    match function {
        Function::Op(Op::Pervasive(per)) => bin_pervade_val(per, w, x, span),
        Function::Op(Op::Rune(rune)) => match rune {
            RuneOp::Laguz => Ok(x),
            RuneOp::Fehu => replicate(w, x, span).map(Val::from),
            RuneOp::Jera => rotate(w, x, span),
            RuneOp::Iwaz => Ok(Array::JoinTo(w.into_array().into(), x.into_array().into()).into()),
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
            let lower = eval_bin_function(atop.g, w, x, span)?;
            eval_un_function(atop.f, lower, span)
        }
        Function::Fork(fork) => {
            let left = eval_bin(fork.left, w.clone(), x.clone(), span)?;
            let right = eval_bin_function(fork.right, w, x, span)?;
            eval_bin_function(fork.center, left, right, span)
        }
        Function::UnMod(un_mod) => match un_mod.m {
            RuneUnMod::Ingwaz => eval_bin(un_mod.f, w, x, span),
            RuneUnMod::Raido => fold(un_mod.f, Some(w), x, span),
            RuneUnMod::Thurisaz => scan(un_mod.f, Some(w), x, span).map(Val::from),
            RuneUnMod::Othala => eval_bin(un_mod.f, x, w, span),
            RuneUnMod::Berkanan => each_bin(un_mod.f, w, x, span).map(Val::from),
            RuneUnMod::Wunjo => table(un_mod.f, w, x, span).map(Val::from),
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

fn reverse(x: Val, span: &Span) -> RuntimeResult<Val> {
    match x {
        Val::Atom(_) => Ok(x),
        Val::Array(arr) if arr.len().is_none() => {
            rt_error("Unbounded arrays cannot be reversed", span)
        }
        Val::Array(arr) => Ok(Array::Reverse(arr.into()).into()),
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
        val => rt_error(
            format!("A range cannot be built from {}", val.type_name()),
            span,
        ),
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

pub fn scan(op: Val, w: Option<Val>, x: Val, span: &Span) -> RuntimeResult<Array> {
    match x {
        Val::Array(arr) => Ok(Array::Scan(ScanArray::new(op, arr, w, span.clone()).into())),
        Val::Atom(atom) => rt_error(format!("Attempted to scan over {}", atom.type_name()), span),
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
    match op {
        Val::Atom(Atom::Function(function)) => function_fold_identity(function, span),
        val => Ok(val.clone()),
    }
}

pub fn function_fold_identity(function: &Function, span: &Span) -> RuntimeResult {
    Ok(match function {
        Function::Op(Op::Pervasive(Pervasive::Math(math))) => match math {
            MathOp::Add | MathOp::Sub => 0i64.into(),
            MathOp::Mul | MathOp::Div => 1i64.into(),
            MathOp::Max => (-Num::INFINIFY).into(),
            MathOp::Min => Num::INFINIFY.into(),
            op => return rt_error(format!("{} has no fold identity", op), span),
        },
        Function::Op(Op::Rune(RuneOp::Iwaz)) => Array::empty().into(),
        Function::Atop(atop) => function_fold_identity(&atop.f, span)?,
        Function::Fork(fork) => function_fold_identity(&fork.center, span)?,
        Function::UnMod(un_mod) => match &un_mod.m {
            RuneUnMod::Ingwaz | RuneUnMod::Othala => fold_identity(&un_mod.f, span)?,
            _ => {
                return rt_error(
                    format!("{} has no fold identity", function.as_string()?),
                    span,
                )
            }
        },
        Function::BinMod(bin_mod) => match &bin_mod.m {
            RuneBinMod::Ehwaz | RuneBinMod::Haglaz => fold_identity(&bin_mod.f, span)?,
            _ => {
                return rt_error(
                    format!("{} has no fold identity", function.as_string()?),
                    span,
                )
            }
        },
        function => {
            return rt_error(
                format!("{} has no fold identity", function.as_string()?),
                span,
            )
        }
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
            if arr.len().is_some() {
                let mut items = arr.into_vec()?;
                items.sort_unstable();
                Ok(Array::concrete(items))
            } else {
                rt_error("Unbounded arrays cannot be sorted", span)
            }
        }
        Val::Atom(atom) => rt_error(format!("{} cannot be sorted", atom.type_name()), span),
    }
}

pub fn grade(x: Val, span: &Span) -> RuntimeResult<Array> {
    match x {
        Val::Array(arr) => {
            if arr.len().is_some() {
                let mut items: Vec<(usize, Val)> =
                    arr.into_vec()?.into_iter().enumerate().collect();
                items.sort_unstable_by(|(_, a), (_, b)| a.cmp(b));
                Ok(Array::concrete(items.into_iter().map(|(i, _)| i)))
            } else {
                rt_error("Unbounded arrays cannot be graded", span)
            }
        }
        Val::Atom(atom) => rt_error(format!("{} cannot be graded", atom.type_name()), span),
    }
}

pub fn first(x: Val, span: &Span) -> RuntimeResult {
    Ok(match x {
        x @ Val::Atom(_) => x,
        Val::Array(x) => {
            if let Some(val) = x.get(0)? {
                val.into_owned()
            } else {
                return rt_error("Array has no first element", span);
            }
        }
    })
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

pub fn table(f: Val, w: Val, x: Val, span: &Span) -> RuntimeResult<Array> {
    match (w, x) {
        (Val::Array(w), Val::Array(x)) => {
            Ok(Array::Table(TableArray::new(f, w, x, span.clone()).into()))
        }
        (w, x) => each_bin(f, w, x, span),
    }
}

pub fn rt_error<T>(message: impl Into<String>, span: &Span) -> RuntimeResult<T> {
    Err(RuntimeError::new(message, span.clone()))
}
