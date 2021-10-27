use crate::num::Num;

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

struct Evaler {}
