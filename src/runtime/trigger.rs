use crate::ast::{TriggerDeclaration, TriggerEvent, TriggerTiming};
use crate::db::Row;
use crate::expr::{EvalError, Value};

use super::{block, Environment};

pub(super) fn fire_row_triggers(
    timing: TriggerTiming,
    event: TriggerEvent,
    table: &str,
    old_row: &Row,
    new_row: &Row,
    env: &Environment,
) -> Result<Row, EvalError> {
    let mut current_new = new_row.clone();

    for trigger in env.triggers().matching(table, timing, event) {
        current_new = execute_trigger(&trigger, old_row, &current_new, env)?;
    }

    Ok(current_new)
}

fn execute_trigger(
    trigger: &TriggerDeclaration,
    old_row: &Row,
    new_row: &Row,
    env: &Environment,
) -> Result<Row, EvalError> {
    let trigger_env = env.child();
    trigger_env.set("OLD", Value::Record(old_row.clone()));
    trigger_env.set("NEW", Value::Record(new_row.clone()));

    match block::execute_block_in_scope(&trigger.body, &trigger_env)? {
        block::ExecFlow::Value(_) | block::ExecFlow::NoValue => {}
        block::ExecFlow::ExitLoop(_) => return Err(EvalError::ExitOutsideLoop),
        block::ExecFlow::Raise(name) => return Err(EvalError::UnhandledException(name)),
    }

    match trigger_env.get("NEW") {
        Some(Value::Record(row)) => Ok(row),
        Some(value) => Err(EvalError::TypeError(format!(
            "trigger `NEW` must be RECORD, got {}",
            value_type_name(&value)
        ))),
        None => Err(EvalError::TypeError(
            "trigger `NEW` is missing".to_string(),
        )),
    }
}

fn value_type_name(value: &Value) -> &'static str {
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