use super::Statement;

#[derive(Clone, Debug, PartialEq)]
pub enum ExceptionCondition {
    Named(String),
    Others,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExceptionHandler {
    pub condition: ExceptionCondition,
    pub statements: Vec<Statement>,
}