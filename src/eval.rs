use std::{fmt, ops::*};

use crate::{
    ast::*,
    error::{CompileError, CompileResult, Problem},
    ev::Ev,
    lex::Span,
    num::Num,
    op::{Op, Visit},
    types::{ArrayType, AtomType, Ty},
    value::{Array, Atom, Val},
};

pub type EvalResult<T = Ev> = Result<T, CompileError>;

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
    fn expr(&mut self, expr: Expr) -> CompileResult<Ev> {
        self.spans.push(expr.span().clone());
        let res = match expr {
            Expr::Ident(..) => todo!(),
            Expr::Num(num, _) => Ok(num.into()),
            Expr::Char(c, _) => Ok(c.into()),
            Expr::String(s, _) => Ok(Array::String(s).into()),
            Expr::Array(expr) => {
                Ev::from_try_iter(expr.items.into_iter().map(|expr| self.expr(expr)))
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

fn pass<X>(x: X) -> EvalResult
where
    X: Into<Ev>,
{
    Ok(x.into())
}

fn pass_right<W, X>(w: W, x: X) -> EvalResult
where
    X: Into<Ev>,
{
    Ok(x.into())
}

type UnFn<X> = fn(X) -> EvalResult;
type ArrayFn = UnFn<Array>;
type NumFn = UnFn<Num>;
type AtomFn = UnFn<Atom>;
type AtomTyFn = UnFn<AtomType>;
type SheFn = UnFn<Vec<Ty>>;
type ArrayTyFn = UnFn<ArrayType>;

struct Un {
    op: Op,
    array: Option<ArrayFn>,
    num: Option<NumFn>,
    atom: Option<AtomFn>,
    atom_ty: Option<AtomTyFn>,
    array_ty: Option<ArrayTyFn>,
    she: Option<SheFn>,
}

impl Un {
    pub fn new(op: Op) -> Self {
        Un {
            op,
            array: None,
            num: None,
            atom: None,
            atom_ty: None,
            array_ty: None,
            she: None,
        }
    }
    pub fn array(self, f: ArrayFn) -> Self {
        Un {
            array: Some(f),
            ..self
        }
    }
    pub fn num(self, f: NumFn) -> Self {
        Un {
            num: Some(f),
            ..self
        }
    }
    pub fn atom(self, f: AtomFn) -> Self {
        Un {
            atom: Some(f),
            ..self
        }
    }
    pub fn atom_ty(self, f: AtomTyFn) -> Self {
        Un {
            atom_ty: Some(f),
            ..self
        }
    }
    pub fn array_ty(self, f: ArrayTyFn) -> Self {
        Un {
            array_ty: Some(f),
            ..self
        }
    }
    pub fn she(self, f: SheFn) -> Self {
        Un {
            she: Some(f),
            ..self
        }
    }
    pub fn eval(&self, x: Ev) -> EvalResult {
        match (self, x) {
            (Un { num: Some(f), .. }, Ev::Value(Val::Atom(Atom::Num(n)))) => f(n),
            (Un { atom: Some(f), .. }, Ev::Value(Val::Atom(atom))) => f(atom),
            (Un { array: Some(f), .. }, Ev::Value(Val::Array(arr))) => f(arr),
            (
                Un {
                    atom_ty: Some(f), ..
                },
                Ev::Type(Ty::Atom(ty)),
            ) => f(ty),
            (Un { she: Some(f), .. }, Ev::Type(Ty::Array(ArrayType::StaticHetero(tys)))) => f(tys),
            (
                Un {
                    array_ty: Some(f), ..
                },
                Ev::Type(Ty::Array(ty)),
            ) => f(ty),
            (_, x) => self.op.err_un(x),
        }
    }
}

type BinFn<W, X> = fn(W, X) -> EvalResult;
type NumArrayFn = BinFn<Num, Array>;
type NumSheFn = BinFn<Num, Vec<Ty>>;
type NumArrayTyFn = BinFn<Num, ArrayType>;

struct Bin {
    op: Op,
    num_array: Option<NumArrayFn>,
    num_she: Option<NumSheFn>,
    num_array_ty: Option<NumArrayTyFn>,
}

impl Bin {
    pub fn new(op: Op) -> Self {
        Bin {
            op,
            num_array: None,
            num_she: None,
            num_array_ty: None,
        }
    }
    pub fn num_array(self, f: NumArrayFn) -> Self {
        Bin {
            num_array: Some(f),
            ..self
        }
    }
    pub fn num_she(self, f: NumSheFn) -> Self {
        Bin {
            num_she: Some(f),
            ..self
        }
    }
    pub fn num_array_ty(self, f: NumArrayTyFn) -> Self {
        Bin {
            num_array_ty: Some(f),
            ..self
        }
    }
    pub fn eval(&self, w: Ev, x: Ev) -> EvalResult {
        match (self, w, x) {
            (
                Bin {
                    num_array: Some(f), ..
                },
                Ev::Value(Val::Atom(Atom::Num(n))),
                Ev::Value(Val::Array(arr)),
            ) => f(n, arr),
            (
                Bin {
                    num_she: Some(f), ..
                },
                Ev::Value(Val::Atom(Atom::Num(n))),
                Ev::Type(Ty::Array(ArrayType::StaticHetero(tys))),
            ) => f(n, tys),
            (
                Bin {
                    num_array_ty: Some(f),
                    ..
                },
                Ev::Value(Val::Atom(Atom::Num(n))),
                Ev::Type(Ty::Array(ty)),
            ) => f(n, ty),
            (_, w, x) => self.op.err_bin(w, x),
        }
    }
}

impl Visit<Evaler> for Op {
    type Input = Ev;
    type Output = Ev;
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
            Op::Jera => jera::bin(w, x),
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
                Ev::Value(Val::Atom(_)) => 0,
                Ev::Value(Val::Array(arr)) => arr.len(),
                Ev::Type(Ty::Atom(_)) => 0,
                Ev::Type(Ty::Array(arr)) => match arr {
                    ArrayType::Empty => 0,
                    ArrayType::StaticHomo(_, len) => len,
                    ArrayType::StaticHetero(tys) => tys.len(),
                    ArrayType::DynamicHomo(_) => return Ok(AtomType::Num.into()),
                },
            })
            .into()),
            Op::Jera => jera::un(x),
            op => op.err_un(x),
        }
        .map_err(|e| e.at(state.span().clone()))
    }
}

