use crate::{lex::Span, op::Op};

#[derive(Debug)]
pub enum Item {
    Expr(Expr),
}

#[derive(Debug)]
pub struct Expr {
    op: Op,
    left: Option<Box<Self>>,
    right: Box<Self>,
    op_span: Span,
    span: Span,
}
