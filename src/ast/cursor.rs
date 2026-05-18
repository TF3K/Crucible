use super::Expr;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JoinKind {
    Inner,
    Left,
    Right,
    Outer,
}

#[derive(Clone, Debug, PartialEq)]
pub struct QuerySource {
    pub table: String,
    pub alias: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct JoinClause {
    pub kind: JoinKind,
    pub source: QuerySource,
    pub on: Expr,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CursorQuery {
    pub select_list: Vec<Expr>,
    pub from: QuerySource,
    pub joins: Vec<JoinClause>,
    pub where_clause: Option<Expr>,
}
