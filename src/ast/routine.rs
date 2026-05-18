use super::Block;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RoutineKind {
    Procedure,
    Function,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoutineParameter {
    pub name: String,
    pub type_name: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RoutineDeclaration {
    pub kind: RoutineKind,
    pub name: String,
    pub parameters: Vec<RoutineParameter>,
    pub return_type: Option<String>,
    pub body: Box<Block>,
}