use std::{collections::BTreeSet, fmt};

use crate::{
    array::Array,
    ast::*,
    error::{CompileError, CompileResult},
    lex::Span,
    op::*,
    value::Val,
};

#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TypeSet(BTreeSet<TypeConst>);

impl TypeSet {
    pub fn join(self, other: Self, span: &Span) -> CompileResult<Self> {
        let intersection: BTreeSet<TypeConst> = self.0.intersection(&other.0).cloned().collect();
        if intersection.is_empty() {
            Err(CompileError::UnreconcilableTypes(self, other).at(span.clone()))
        } else {
            Ok(TypeSet(intersection))
        }
    }
}

impl fmt::Debug for TypeSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, ty) in self.0.iter().enumerate() {
            if i > 0 {
                write!(f, " | ")?;
            }
            ty.fmt(f)?;
        }
        Ok(())
    }
}

impl<T> From<T> for TypeSet
where
    T: Into<TypeConst>,
{
    fn from(ty: T) -> Self {
        [ty].into()
    }
}

impl<T, const N: usize> From<[T; N]> for TypeSet
where
    T: Into<TypeConst>,
{
    fn from(types: [T; N]) -> Self {
        TypeSet(BTreeSet::from(types.map(Into::into)))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum TypeConst {
    Type(Type),
    Const(Val),
}

impl From<AtomType> for TypeConst {
    fn from(at: AtomType) -> Self {
        TypeConst::Type(at.into())
    }
}

impl From<ArrayType> for TypeConst {
    fn from(at: ArrayType) -> Self {
        TypeConst::Type(at.into())
    }
}

impl<V> From<V> for TypeConst
where
    V: Into<Val>,
{
    fn from(val: V) -> Self {
        TypeConst::Const(val.into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Type {
    Atom(AtomType),
    Array(ArrayType),
}

impl From<AtomType> for Type {
    fn from(at: AtomType) -> Self {
        Type::Atom(at)
    }
}

impl From<ArrayType> for Type {
    fn from(at: ArrayType) -> Self {
        Type::Array(at)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum AtomType {
    Num,
    Char,
    Op,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ArrayType {
    StaticHomo(TypeSet, usize),
    StaticHetero(Vec<TypeSet>),
    Dynamic(TypeSet),
}

pub struct TypeChecker {}

pub trait TypeCheck {
    fn check(&self, checker: &mut TypeChecker) -> CompileResult<TypeSet>;
}

impl TypeCheck for ExprItem {
    fn check(&self, checker: &mut TypeChecker) -> CompileResult<TypeSet> {
        self.expr.check(checker)
    }
}

impl TypeCheck for OpExpr {
    fn check(&self, checker: &mut TypeChecker) -> CompileResult<TypeSet> {
        match self {
            OpExpr::Val(expr) => expr.check(checker),
            OpExpr::Un(expr) => expr.check(checker),
            OpExpr::Bin(expr) => expr.check(checker),
        }
    }
}

impl TypeCheck for ValExpr {
    fn check(&self, checker: &mut TypeChecker) -> CompileResult<TypeSet> {
        match self {
            ValExpr::Num(n) => Ok((**n).into()),
            ValExpr::Char(c) => Ok((**c).into()),
            ValExpr::String(s) => Ok(Array::string(s.data.clone()).into()),
            ValExpr::Array(expr) => {
                let mut types: Vec<TypeSet> = expr
                    .items
                    .iter()
                    .map(|item| item.check(checker))
                    .collect::<CompileResult<_>>()?;
                Ok(if types.is_empty() {
                    Array::concrete(<[Val; 0]>::default()).into()
                } else if types.windows(2).fold(true, |acc, win| win[0] == win[1]) {
                    let len = types.len();
                    ArrayType::StaticHomo(types.swap_remove(0), len).into()
                } else {
                    ArrayType::StaticHetero(types).into()
                })
            }
            ValExpr::Parened(expr) => expr.check(checker),
            ValExpr::Mod(expr) => todo!("{:?}", expr),
        }
    }
}

impl TypeCheck for UnOpExpr {
    fn check(&self, checker: &mut TypeChecker) -> CompileResult<TypeSet> {
        match self.x.check(checker)? {}
    }
}

impl TypeCheck for BinOpExpr {
    fn check(&self, checker: &mut TypeChecker) -> CompileResult<TypeSet> {
        todo!()
    }
}

impl TypeCheck for ArrayItemExpr {
    fn check(&self, checker: &mut TypeChecker) -> CompileResult<TypeSet> {
        match self {
            ArrayItemExpr::Val(expr) => expr.check(checker),
            ArrayItemExpr::Function(expr) => todo!("{:?}", expr),
        }
    }
}
