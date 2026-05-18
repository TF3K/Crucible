use crate::ast::{Block, ExceptionCondition, ExceptionHandler, Expr, IfBranch, Statement};
use crate::expr::{EvalError, Value, eval};
use chrono::Local;

use super::{Environment, cursor, ddl, dml, routine};

pub fn execute_block(block: &Block, env: &Environment) -> Result<Value, EvalError> {
    let local_env = env.child();

    match execute_block_in_scope(block, &local_env)? {
        ExecFlow::Value(value) => Ok(value),
        ExecFlow::NoValue => Ok(Value::Null),
        ExecFlow::ExitLoop(_) => Err(EvalError::ExitOutsideLoop),
        ExecFlow::Raise(name) => Err(EvalError::UnhandledException(name)),
        ExecFlow::Return(_) => Err(EvalError::ReturnOutsideRoutine),
    }
}

/// Execute a block and collect the evaluated value for each top-level statement in the
/// block's `BEGIN` section. Declarations are executed first (same as `execute_block`).
/// Returns a vector with one entry per statement: `Value::Null` for statements that
/// produce no value, or the produced `Value`.
pub fn execute_block_collect_values(
    block: &Block,
    env: &Environment,
) -> Result<Vec<Value>, EvalError> {
    let local_env = env.child();

    // Execute declarations first (same behavior as execute_block)
    if let Err(err) = execute_declarations(block, &local_env) {
        if let Some(handler) =
            find_exception_handler(&block.exception_handlers, &ExceptionSignal::Runtime)
        {
            // If a handler exists, execute its statements and continue
            match execute_statements(&handler.statements, &local_env)? {
                ExecFlow::Return(_) => return Err(EvalError::ReturnOutsideRoutine),
                _ => {}
            }
        } else {
            return Err(err);
        }
    }

    let mut results: Vec<Value> = Vec::new();

    for statement in &block.statements {
        match execute_statement(statement, &local_env)? {
            ExecFlow::Value(value) => results.push(value),
            ExecFlow::NoValue => results.push(Value::Null),
            ExecFlow::Raise(name) => return Err(EvalError::UnhandledException(name)),
            ExecFlow::Return(_) => return Err(EvalError::ReturnOutsideRoutine),
            ExecFlow::ExitLoop(opt) => {
                results.push(opt.unwrap_or(Value::Null));
                return Ok(results);
            }
        }
    }

    Ok(results)
}

pub(super) fn execute_block_in_scope(
    block: &Block,
    env: &Environment,
) -> Result<ExecFlow, EvalError> {
    if let Err(err) = execute_declarations(block, env) {
        if let Some(handler) =
            find_exception_handler(&block.exception_handlers, &ExceptionSignal::Runtime)
        {
            return execute_statements(&handler.statements, env);
        }

        return Err(err);
    }

    match execute_statements(&block.statements, env) {
        Ok(ExecFlow::Raise(name)) => {
            if let Some(handler) =
                find_exception_handler(&block.exception_handlers, &ExceptionSignal::Named(&name))
            {
                execute_statements(&handler.statements, env)
            } else {
                Ok(ExecFlow::Raise(name))
            }
        }
        Ok(flow) => Ok(flow),
        Err(err) => {
            if let Some(handler) =
                find_exception_handler(&block.exception_handlers, &ExceptionSignal::Runtime)
            {
                execute_statements(&handler.statements, env)
            } else {
                Err(err)
            }
        }
    }
}

