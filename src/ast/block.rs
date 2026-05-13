use super::{Declaration, ExceptionHandler, Statement};

#[derive(Clone, Debug, PartialEq)]
pub struct Block {
    pub declarations: Vec<Declaration>,
    pub statements: Vec<Statement>,
    pub exception_handlers: Vec<ExceptionHandler>,
}
