use crate::{
    array::Array,
    ast::{Bin, OpTreeExpr},
    cwt::{BinVal, UnVal, ValNode},
    error::{RuntimeError, RuntimeResult},
    lex::Span,
    num::Num,
    op::*,
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
                Rune::Jera => Ok(reverse(x, &span)),
                rune => todo!("{}", rune),
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
                Rune::Jera => rotate(w, x, &span),
                rune => todo!("{}", rune),
            },
            val => Ok(val),
        }
    }
}

pub fn un_pervade_val(per: Pervasive, x: Val, span: &Span) -> RuntimeResult {
    Ok(match (per, x) {
        (per, Val::Atom(x)) => un_pervade_atom(per, x, span)?,
        (Pervasive::Comparison(cmp), Val::Array(arr)) => todo!(),
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
                let x = i64::from(x) as u32;
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
                return Ok(char::from_u32(i64::from(w) as u32 + x as u32)
                    .unwrap_or_default()
                    .into())
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
            let mut items: Vec<Val> = arr.iter().collect();
            let n = i64::from(n);
            if n >= 0 {
                items.rotate_left(n as usize);
            } else {
                items.rotate_right((-n) as usize);
            }
            Ok(items.into_iter().collect())
        }
        (Val::Array(ns), x) if ns.len() == 1 => rotate(ns.iter().next().unwrap(), x, span),
        (Val::Array(ns), Val::Array(arr)) => {
            if ns.len() != arr.rank() {
                error(
                    format!(
                        "w's length {} does not match x's rank {}",
                        ns.len(),
                        arr.rank()
                    ),
                    span,
                )
            } else {
                let sub_ns: Array = ns.iter().skip(1).collect();
                let mut items: Vec<Val> = arr
                    .iter()
                    .map(|sub| rotate(sub_ns.clone().into(), sub, span))
                    .collect::<RuntimeResult<_>>()?;
                Ok(items.into_iter().collect())
            }
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
        Val::Array(arr) => {
            let mut items: Vec<Val> = arr.iter().collect();
            items.reverse();
            items.into_iter().collect()
        }
    }
}

fn error(message: impl Into<String>, span: &Span) -> RuntimeResult {
    Err(RuntimeError::new(message, span.clone()))
}