fn execute_statement(statement: &Statement, env: &Environment) -> Result<ExecFlow, EvalError> {
    match statement {
        Statement::Block(block) => {
            let child_env = env.child();
            execute_block_in_scope(block, &child_env)
        }

        Statement::If {
            condition,
            then_branch,
            elsif_branches,
            else_branch,
        } => execute_if_statement(condition, then_branch, elsif_branches, else_branch, env),

        Statement::While { condition, body } => execute_while_statement(condition, body, env),

        Statement::Loop { body } => execute_loop_statement(body, env),

        Statement::For {
            variable,
            start,
            end,
            body,
        } => execute_for_statement(variable, start, end, body, env),

        Statement::Exit { condition } => execute_exit_statement(condition, env),

        Statement::OpenCursor { name } => cursor::execute_open_cursor(name, env),

        Statement::FetchCursor { name, targets } => {
            cursor::execute_fetch_cursor(name, targets, env)
        }

        Statement::CloseCursor { name } => cursor::execute_close_cursor(name, env),

        Statement::CursorFor {
            targets,
            cursor: cursor_name,
            body,
        } => cursor::execute_cursor_for_statement(targets, cursor_name, body, env),

        Statement::SelectInto(stmt) => Ok(ExecFlow::Value(dml::execute_select_into(stmt, env)?)),

        Statement::Insert(stmt) => Ok(ExecFlow::Value(dml::execute_insert(stmt, env)?)),

        Statement::Update(stmt) => Ok(ExecFlow::Value(dml::execute_update(stmt, env)?)),

        Statement::Delete(stmt) => Ok(ExecFlow::Value(dml::execute_delete(stmt, env)?)),

        Statement::CreateTable(stmt) => Ok(ExecFlow::Value(ddl::execute_create_table(stmt, env)?)),

        Statement::AlterTable(stmt) => Ok(ExecFlow::Value(ddl::execute_alter_table(stmt, env)?)),

        Statement::DropTable(stmt) => Ok(ExecFlow::Value(ddl::execute_drop_table(stmt, env)?)),

        Statement::Call { name, args } => {
            let value = routine::execute_routine_call(
                name,
                args,
                env,
                routine::RoutineCallContext::Statement,
            )?;

            Ok(match value {
                Some(value) => ExecFlow::Value(value),
                None => ExecFlow::NoValue,
            })
        }

        Statement::Return(value) => Ok(ExecFlow::Return(match value {
            Some(expr) => Some(eval(expr, env)?),
            None => None,
        })),

        Statement::Expression(expr) => Ok(ExecFlow::Value(eval(expr, env)?)),
        Statement::Assignment { name, value } => {
            let result = eval(value, env)?;
            env.assign(name, result.clone())?;
            Ok(ExecFlow::Value(result))
        }
        Statement::Raise { name } => Ok(ExecFlow::Raise(name.clone())),

        Statement::Commit => {
            env.database().commit_transaction();
            Ok(ExecFlow::NoValue)
        }

        Statement::Rollback => {
            env.database().rollback_transaction();
            Ok(ExecFlow::NoValue)
        }
    }
}

fn execute_if_statement(
    condition: &Expr,
    then_branch: &[Statement],
    elsif_branches: &[IfBranch],
    else_branch: &Option<Vec<Statement>>,
    env: &Environment,
) -> Result<ExecFlow, EvalError> {
    if evaluate_condition(condition, env)? {
        return execute_statements(then_branch, env);
    }

    for branch in elsif_branches {
        if evaluate_condition(&branch.condition, env)? {
            return execute_statements(&branch.statements, env);
        }
    }

    if let Some(statements) = else_branch {
        return execute_statements(statements, env);
    }

    Ok(ExecFlow::NoValue)
}

fn execute_while_statement(
    condition: &Expr,
    body: &[Statement],
    env: &Environment,
) -> Result<ExecFlow, EvalError> {
    let mut last_value: Option<Value> = None;

    while evaluate_condition(condition, env)? {
        match execute_statements(body, env)? {
            ExecFlow::Value(value) => last_value = Some(value),
            ExecFlow::NoValue => {}
            ExecFlow::Raise(name) => return Ok(ExecFlow::Raise(name)),
            ExecFlow::Return(value) => return Ok(ExecFlow::Return(value)),
            ExecFlow::ExitLoop(value) => {
                return Ok(ExecFlow::Value(value.or(last_value).unwrap_or(Value::Null)));
            }
        }
    }

    Ok(last_value.map(ExecFlow::Value).unwrap_or(ExecFlow::NoValue))
}

fn execute_loop_statement(body: &[Statement], env: &Environment) -> Result<ExecFlow, EvalError> {
    let mut last_value: Option<Value> = None;

    loop {
        match execute_statements(body, env)? {
            ExecFlow::Value(value) => last_value = Some(value),
            ExecFlow::NoValue => {}
            ExecFlow::Raise(name) => return Ok(ExecFlow::Raise(name)),
            ExecFlow::Return(value) => return Ok(ExecFlow::Return(value)),
            ExecFlow::ExitLoop(value) => {
                return Ok(ExecFlow::Value(value.or(last_value).unwrap_or(Value::Null)));
            }
        }
    }
}

fn execute_for_statement(
    variable: &str,
    start: &Expr,
    end: &Expr,
    body: &[Statement],
    env: &Environment,
) -> Result<ExecFlow, EvalError> {
    let start = expect_number(eval(start, env)?)?;
    let end = expect_number(eval(end, env)?)?;

    let loop_env = env.child();
    let mut last_value: Option<Value> = None;
    let mut current = start;

    while current <= end {
        loop_env.set(variable.to_string(), Value::Number(current));

        match execute_statements(body, &loop_env)? {
            ExecFlow::Value(value) => last_value = Some(value),
            ExecFlow::NoValue => {}
            ExecFlow::Raise(name) => return Ok(ExecFlow::Raise(name)),
            ExecFlow::Return(value) => return Ok(ExecFlow::Return(value)),
            ExecFlow::ExitLoop(value) => {
                return Ok(ExecFlow::Value(value.or(last_value).unwrap_or(Value::Null)));
            }
        }

        current += 1.0;
    }

    Ok(last_value.map(ExecFlow::Value).unwrap_or(ExecFlow::NoValue))
}

