use super::{Expr, Statement};

#[derive(Clone, Debug, PartialEq)]
pub struct IfBranch {
    pub condition: Expr,
    pub statements: Vec<Statement>,
}