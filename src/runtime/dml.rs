use std::collections::HashMap;

use crate::ast::{
    DeleteStatement, Expr, InsertStatement, SelectIntoTarget, TriggerEvent, TriggerTiming,
    UpdateAssignment, UpdateStatement,
};
use crate::db::{DbError, Row, Table};
use crate::expr::{EvalError, Value, eval};

use super::{Environment, trigger};

pub fn execute_select_into(stmt: &SelectIntoTarget, env: &Environment) -> Result<Value, EvalError> {
    let table = env
        .database()
        .table(&stmt.source)
        .ok_or_else(|| EvalError::TableNotFound(stmt.source.clone()))?;

    let mut matching_rows = Vec::new();
    for row in table.rows() {
        if row_matches_where(stmt.where_clause.as_ref(), env, row)? {
            matching_rows.push(row);
        }
    }

    if matching_rows.is_empty() {
        return Err(EvalError::NoDataFound);
    }

    if matching_rows.len() > 1 {
        return Err(EvalError::TooManyRows);
    }

    if stmt.targets.len() != stmt.select_list.len() {
        return Err(EvalError::ColumnCountMismatch {
            expected: stmt.targets.len(),
            found: stmt.select_list.len(),
        });
    }

    let row_env = row_environment(env, matching_rows[0]);
    let values = evaluate_expressions(&stmt.select_list, &row_env)?;

    for (target, value) in stmt.targets.iter().zip(values.into_iter()) {
        env.assign(target, value);
    }

    Ok(Value::Number(1.0))
}

pub fn execute_insert(stmt: &InsertStatement, env: &Environment) -> Result<Value, EvalError> {
    let values = evaluate_expressions(&stmt.values, env)?;

    let original_table = env
        .database()
        .table(&stmt.table)
        .ok_or_else(|| EvalError::TableNotFound(stmt.table.clone()))?;
    let inserted_row = build_insert_row(&stmt.table, &original_table, stmt, values)?;
    let old_row = blank_row(&original_table);

    let before_row = trigger::fire_row_triggers(
        TriggerTiming::Before,
        TriggerEvent::Insert,
        &stmt.table,
        &old_row,
        &inserted_row,
        env,
    )?;
    let final_row = normalize_row(&stmt.table, &original_table, &before_row)?;

    let mut final_table = original_table.clone();
    final_table.rows_mut().push(final_row.clone());
    env.database()
        .replace_table(&stmt.table, final_table)
        .map_err(map_db_error)?;

    if let Err(err) = trigger::fire_row_triggers(
        TriggerTiming::After,
        TriggerEvent::Insert,
        &stmt.table,
        &old_row,
        &final_row,
        env,
    ) {
        restore_table(env, &stmt.table, &original_table);
        return Err(err);
    }

    Ok(Value::Number(1.0))
}

pub fn execute_update(stmt: &UpdateStatement, env: &Environment) -> Result<Value, EvalError> {
    let original_table = env
        .database()
        .table(&stmt.table)
        .ok_or_else(|| EvalError::TableNotFound(stmt.table.clone()))?;

    for assignment in &stmt.assignments {
        if !original_table
            .columns()
            .iter()
            .any(|column| column == &assignment.column)
        {
            return Err(EvalError::ColumnNotFound {
                table: stmt.table.clone(),
                column: assignment.column.clone(),
            });
        }
    }

    let mut affected = 0usize;
    let mut affected_rows = Vec::new();
    let mut updated_rows = Vec::with_capacity(original_table.rows().len());

    for row in original_table.rows().iter().cloned() {
        let row_env = row_environment(env, &row);
        if row_matches_where(stmt.where_clause.as_ref(), &row_env, &row)? {
            let updated_values = evaluate_update_values(&stmt.assignments, &row_env)?;
            let mut updated_row = row.clone();
            for (column, value) in updated_values {
                updated_row.insert(column, value);
            }

            let updated_row = trigger::fire_row_triggers(
                TriggerTiming::Before,
                TriggerEvent::Update,
                &stmt.table,
                &row,
                &updated_row,
                env,
            )?;
            let updated_row = normalize_row(&stmt.table, &original_table, &updated_row)?;

            affected_rows.push((row.clone(), updated_row.clone()));
            updated_rows.push(updated_row);
            affected += 1;
        } else {
            updated_rows.push(row);
        }
    }

    let mut final_table = original_table.clone();
    *final_table.rows_mut() = updated_rows;
    env.database()
        .replace_table(&stmt.table, final_table)
        .map_err(map_db_error)?;

    for (old_row, new_row) in affected_rows {
        if let Err(err) = trigger::fire_row_triggers(
            TriggerTiming::After,
            TriggerEvent::Update,
            &stmt.table,
            &old_row,
            &new_row,
            env,
        ) {
            restore_table(env, &stmt.table, &original_table);
            return Err(err);
        }
    }

    Ok(Value::Number(affected as f64))
}