fn execute_exit_statement(
    condition: &Option<Expr>,
    env: &Environment,
) -> Result<ExecFlow, EvalError> {
    match condition {
        Some(expr) => {
            if evaluate_condition(expr, env)? {
                Ok(ExecFlow::ExitLoop(None))
            } else {
                Ok(ExecFlow::NoValue)
            }
        }
        None => Ok(ExecFlow::ExitLoop(None)),
    }
}

pub(super) fn execute_statements(
    statements: &[Statement],
    env: &Environment,
) -> Result<ExecFlow, EvalError> {
    let mut last_value: Option<Value> = None;

    for statement in statements {
        match execute_statement(statement, env)? {
            ExecFlow::Value(value) => last_value = Some(value),
            ExecFlow::NoValue => {}
            ExecFlow::Raise(name) => {
                return Ok(ExecFlow::Raise(name));
            }
            ExecFlow::Return(value) => {
                return Ok(ExecFlow::Return(value));
            }
            ExecFlow::ExitLoop(value) => {
                return Ok(ExecFlow::ExitLoop(value.or(last_value)));
            }
        }
    }

    Ok(last_value.map(ExecFlow::Value).unwrap_or(ExecFlow::NoValue))
}

fn evaluate_condition(expr: &Expr, env: &Environment) -> Result<bool, EvalError> {
    match eval(expr, env)? {
        Value::Bool(value) => Ok(value),
        Value::Null => Ok(false),
        other => Err(EvalError::TypeError(format!(
            "IF condition must be BOOLEAN, got {}",
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

fn expect_number(value: Value) -> Result<f64, EvalError> {
    match value {
        Value::Number(number) => Ok(number),
        other => Err(EvalError::TypeError(format!(
            "FOR bounds must be NUMBER, got {}",
            type_name(&other)
        ))),
    }
}

fn execute_declarations(block: &Block, env: &Environment) -> Result<(), EvalError> {
    for declaration in &block.declarations {
        if let Some(trigger) = &declaration.trigger {
            env.triggers().declare(trigger.clone())?;
            continue;
        }

        if let Some(routine) = &declaration.routine {
            env.routines().declare(routine.clone())?;
            continue;
        }

        if let Some(cursor_query) = &declaration.cursor_query {
            env.cursors()
                .declare(declaration.name.clone(), cursor_query.clone())?;
            continue;
        }

        if declaration.type_name.eq_ignore_ascii_case("EXCEPTION") {
            continue;
        }

        let value = match &declaration.init_value {
            Some(expr) => {
                // Handle SYSDATE specially based on the declared type
                match expr {
                    Expr::Sysdate => {
                        let now_local = Local::now();
                        match declaration.type_name.to_uppercase().as_str() {
                            "DATE" => Value::Date(now_local.naive_local().date()),
                            "TIMESTAMP" => Value::Timestamp(now_local.naive_local()),
                            "DATETIME" => Value::DateTime(
                                now_local.with_timezone(&chrono::FixedOffset::east_opt(0).unwrap()),
                            ),
                            _ => eval(expr, env)?,
                        }
                    }
                    _ => eval(expr, env)?,
                }
            }
            None => Value::Null,
        };
        env.declare(
            declaration.name.clone(),
            value,
            declaration.type_name.clone(),
        )?;
    }

    Ok(())
}

fn find_exception_handler<'a>(
    handlers: &'a [ExceptionHandler],
    signal: &ExceptionSignal<'_>,
) -> Option<&'a ExceptionHandler> {
    handlers
        .iter()
        .find(|handler| handler_matches(handler, signal))
}

fn handler_matches(handler: &ExceptionHandler, signal: &ExceptionSignal<'_>) -> bool {
    match (&handler.condition, signal) {
        (ExceptionCondition::Others, _) => true,
        (ExceptionCondition::Named(name), ExceptionSignal::Named(signal_name)) => {
            name == signal_name
        }
        (ExceptionCondition::Named(_), ExceptionSignal::Runtime) => false,
    }
}

enum ExceptionSignal<'a> {
    Named(&'a str),
    Runtime,
}

#[derive(Debug)]
pub(super) enum ExecFlow {
    Value(Value),
    NoValue,
    ExitLoop(Option<Value>),
    Raise(String),
    Return(Option<Value>),
}
