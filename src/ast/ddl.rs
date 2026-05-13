#[derive(Clone, Debug, PartialEq)]
pub struct CreateTableStatement {
    pub table: String,
    pub columns: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AlterTableStatement {
    pub table: String,
    pub action: AlterTableAction,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AlterTableAction {
    AddConstraint { name: String, columns: Vec<String> },
}

#[derive(Clone, Debug, PartialEq)]
pub struct DropTableStatement {
    pub table: String,
}
