use crate::ast::CreateTableStatement;
use crate::ast::{AlterTableAction, AlterTableStatement, DropTableStatement};
use crate::expr::{EvalError, Value};

use super::Environment;
use super::dml::map_db_error;

pub(super) fn execute_create_table(
    stmt: &CreateTableStatement,
    env: &Environment,
) -> Result<Value, EvalError> {
    env.database()
        .create_table(&stmt.table, stmt.columns.clone());
    Ok(Value::Number(1.0))
}

pub(super) fn execute_alter_table(
    stmt: &AlterTableStatement,
    env: &Environment,
) -> Result<Value, EvalError> {
    match &stmt.action {
        AlterTableAction::AddConstraint { name, columns } => env
            .database()
            .alter_table_add_constraint(&stmt.table, name.clone(), columns.clone())
            .map_err(map_db_error)?,
    }

    Ok(Value::Number(1.0))
}

pub(super) fn execute_drop_table(
    stmt: &DropTableStatement,
    env: &Environment,
) -> Result<Value, EvalError> {
    env.database()
        .drop_table(&stmt.table)
        .map_err(map_db_error)?;
    Ok(Value::Number(1.0))
}
