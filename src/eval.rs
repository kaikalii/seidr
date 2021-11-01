use crate::{
    array::Array,
    ast::{Bin, OpTreeExpr},
    cwt::{BinVal, UnVal, ValNode},
    error::{RuntimeError, RuntimeResult},
    lex::Span,
    op::*,
    value::{Atom, Val},
};

#[derive(Default)]
pub struct Runtime {}

impl Runtime {}

pub trait Eval {
    fn eval(&self, rt: &mut Runtime) -> RuntimeResult<Val>;
}

impl Eval for Val {
    fn eval(&self, _: &mut Runtime) -> RuntimeResult<Val> {
        Ok(self.clone())
    }
}

impl Eval for ValNode {
    fn eval(&self, rt: &mut Runtime) -> RuntimeResult<Val> {
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
    fn eval(&self, rt: &mut Runtime) -> RuntimeResult<Val> {
        todo!()
    }
}

impl Eval for BinVal {
    fn eval(&self, rt: &mut Runtime) -> RuntimeResult<Val> {
        let op = self.op.eval(rt)?;
        let span = self.op.span.clone();
        let w = self.w.eval(rt)?;
        let x = self.x.eval(rt)?;
        Ok(match op {
            Val::Atom(Atom::Op(Op::Pervasive(per))) => pervade_val(per, w, x, &span)?,
            Val::Atom(Atom::Op(Op::Rune(rune))) => todo!("{}", rune),
            val => val,
        })
    }
}

pub fn pervade_val(per: Pervasive, w: Val, x: Val, span: &Span) -> RuntimeResult<Val> {
    Ok(match (w, x) {
        (Val::Atom(w), Val::Atom(x)) => pervade_atom(per, w, x, span)?,
        (Val::Array(w), Val::Array(x)) => w
            .pervade_with(&x, span, |w, x| pervade_val(per, w, x, span))?
            .into(),
        (w, Val::Array(x)) => x.pervade(|x| pervade_val(per, w.clone(), x, span))?.into(),
        (Val::Array(w), x) => w.pervade(|w| pervade_val(per, w, x.clone(), span))?.into(),
    })
}

pub fn pervade_atom(per: Pervasive, w: Atom, x: Atom, span: &Span) -> RuntimeResult<Val> {
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
    Err(RuntimeError::new(
        format!("{} {} {} is invalid", w, per, x),
        span.clone(),
    ))
}