pub fn execute_delete(stmt: &DeleteStatement, env: &Environment) -> Result<Value, EvalError> {
    let original_table = env
        .database()
        .table(&stmt.table)
        .ok_or_else(|| EvalError::TableNotFound(stmt.table.clone()))?;

    let mut affected = 0usize;
    let mut affected_rows = Vec::new();
    let mut remaining = Vec::with_capacity(original_table.rows().len());
    let blank_new_row = blank_row(&original_table);

    for row in original_table.rows().iter().cloned() {
        let row_env = row_environment(env, &row);
        if row_matches_where(stmt.where_clause.as_ref(), &row_env, &row)? {
            trigger::fire_row_triggers(
                TriggerTiming::Before,
                TriggerEvent::Delete,
                &stmt.table,
                &row,
                &blank_new_row,
                env,
            )?;
            affected_rows.push(row.clone());
            affected += 1;
        } else {
            remaining.push(row);
        }
    }

    let mut final_table = original_table.clone();
    *final_table.rows_mut() = remaining;
    env.database()
        .replace_table(&stmt.table, final_table)
        .map_err(map_db_error)?;

    for old_row in affected_rows {
        if let Err(err) = trigger::fire_row_triggers(
            TriggerTiming::After,
            TriggerEvent::Delete,
            &stmt.table,
            &old_row,
            &blank_new_row,
            env,
        ) {
            restore_table(env, &stmt.table, &original_table);
            return Err(err);
        }
    }

    Ok(Value::Number(affected as f64))
}

fn evaluate_expressions(expressions: &[Expr], env: &Environment) -> Result<Vec<Value>, EvalError> {
    expressions.iter().map(|expr| eval(expr, env)).collect()
}

fn evaluate_update_values(
    assignments: &[UpdateAssignment],
    env: &Environment,
) -> Result<Vec<(String, Value)>, EvalError> {
    assignments
        .iter()
        .map(|assignment| Ok((assignment.column.clone(), eval(&assignment.value, env)?)))
        .collect()
}

fn build_insert_row(
    table_name: &str,
    table: &Table,
    stmt: &InsertStatement,
    values: Vec<Value>,
) -> Result<Row, EvalError> {
    match &stmt.columns {
        Some(columns) => {
            if columns.len() != values.len() {
                return Err(EvalError::ColumnCountMismatch {
                    expected: columns.len(),
                    found: values.len(),
                });
            }

            let mut row = blank_row(table);
            for (column, value) in columns.iter().cloned().zip(values) {
                if !table
                    .columns()
                    .iter()
                    .any(|table_column| table_column == &column)
                {
                    return Err(EvalError::ColumnNotFound {
                        table: table_name.to_string(),
                        column,
                    });
                }
                row.insert(column, value);
            }

            Ok(row)
        }
        None => {
            if values.len() != table.columns().len() {
                return Err(EvalError::ColumnCountMismatch {
                    expected: table.columns().len(),
                    found: values.len(),
                });
            }

            let mut row = HashMap::new();
            for (column, value) in table.columns().iter().cloned().zip(values) {
                row.insert(column, value);
            }
            Ok(row)
        }
    }
}

fn blank_row(table: &Table) -> Row {
    let mut row = HashMap::new();
    for column in table.columns() {
        row.insert(column.clone(), Value::Null);
    }
    row
}

fn normalize_row(table_name: &str, table: &Table, row: &Row) -> Result<Row, EvalError> {
    for column in row.keys() {
        if !table
            .columns()
            .iter()
            .any(|table_column| table_column == column)
        {
            return Err(EvalError::ColumnNotFound {
                table: table_name.to_string(),
                column: column.clone(),
            });
        }
    }

    let mut normalized = HashMap::new();
    for column in table.columns() {
        normalized.insert(
            column.clone(),
            row.get(column).cloned().unwrap_or(Value::Null),
        );
    }

    Ok(normalized)
}

fn restore_table(env: &Environment, table_name: &str, table: &Table) {
    let _ = env.database().replace_table(table_name, table.clone());
}

fn row_environment(env: &Environment, row: &Row) -> Environment {
    let row_env = env.child();
    for (column, value) in row {
        row_env.set(column.clone(), value.clone());
    }
    row_env
}

fn row_matches_where(
    where_clause: Option<&Expr>,
    env: &Environment,
    row: &Row,
) -> Result<bool, EvalError> {
    match where_clause {
        None => Ok(true),
        Some(expr) => match eval(expr, &row_environment(env, row))? {
            Value::Bool(value) => Ok(value),
            Value::Null => Ok(false),
            other => Err(EvalError::TypeError(format!(
                "WHERE clause must be BOOLEAN, got {}",
                type_name(&other)
            ))),
        },
    }
}

fn map_db_error(err: DbError) -> EvalError {
    match err {
        DbError::TableNotFound(table) => EvalError::TableNotFound(table),
        DbError::ColumnNotFound { table, column } => EvalError::ColumnNotFound { table, column },
        DbError::ColumnCountMismatch { expected, found } => {
            EvalError::ColumnCountMismatch { expected, found }
        }
    }
}

fn type_name(value: &Value) -> &'static str {
    match value {
        Value::Number(_) => "NUMBER",
        Value::Text(_) => "TEXT",
        Value::Bool(_) => "BOOLEAN",
        Value::Date(_) => "DATE",
        Value::Timestamp(_) => "TIMESTAMP",
        Value::DateTime(_) => "DATETIME",
        Value::Record(_) => "RECORD",
        Value::Null => "NULL",
    }
}
