use super::{ParseError, Rule, block::build_block, expr::build_expr};
use crate::ast::{Declaration, Expr, RoutineDeclaration, RoutineKind, RoutineParameter, Statement};

pub(super) fn build_routine_declaration(
    pair: pest::iterators::Pair<Rule>,
) -> Result<Declaration, ParseError> {
    let rule = pair.as_rule();
    let mut inner = pair.into_inner();

    let name = inner
        .next()
        .ok_or(ParseError::UnexpectedStructure)?
        .as_str()
        .to_string();
    let parameters =
        build_routine_parameters(inner.next().ok_or(ParseError::UnexpectedStructure)?)?;

    let (kind, return_type, body_pair) = match rule {
        Rule::procedure_declaration => {
            let body = inner.next().ok_or(ParseError::UnexpectedStructure)?;
            (RoutineKind::Procedure, None, body)
        }
        Rule::function_declaration => {
            let return_type = inner
                .next()
                .ok_or(ParseError::UnexpectedStructure)?
                .as_str()
                .to_string();
            let body = inner.next().ok_or(ParseError::UnexpectedStructure)?;
            (RoutineKind::Function, Some(return_type), body)
        }
        _ => return Err(ParseError::UnexpectedStructure),
    };

    let body = build_block(body_pair)?;
    let routine = RoutineDeclaration {
        kind: kind.clone(),
        name: name.clone(),
        parameters,
        return_type: return_type.clone(),
        body: Box::new(body),
    };

    Ok(Declaration {
        name,
        type_name: match kind {
            RoutineKind::Procedure => "PROCEDURE".to_string(),
            RoutineKind::Function => "FUNCTION".to_string(),
        },
        init_value: None,
        cursor_query: None,
        trigger: None,
        routine: Some(routine),
    })
}

pub(super) fn build_call_expr(pair: pest::iterators::Pair<Rule>) -> Result<Expr, ParseError> {
    let (name, args) = build_call(pair)?;
    Ok(Expr::Call { name, args })
}

pub(super) fn build_routine_call_statement(
    pair: pest::iterators::Pair<Rule>,
) -> Result<Statement, ParseError> {
    let (name, args) = build_call(pair)?;
    Ok(Statement::Call { name, args })
}

pub(super) fn build_return_statement(
    pair: pest::iterators::Pair<Rule>,
) -> Result<Statement, ParseError> {
    let mut inner = pair.into_inner();
    let value = match inner.next() {
        Some(expr_pair) => Some(build_expr(expr_pair)?),
        None => None,
    };

    Ok(Statement::Return(value))
}

fn build_call(pair: pest::iterators::Pair<Rule>) -> Result<(String, Vec<Expr>), ParseError> {
    let mut inner = pair.into_inner();
    let name = inner
        .next()
        .ok_or(ParseError::UnexpectedStructure)?
        .as_str()
        .to_string();
    let args = build_call_arguments(inner.next().ok_or(ParseError::UnexpectedStructure)?)?;

    Ok((name, args))
}

fn build_routine_parameters(
    pair: pest::iterators::Pair<Rule>,
) -> Result<Vec<RoutineParameter>, ParseError> {
    if pair.as_rule() != Rule::routine_parameters {
        return Err(ParseError::UnexpectedStructure);
    }

    let mut parameters = Vec::new();
    for child in pair.into_inner() {
        if child.as_rule() == Rule::routine_parameter {
            parameters.push(build_routine_parameter(child)?);
        }
    }

    Ok(parameters)
}

fn build_routine_parameter(
    pair: pest::iterators::Pair<Rule>,
) -> Result<RoutineParameter, ParseError> {
    let mut inner = pair.into_inner();
    let name = inner
        .next()
        .ok_or(ParseError::UnexpectedStructure)?
        .as_str()
        .to_string();
    let type_name = inner
        .next()
        .ok_or(ParseError::UnexpectedStructure)?
        .as_str()
        .to_string();

    Ok(RoutineParameter { name, type_name })
}

fn build_call_arguments(pair: pest::iterators::Pair<Rule>) -> Result<Vec<Expr>, ParseError> {
    if pair.as_rule() != Rule::call_arguments {
        return Err(ParseError::UnexpectedStructure);
    }

    let mut arguments = Vec::new();
    for child in pair.into_inner() {
        if child.as_rule() == Rule::expression_list {
            for expr_pair in child.into_inner() {
                if expr_pair.as_rule() == Rule::expression {
                    arguments.push(build_expr(expr_pair)?);
                }
            }
        }
    }

    Ok(arguments)
}
