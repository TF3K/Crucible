use super::{CursorQuery, Expr};

#[derive(Clone, Debug, PartialEq)]
pub struct SelectIntoTarget {
    pub targets: Vec<String>,
    pub query: CursorQuery,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InsertStatement {
    pub table: String,
    pub columns: Option<Vec<String>>,
    pub values: Vec<Expr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UpdateAssignment {
    pub column: String,
    pub value: Expr,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UpdateStatement {
    pub table: String,
    pub assignments: Vec<UpdateAssignment>,
    pub where_clause: Option<Expr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DeleteStatement {
    pub table: String,
    pub where_clause: Option<Expr>,
}