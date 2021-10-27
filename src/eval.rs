use std::{fmt, ops::*};

use crate::{
    ast::*,
    error::{CompileErrorKind, CompileResult, Problem},
    lex::Span,
    num::Num,
    op::{Op, Visit},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Num(Num),
    Char(char),
    Array(Array),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Array {
    String(String),
    List(Vec<Value>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Const(Value),
    Num,
    Char,
    Array(Box<ArrayType>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArrayType {
    Empty,
    StaticHomo(Type, usize),
    DynamicHomo(Type),
    StaticHetero(Vec<Type>),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Num(n) => n.fmt(f),
            Value::Char(c) => write!(f, "{:?}", c),
            Value::Array(arr) => arr.fmt(f),
        }
    }
}

impl fmt::Display for Array {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Array::String(s) => write!(f, "{:?}", s),
            Array::List(items) => {
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    item.fmt(f)?;
                }
                write!(f, "]")
            }
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Const(val) => val.fmt(f),
            Type::Num => "num".fmt(f),
            Type::Char => "char".fmt(f),
            Type::Array(ty) => ty.fmt(f),
        }
    }
}

impl fmt::Display for ArrayType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArrayType::Empty => "[]".fmt(f),
            ArrayType::StaticHomo(ty, n) => write!(f, "[{}; {}]", ty, n),
            ArrayType::DynamicHomo(ty) => write!(f, "[{}]", ty),
            ArrayType::StaticHetero(tys) => f.debug_list().entries(tys).finish(),
        }
    }
}

impl From<ArrayType> for Type {
    fn from(at: ArrayType) -> Self {
        Type::Array(Box::new(at))
    }
}

impl Value {
    pub fn ty(&self) -> Type {
        match self {
            Value::Num(_) => Type::Num,
            Value::Char(_) => Type::Char,
            Value::Array(arr) => match arr {
                Array::String(s) => ArrayType::StaticHomo(Type::Char, s.chars().count()),
                Array::List(items) => {
                    let mut types: Vec<Type> = items.iter().map(Value::ty).collect();
                    if types.windows(2).all(|win| win[0] == win[1]) {
                        let len = types.len();
                        if let Some(ty) = types.pop() {
                            ArrayType::StaticHomo(ty, len)
                        } else {
                            ArrayType::Empty
                        }
                    } else {
                        ArrayType::StaticHetero(types)
                    }
                }
            }
            .into(),
        }
    }
}

pub struct Evaler {
    span: Span,
}

impl Default for Evaler {
    fn default() -> Self {
        Evaler { span: Span::dud() }
    }
}

impl Evaler {
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
    fn expr(&mut self, expr: Expr) -> CompileResult<Type> {
        match expr {
            Expr::Ident(..) => todo!(),
            Expr::Num(num, _) => Ok(Type::Const(Value::Num(num))),
            Expr::Un(expr) => {
                let inner = self.expr(expr.inner)?;
                expr.op.visit_un(inner, self)
            }
            Expr::Bin(expr) => {
                let left = self.expr(expr.left)?;
                let right = self.expr(expr.right)?;
                expr.op.visit_bin(left, right, self)
            }
        }
    }
}

impl Visit<Evaler> for Op {
    type Input = Type;
    type Output = Type;
    type Error = Problem;
    fn visit_bin(
        &self,
        left: Self::Input,
        right: Self::Input,
        state: &mut Evaler,
    ) -> Result<Self::Output, Self::Error> {
        match self {
            Op::Add => bin_math(*self, left, right, &state.span, Num::add),
            Op::Sub => bin_math(*self, left, right, &state.span, Num::sub),
            Op::Mul => bin_math(*self, left, right, &state.span, Num::mul),
            Op::Div => bin_math(*self, left, right, &state.span, Num::div),
            op => todo!("{}", op),
        }
    }
    fn visit_un(
        &self,
        inner: Self::Input,
        state: &mut Evaler,
    ) -> Result<Self::Output, Self::Error> {
        todo!()
    }
}

fn bin_math(
    op: Op,
    left: Type,
    right: Type,
    span: &Span,
    f: fn(Num, Num) -> Num,
) -> CompileResult<Type> {
    Ok(match (left, right) {
        (Type::Const(Value::Num(a)), Type::Const(Value::Num(b))) => {
            Type::Const(Value::Num(f(a, b)))
        }
        (left, right) => {
            return Err(CompileErrorKind::IncompatibleBinTypes(op, left, right).at(span.clone()))
        }
    })
}
