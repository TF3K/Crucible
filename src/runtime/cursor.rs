use std::collections::{HashMap, HashSet};

use crate::ast::{CursorQuery, Expr, JoinClause, JoinKind, Statement};
use crate::db::{Column, Row};
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
                env.assign(target, value)?;
            }

            Ok(super::block::ExecFlow::Value(Value::Number(1.0)))
        }
        None => {
            for target in targets {
                env.assign(target, Value::Null)?;
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
            Ok(super::block::ExecFlow::Return(value)) => {
                let _ = env.cursors().close(cursor_name);
                return Ok(super::block::ExecFlow::Return(value));
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

pub(super) fn materialize_query(
    query: &CursorQuery,
    env: &Environment,
) -> Result<Vec<Vec<Value>>, EvalError> {
    let sources = load_sources(query, env)?;

    let mut rows = initial_rows(&sources[0]);

    for (index, join) in query.joins.iter().enumerate() {
        let left_sources = &sources[..index + 1];
        let right_source = &sources[index + 1];
        rows = apply_join(rows, left_sources, right_source, join, env)?;
    }

    let final_counts = build_column_counts(&sources);
    let mut results = Vec::new();

    for row_state in rows {
        let row_env = build_row_environment(env, &sources, &row_state, &final_counts);
        if row_matches_where(query.where_clause.as_ref(), &row_env)? {
            results.push(evaluate_expressions(&query.select_list, &row_env)?);
        }
    }

    Ok(results)
}

fn load_sources(query: &CursorQuery, env: &Environment) -> Result<Vec<SourceData>, EvalError> {
    let mut sources = Vec::new();
    let mut seen_qualifiers = HashSet::new();

    for source in std::iter::once(&query.from).chain(query.joins.iter().map(|join| &join.source)) {
        let table = env
            .database()
            .table(&source.table)
            .ok_or_else(|| EvalError::TableNotFound(source.table.clone()))?;

        let qualifier = source.alias.clone().unwrap_or_else(|| source.table.clone());

        if !seen_qualifiers.insert(qualifier.clone()) {
            return Err(EvalError::QueryQualifierAlreadyDeclared(qualifier));
        }

        sources.push(SourceData {
            qualifier,
            columns: table.columns().to_vec(),
            rows: table.rows().to_vec(),
        });
    }

    Ok(sources)
}

fn initial_rows(source: &SourceData) -> Vec<JoinedRow> {
    source
        .rows
        .iter()
        .cloned()
        .map(|row| {
            let mut sources = HashMap::new();
            sources.insert(source.qualifier.clone(), Some(row));
            JoinedRow { sources }
        })
        .collect()
}

fn apply_join(
    left_rows: Vec<JoinedRow>,
    left_sources: &[SourceData],
    right_source: &SourceData,
    join: &JoinClause,
    env: &Environment,
) -> Result<Vec<JoinedRow>, EvalError> {
    let mut active_sources = left_sources.to_vec();
    active_sources.push(right_source.clone());
    let column_counts = build_column_counts(&active_sources);

    let left_qualifiers = left_sources
        .iter()
        .map(|source| source.qualifier.clone())
        .collect::<Vec<_>>();

    let mut next_rows = Vec::new();
    let mut left_matched = vec![false; left_rows.len()];
    let mut right_matched = vec![false; right_source.rows.len()];

    for (left_index, left_row) in left_rows.iter().enumerate() {
        for (right_index, right_value) in right_source.rows.iter().enumerate() {
            let mut candidate = left_row.clone();
            candidate
                .sources
                .insert(right_source.qualifier.clone(), Some(right_value.clone()));

            let row_env = build_row_environment(env, &active_sources, &candidate, &column_counts);
            if evaluate_condition(&join.on, &row_env)? {
                left_matched[left_index] = true;
                right_matched[right_index] = true;
                next_rows.push(candidate);
            }
        }
    }

    match join.kind {
        JoinKind::Inner => {}
        JoinKind::Left => {
            for (left_index, left_row) in left_rows.into_iter().enumerate() {
                if !left_matched[left_index] {
                    let mut row = left_row;
                    row.sources.insert(right_source.qualifier.clone(), None);
                    next_rows.push(row);
                }
            }
        }
        JoinKind::Right => {
            for (right_index, right_value) in right_source.rows.iter().enumerate() {
                if !right_matched[right_index] {
                    let mut row = null_join_row(&left_qualifiers);
                    row.sources
                        .insert(right_source.qualifier.clone(), Some(right_value.clone()));
                    next_rows.push(row);
                }
            }
        }
        JoinKind::Outer => {
            for (left_index, left_row) in left_rows.into_iter().enumerate() {
                if !left_matched[left_index] {
                    let mut row = left_row;
                    row.sources.insert(right_source.qualifier.clone(), None);
                    next_rows.push(row);
                }
            }

            for (right_index, right_value) in right_source.rows.iter().enumerate() {
                if !right_matched[right_index] {
                    let mut row = null_join_row(&left_qualifiers);
                    row.sources
                        .insert(right_source.qualifier.clone(), Some(right_value.clone()));
                    next_rows.push(row);
                }
            }
        }
    }

    Ok(next_rows)
}

fn build_column_counts(sources: &[SourceData]) -> HashMap<String, usize> {
    let mut counts = HashMap::new();

    for source in sources {
        for column in &source.columns {
            *counts.entry(column.name.clone()).or_insert(0) += 1;
        }
    }

    counts
}

fn build_row_environment(
    env: &Environment,
    sources: &[SourceData],
    row_state: &JoinedRow,
    column_counts: &HashMap<String, usize>,
) -> Environment {
    let row_env = env.child();

    for source in sources {
        let row = row_state
            .sources
            .get(&source.qualifier)
            .and_then(|row| row.clone())
            .unwrap_or_else(|| null_row(&source.columns));

        row_env.bind_query_binding(source.qualifier.clone(), Value::Record(row.values.clone()));

        for column in &source.columns {
            let value = row.get(&column.name).cloned().unwrap_or(Value::Null);

            if column_counts.get(&column.name).copied().unwrap_or(0) == 1 {
                row_env.bind_query_column(column.name.clone(), value);
            } else {
                row_env.mark_query_ambiguous(column.name.clone());
            }
        }
    }

    row_env
}

fn evaluate_expressions(expressions: &[Expr], env: &Environment) -> Result<Vec<Value>, EvalError> {
    expressions.iter().map(|expr| eval(expr, env)).collect()
}

fn row_matches_where(where_clause: Option<&Expr>, env: &Environment) -> Result<bool, EvalError> {
    match where_clause {
        None => Ok(true),
        Some(expr) => evaluate_condition(expr, env),
    }
}

fn evaluate_condition(expr: &Expr, env: &Environment) -> Result<bool, EvalError> {
    match eval(expr, env)? {
        Value::Bool(value) => Ok(value),
        Value::Null => Ok(false),
        other => Err(EvalError::TypeError(format!(
            "condition must be BOOLEAN, got {}",
            type_name(&other)
        ))),
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

#[derive(Clone)]
struct JoinedRow {
    sources: HashMap<String, Option<Row>>,
}

fn null_join_row(left_qualifiers: &[String]) -> JoinedRow {
    let mut sources = HashMap::new();

    for qualifier in left_qualifiers {
        sources.insert(qualifier.clone(), None);
    }

    JoinedRow { sources }
}

fn null_row(columns: &[Column]) -> Row {
    Row::blank_from_schema(columns)
}

#[derive(Clone)]
struct SourceData {
    qualifier: String,
    columns: Vec<Column>,
    rows: Vec<Row>,
}
