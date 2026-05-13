use super::{CursorQuery, Expr, TriggerDeclaration};

#[derive(Clone, Debug, PartialEq)]
pub struct Declaration {
    pub name: String,
    pub type_name: String,
    pub init_value: Option<Expr>,
    pub cursor_query: Option<CursorQuery>,
    pub trigger: Option<TriggerDeclaration>,
}
