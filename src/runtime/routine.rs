use crate::ast::{Expr, RoutineKind};
use crate::expr::{DataType, EvalError, Value, eval};

use super::{Environment, block};

pub(crate) enum RoutineCallContext {
    Expression,
    Statement,
}

pub(crate) fn execute_routine_call(
    name: &str,
    args: &[Expr],
    env: &Environment,
    context: RoutineCallContext,
) -> Result<Option<Value>, EvalError> {
    let routine = env
        .routines()
        .get(name)
        .ok_or_else(|| EvalError::RoutineNotDeclared(name.to_string()))?;

    let mut evaluated_args = Vec::new();
    for arg in args {
        evaluated_args.push(eval(arg, env)?);
    }

    if evaluated_args.len() != routine.parameters.len() {
        return Err(EvalError::RoutineArgumentCountMismatch {
            routine: routine.name.clone(),
            expected: routine.parameters.len(),
            found: evaluated_args.len(),
        });
    }

    let call_env = env.child();
    for (parameter, value) in routine.parameters.iter().zip(evaluated_args.into_iter()) {
        call_env.declare(parameter.name.clone(), value, parameter.type_name.clone())?;
    }

    let flow = block::execute_block_in_scope(&routine.body, &call_env)?;

    match routine.kind {
        RoutineKind::Procedure => match context {
            RoutineCallContext::Expression => Err(EvalError::TypeError(format!(
                "routine `{}` is a procedure and cannot be used in an expression",
                routine.name
            ))),
            RoutineCallContext::Statement => Ok(None),
        },
        RoutineKind::Function => {
            let value = match flow {
                block::ExecFlow::Value(value) => value,
                block::ExecFlow::NoValue => Value::Null,
                block::ExecFlow::Return(value) => value.unwrap_or(Value::Null),
                block::ExecFlow::ExitLoop(_) => return Err(EvalError::ExitOutsideLoop),
                block::ExecFlow::Raise(name) => return Err(EvalError::UnhandledException(name)),
            };

            validate_return_value(&routine.name, routine.return_type.as_deref(), &value)?;

            Ok(Some(value))
        }
    }
}

fn validate_return_value(
    routine_name: &str,
    return_type: Option<&str>,
    value: &Value,
) -> Result<(), EvalError> {
    let Some(return_type) = return_type else {
        return Ok(());
    };

    let expected_type = DataType::from_name(return_type).ok_or_else(|| {
        EvalError::TypeError(format!(
            "unknown return type `{}` for routine `{}`",
            return_type, routine_name
        ))
    })?;

    if expected_type.accept(value) {
        return Ok(());
    }

    Err(EvalError::TypeError(format!(
        "routine `{}` expects {}, got {}",
        routine_name,
        expected_type.name(),
        value.type_name()
    )))
}
