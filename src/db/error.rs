#[derive(Debug, thiserror::Error, PartialEq)]
pub enum DbError {
    #[error("table `{0}` not found")]
    TableNotFound(String),

    #[error("constraint `{constraint}` already exists on table `{table}`")]
    ConstraintAlreadyExists { table: String, constraint: String },

    #[error("column `{column}` not found in table `{table}`")]
    ColumnNotFound { table: String, column: String },

    #[error("insert column count mismatch: expected {expected}, found {found}")]
    ColumnCountMismatch { expected: usize, found: usize },
}