fn pervasize_bin(op: Op, w: Ev, x: Ev, f: fn(Atom, Atom) -> EvalResult<Atom>) -> EvalResult<Ev> {
    match (w, x) {
        (Ev::Value(w), Ev::Value(x)) => pervasize_bin_value(op, w, x, f).map(Into::into),
        (w, x) => op.err_bin(w, x),
    }
}

fn pervasize_bin_value(
    op: Op,
    w: Val,
    x: Val,
    f: fn(Atom, Atom) -> EvalResult<Atom>,
) -> EvalResult<Val> {
    Ok(match (w, x) {
        (Val::Atom(w), Val::Atom(x)) => Val::Atom(f(w, x)?),
        (Val::Atom(w), Val::Array(x)) => Val::Array(Array::from_try_iter(
            x.iter()
                .map(|b| pervasize_bin_value(op, Val::Atom(w), b, f)),
        )?),
        (Val::Array(w), Val::Atom(x)) => Val::Array(Array::from_try_iter(
            w.iter()
                .map(|a| pervasize_bin_value(op, a, Val::Atom(x), f)),
        )?),
        (Val::Array(w), Val::Array(x)) => {
            if w.len() == x.len() {
                Val::Array(Array::from_try_iter(
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

fn pervasive_un(op: Op, x: Ev, f: fn(Atom) -> EvalResult<Atom>) -> EvalResult<Ev> {
    match x {
        Ev::Value(val) => pervasive_un_value(op, val, f).map(Into::into),
        x => op.err_un(x),
    }
}

fn pervasive_un_value(op: Op, x: Val, f: fn(Atom) -> EvalResult<Atom>) -> EvalResult<Val> {
    Ok(match x {
        Val::Atom(atom) => f(atom)?.into(),
        Val::Array(arr) => {
            Array::from_try_iter(arr.into_iter().map(|val| pervasive_un_value(op, val, f)))?.into()
        }
    })
}

mod jera {
    use super::*;
    pub fn un(x: Ev) -> EvalResult {
        Un::new(Op::Jera)
            .atom(pass)
            .atom_ty(pass)
            .array_ty(pass)
            .she(|mut tys| {
                tys.reverse();
                Ok(ArrayType::StaticHetero(tys).into())
            })
            .array(|mut arr| {
                let mut values: Vec<Val> = arr.into_iter().collect();
                values.reverse();
                Ok(Array::from_iter(values).into())
            })
            .eval(x)
    }
    pub fn bin(w: Ev, x: Ev) -> EvalResult<Ev> {
        Bin::new(Op::Jera)
            .num_array(|n, arr| {
                let mut values: Vec<Val> = arr.into_iter().collect();
                if n >= 0 {
                    values.rotate_left(n.into())
                } else {
                    values.rotate_right(-i64::from(n) as usize)
                }
                Ok(Array::from_iter(values).into())
            })
            .num_she(|n, mut tys| {
                if n >= 0 {
                    tys.rotate_left(n.into())
                } else {
                    tys.rotate_right(-i64::from(n) as usize)
                }
                Ok(Ty::from_iter(tys).into())
            })
            .num_array_ty(pass_right)
            .eval(w, x)
    }
}
