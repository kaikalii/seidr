use std::rc::Rc;

use crate::{
    array::Array,
    error::RuntimeResult,
    eval::rt_error,
    lex::Span,
    num::Num,
    op::*,
    value::{Atom, Val},
};

#[derive(Clone)]
pub struct PervadedArray {
    form: PervadedArrayForm,
    op: Rc<PervadedArrayOp>,
}

struct PervadedArrayOp {
    pub per: Pervasive,
    pub span: Span,
}

#[derive(Clone)]
pub enum PervadedArrayForm {
    Un(Array),
    BinLeft(Atom, Array),
    BinRight(Array, Atom),
    Bin(Array, Array),
}

impl PervadedArrayForm {
    pub fn with(self, per: Pervasive, span: Span) -> PervadedArray {
        PervadedArray {
            form: self,
            op: PervadedArrayOp { per, span }.into(),
        }
    }
}

impl PervadedArray {
    pub fn len(&self) -> Option<usize> {
        match &self.form {
            PervadedArrayForm::Un(arr)
            | PervadedArrayForm::BinLeft(_, arr)
            | PervadedArrayForm::BinRight(arr, _) => arr.len(),
            PervadedArrayForm::Bin(a, b) => match (a.len(), b.len()) {
                (Some(a), Some(b)) => Some(a.min(b)),
                (Some(a), None) => Some(a),
                (None, Some(b)) => Some(b),
                (None, None) => None,
            },
        }
    }
    pub fn get(&self, index: usize) -> RuntimeResult<Option<Val>> {
        match &self.form {
            PervadedArrayForm::Un(arr) => {
                let val = if let Some(val) = arr.get(index)? {
                    val.into_owned()
                } else {
                    return Ok(None);
                };
                match val {
                    Val::Atom(atom) => un_pervade_atom(self.op.per, atom, &self.op.span).map(Some),
                    Val::Array(arr) => Ok(Some(
                        Array::from(PervadedArray {
                            form: PervadedArrayForm::Un(arr),
                            op: self.op.clone(),
                        })
                        .into(),
                    )),
                }
            }
            PervadedArrayForm::BinLeft(w, x) => {
                let x = if let Some(x) = x.get(index)? {
                    x.into_owned()
                } else {
                    return Ok(None);
                };
                match x {
                    Val::Atom(x) => bin_pervade_atom(self.op.per, *w, x, &self.op.span).map(Some),
                    Val::Array(x) => Ok(Some(
                        Array::from(PervadedArray {
                            form: PervadedArrayForm::BinLeft(*w, x),
                            op: self.op.clone(),
                        })
                        .into(),
                    )),
                }
            }
            PervadedArrayForm::BinRight(w, x) => {
                let w = if let Some(w) = w.get(index)? {
                    w.into_owned()
                } else {
                    return Ok(None);
                };
                match w {
                    Val::Atom(w) => bin_pervade_atom(self.op.per, w, *x, &self.op.span).map(Some),
                    Val::Array(w) => Ok(Some(
                        Array::from(PervadedArray {
                            form: PervadedArrayForm::BinRight(w, *x),
                            op: self.op.clone(),
                        })
                        .into(),
                    )),
                }
            }
            PervadedArrayForm::Bin(w, x) => {
                let x = if let Some(x) = x.get(index)? {
                    x.into_owned()
                } else {
                    return Ok(None);
                };
                let w = if let Some(w) = w.get(index)? {
                    w.into_owned()
                } else {
                    return Ok(None);
                };
                bin_pervade_val(self.op.per, w, x, &self.op.span).map(Some)
            }
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
        (Pervasive::Math(_), Val::Array(x)) => {
            PervadedArrayForm::Un(x).with(per, span.clone()).into()
        }
    })
}

pub fn bin_pervade_val(per: Pervasive, w: Val, x: Val, span: &Span) -> RuntimeResult {
    Ok(match (w, x) {
        (Val::Atom(w), Val::Atom(x)) => bin_pervade_atom(per, w, x, span)?,
        (Val::Array(w), Val::Array(x)) => {
            PervadedArrayForm::Bin(w, x).with(per, span.clone()).into()
        }
        (Val::Atom(w), Val::Array(x)) => PervadedArrayForm::BinLeft(w, x)
            .with(per, span.clone())
            .into(),
        (Val::Array(w), Val::Atom(x)) => PervadedArrayForm::BinRight(w, x)
            .with(per, span.clone())
            .into(),
    })
}

pub fn un_pervade_atom(per: Pervasive, x: Atom, span: &Span) -> RuntimeResult {
    match (per, x) {
        (Pervasive::Math(MathOp::Add), atom) => Ok(atom.into()),
        (Pervasive::Math(MathOp::Sub), Atom::Num(n)) => Ok((-n).into()),
        (Pervasive::Math(MathOp::Sub), atom) => {
            rt_error(format!("{} cannot be negated", atom.type_name()), span)
        }
        (Pervasive::Math(MathOp::Mul), Atom::Num(n)) => Ok(n.sign().into()),
        (Pervasive::Math(MathOp::Div), Atom::Num(n)) => Ok((Num::Int(1) / n).into()),
        (Pervasive::Math(MathOp::Max), Atom::Num(n)) => Ok(n.ceil().into()),
        (Pervasive::Math(MathOp::Min), Atom::Num(n)) => Ok(n.floor().into()),
        _ => rt_error(format!("{} {} is invalid", per, x.type_name()), span),
    }
}

pub fn bin_pervade_atom(per: Pervasive, w: Atom, x: Atom, span: &Span) -> RuntimeResult {
    match per {
        Pervasive::Math(math) => match (w, x) {
            (Atom::Num(w), Atom::Num(x)) => Ok((match math {
                MathOp::Add => w + x,
                MathOp::Sub => w - x,
                MathOp::Mul => w * x,
                MathOp::Div => w / x,
                MathOp::Max => w.max(x),
                MathOp::Min => w.min(x),
            })
            .into()),
            (Atom::Char(wc), Atom::Num(xn)) => {
                let w = wc as u32;
                let x = u32::from(xn);
                match math {
                    MathOp::Add => Ok(char::from_u32(w.saturating_add(x))
                        .unwrap_or_default()
                        .into()),
                    MathOp::Sub => Ok(char::from_u32(w.saturating_sub(x))
                        .unwrap_or_default()
                        .into()),
                    MathOp::Max => Ok(wc.into()),
                    MathOp::Min => Ok(xn.into()),
                    _ => rt_error(format!("character {} number is invalid", per), span),
                }
            }
            (Atom::Num(w), Atom::Char(x)) if math == MathOp::Add => {
                Ok(char::from_u32((i64::from(w) + x as u32 as i64) as u32)
                    .unwrap_or_default()
                    .into())
            }
            (Atom::Char(w), Atom::Char(x)) if math == MathOp::Sub => {
                Ok((Num::from(w as u32) - Num::from(x as u32)).into())
            }
            _ if math == MathOp::Max => Ok(w.max(x).into()),
            _ if math == MathOp::Min => Ok(w.min(x).into()),
            _ => rt_error(
                format!("{} {} {} is invalid", w.type_name(), per, x.type_name()),
                span,
            ),
        },
        Pervasive::Comparison(comp) => Ok(match comp {
            ComparisonOp::Equal => w == x,
            ComparisonOp::NotEqual => w != x,
            ComparisonOp::Less => w < x,
            ComparisonOp::LessOrEqual => w <= x,
            ComparisonOp::Greater => w > x,
            ComparisonOp::GreaterOrEqual => w >= x,
        }
        .into()),
    }
}
