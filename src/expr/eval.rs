use crate::ast::{BinaryOp, Expr, UnaryOp};
use crate::expr::value::Value;
use crate::runtime::env::Environment;
use chrono::Local;

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum EvalError {
    #[error("undefined variable `{0}`")]
    UndefinedVariable(String),

    #[error("unhandled exception `{0}`")]
    UnhandledException(String),

    #[error("table `{0}` not found")]
    TableNotFound(String),

    #[error("column `{column}` not found in table `{table}`")]
    ColumnNotFound { table: String, column: String },

    #[error("insert column count mismatch: expected {expected}, found {found}")]
    ColumnCountMismatch { expected: usize, found: usize },

    #[error("no data found")]
    NoDataFound,

    #[error("too many rows")]
    TooManyRows,

    #[error("cursor `{0}` is already declared")]
    CursorAlreadyDeclared(String),

    #[error("cursor `{0}` is not declared")]
    CursorNotDeclared(String),

    #[error("cursor `{0}` is already open")]
    CursorAlreadyOpen(String),

    #[error("cursor `{0}` is not open")]
    CursorNotOpen(String),

    #[error("trigger `{0}` is already declared")]
    TriggerAlreadyDeclared(String),

    #[error("constraint `{constraint}` already exists on table `{table}`")]
    ConstraintAlreadyExists { table: String, constraint: String },

    #[error("exit used outside of a loop")]
    ExitOutsideLoop,

    #[error("type error: {0}")]
    TypeError(String),

    #[error("division by zero")]
    DivisionByZero,
}

pub fn eval(expr: &Expr, env: &Environment) -> Result<Value, EvalError> {
    match expr {
        Expr::Literal(v) => Ok(v.clone()),

        Expr::Var(name) => env
            .get(name)
            .ok_or_else(|| EvalError::UndefinedVariable(name.clone())),

        Expr::Unary { op, expr } => {
            let value = eval(expr, env)?;
            eval_unary(op, value)
        }

        Expr::Binary { left, op, right } => {
            let lhs = eval(left, env)?;
            let rhs = eval(right, env)?;
            eval_binary(op, lhs, rhs)
        }

        Expr::Sysdate => {
            // Default to DATE when used outside of declaration context
            Ok(Value::Date(Local::now().naive_local().date()))
        }
    }
}

fn eval_unary(op: &UnaryOp, value: Value) -> Result<Value, EvalError> {
    match op {
        UnaryOp::Neg => match value {
            Value::Number(n) => Ok(Value::Number(-n)),
            Value::Null => Ok(Value::Null),
            other => Err(EvalError::TypeError(format!(
                "cannot negate value of type {}",
                type_name(&other)
            ))),
        },

        UnaryOp::Not => match value {
            Value::Bool(b) => Ok(Value::Bool(!b)),
            Value::Null => Ok(Value::Null),
            other => Err(EvalError::TypeError(format!(
                "cannot apply NOT to value of type {}",
                type_name(&other)
            ))),
        },
    }
}

fn eval_binary(op: &BinaryOp, lhs: Value, rhs: Value) -> Result<Value, EvalError> {
    use BinaryOp::*;

    match op {
        Add => match (lhs, rhs) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a + b)),
            (Value::Text(a), Value::Text(b)) => Ok(Value::Text(a + &b)),
            (Value::Null, _) | (_, Value::Null) => Ok(Value::Null),
            (a, b) => Err(EvalError::TypeError(format!(
                "cannot add {} and {}",
                type_name(&a),
                type_name(&b)
            ))),
        },

        Sub => numeric_binop(lhs, rhs, |a, b| a - b),
        Mul => numeric_binop(lhs, rhs, |a, b| a * b),

        Div => match (lhs, rhs) {
            (Value::Number(_), Value::Number(0.0)) => Err(EvalError::DivisionByZero),
            (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a / b)),
            (Value::Null, _) | (_, Value::Null) => Ok(Value::Null),
            (a, b) => Err(EvalError::TypeError(format!(
                "cannot divide {} by {}",
                type_name(&a),
                type_name(&b)
            ))),
        },

        Eq => Ok(Value::Bool(equals(&lhs, &rhs))),
        Neq => Ok(Value::Bool(!equals(&lhs, &rhs))),
        Lt | Lte | Gt | Gte => compare_binary(op, lhs, rhs),

        And => match (lhs, rhs) {
            (Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(a && b)),
            (Value::Null, _) | (_, Value::Null) => Ok(Value::Null),
            (a, b) => Err(EvalError::TypeError(format!(
                "cannot apply AND to {} and {}",
                type_name(&a),
                type_name(&b)
            ))),
        },

        Or => match (lhs, rhs) {
            (Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(a || b)),
            (Value::Null, _) | (_, Value::Null) => Ok(Value::Null),
            (a, b) => Err(EvalError::TypeError(format!(
                "cannot apply OR to {} and {}",
                type_name(&a),
                type_name(&b)
            ))),
        },
    }
}

fn numeric_binop<F>(lhs: Value, rhs: Value, f: F) -> Result<Value, EvalError>
where
    F: FnOnce(f64, f64) -> f64,
{
    match (lhs, rhs) {
        (Value::Number(a), Value::Number(b)) => Ok(Value::Number(f(a, b))),
        (Value::Null, _) | (_, Value::Null) => Ok(Value::Null),
        (a, b) => Err(EvalError::TypeError(format!(
            "expected numbers, got {} and {}",
            type_name(&a),
            type_name(&b)
        ))),
    }
}

fn compare_binary(op: &BinaryOp, lhs: Value, rhs: Value) -> Result<Value, EvalError> {
    use BinaryOp::*;

    match (lhs, rhs) {
        (Value::Number(a), Value::Number(b)) => {
            let result = match op {
                Lt => a < b,
                Lte => a <= b,
                Gt => a > b,
                Gte => a >= b,
                _ => unreachable!(),
            };
            Ok(Value::Bool(result))
        }

        (Value::Text(a), Value::Text(b)) => {
            let result = match op {
                Lt => a < b,
                Lte => a <= b,
                Gt => a > b,
                Gte => a >= b,
                _ => unreachable!(),
            };
            Ok(Value::Bool(result))
        }

        (Value::Null, _) | (_, Value::Null) => Ok(Value::Null),

        (a, b) => Err(EvalError::TypeError(format!(
            "cannot compare {} and {}",
            type_name(&a),
            type_name(&b)
        ))),
    }
}

fn equals(lhs: &Value, rhs: &Value) -> bool {
    match (lhs, rhs) {
        (Value::Null, Value::Null) => true,
        (Value::Number(a), Value::Number(b)) => a == b,
        (Value::Text(a), Value::Text(b)) => a == b,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::Date(a), Value::Date(b)) => a == b,
        (Value::Timestamp(a), Value::Timestamp(b)) => a == b,
        (Value::DateTime(a), Value::DateTime(b)) => a == b,
        (Value::Record(a), Value::Record(b)) => a == b,
        _ => false,
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
