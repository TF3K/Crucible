use crate::ast::{CursorQuery, Expr, Statement};
use crate::db::Row;
use crate::expr::{EvalError, Value, eval};

use super::{Environment, block::execute_statements};

pub(super) fn execute_open_cursor(
    name: &str,
    env: &Environment,
) -> Result<super::block::ExecFlow, EvalError> {
    let query = env
        .cursors()
        .query(name)
        .ok_or_else(|| EvalError::CursorNotDeclared(name.to_string()))?;
    let rows = materialize_query(&query, env)?;
    env.cursors().open_rows(name, rows)?;

    Ok(super::block::ExecFlow::NoValue)
}

pub(super) fn execute_fetch_cursor(
    name: &str,
    targets: &[String],
    env: &Environment,
) -> Result<super::block::ExecFlow, EvalError> {
    match env.cursors().next_row(name)? {
        Some(values) => {
            if values.len() != targets.len() {
                return Err(EvalError::ColumnCountMismatch {
                    expected: targets.len(),
                    found: values.len(),
                });
            }

            for (target, value) in targets.iter().zip(values.into_iter()) {
                env.assign(target, value);
            }

            Ok(super::block::ExecFlow::Value(Value::Number(1.0)))
        }
        None => {
            for target in targets {
                env.assign(target, Value::Null);
            }

            Ok(super::block::ExecFlow::Value(Value::Number(0.0)))
        }
    }
}

pub(super) fn execute_close_cursor(
    name: &str,
    env: &Environment,
) -> Result<super::block::ExecFlow, EvalError> {
    env.cursors().close(name)?;
    Ok(super::block::ExecFlow::NoValue)
}

pub(super) fn execute_cursor_for_statement(
    targets: &[String],
    cursor_name: &str,
    body: &[Statement],
    env: &Environment,
) -> Result<super::block::ExecFlow, EvalError> {
    let query = env
        .cursors()
        .query(cursor_name)
        .ok_or_else(|| EvalError::CursorNotDeclared(cursor_name.to_string()))?;
    let rows = materialize_query(&query, env)?;
    env.cursors().open_rows(cursor_name, rows)?;

    let loop_env = env.child();
    let mut last_value: Option<Value> = None;

    loop {
        let next_row = env.cursors().next_row(cursor_name)?;
        let Some(values) = next_row else {
            let _ = env.cursors().close(cursor_name);
            return Ok(last_value
                .map(super::block::ExecFlow::Value)
                .unwrap_or(super::block::ExecFlow::NoValue));
        };

        if values.len() != targets.len() {
            let _ = env.cursors().close(cursor_name);
            return Err(EvalError::ColumnCountMismatch {
                expected: targets.len(),
                found: values.len(),
            });
        }

        for (target, value) in targets.iter().zip(values.into_iter()) {
            loop_env.set(target.clone(), value);
        }

        match execute_statements(body, &loop_env) {
            Ok(super::block::ExecFlow::Value(value)) => last_value = Some(value),
            Ok(super::block::ExecFlow::NoValue) => {}
            Ok(super::block::ExecFlow::Raise(name)) => {
                let _ = env.cursors().close(cursor_name);
                return Ok(super::block::ExecFlow::Raise(name));
            }
            Ok(super::block::ExecFlow::ExitLoop(value)) => {
                let _ = env.cursors().close(cursor_name);
                return Ok(super::block::ExecFlow::Value(
                    value.or(last_value).unwrap_or(Value::Null),
                ));
            }
            Err(err) => {
                let _ = env.cursors().close(cursor_name);
                return Err(err);
            }
        }
    }
}

fn materialize_query(query: &CursorQuery, env: &Environment) -> Result<Vec<Vec<Value>>, EvalError> {
    let table = env
        .database()
        .table(&query.source)
        .ok_or_else(|| EvalError::TableNotFound(query.source.clone()))?;

    let mut rows = Vec::new();
    for row in table.rows() {
        if row_matches_where(query.where_clause.as_ref(), env, row)? {
            let row_env = row_environment(env, row);
            rows.push(evaluate_expressions(&query.select_list, &row_env)?);
        }
    }

    Ok(rows)
}

fn evaluate_expressions(expressions: &[Expr], env: &Environment) -> Result<Vec<Value>, EvalError> {
    expressions.iter().map(|expr| eval(expr, env)).collect()
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
