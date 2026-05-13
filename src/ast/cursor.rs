use super::Expr;

#[derive(Clone, Debug, PartialEq)]
pub struct CursorQuery {
    pub select_list: Vec<Expr>,
    pub source: String,
    pub where_clause: Option<Expr>,
}