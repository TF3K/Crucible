use crate::expr::value::Value;

use super::{BinaryOp, UnaryOp};

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Literal(Value),
    Var(String),
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
}