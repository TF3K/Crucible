use super::{ParseError, Rule, expr::build_expr};
use crate::ast::{CursorQuery, Declaration, Statement};

pub(super) fn build_cursor_declaration(
    pair: pest::iterators::Pair<Rule>,
) -> Result<Declaration, ParseError> {
    let mut inner = pair.into_inner();

    let name = inner
        .next()
        .ok_or(ParseError::UnexpectedStructure)?
        .as_str()
        .to_string();
    let query = build_cursor_query(inner.next().ok_or(ParseError::UnexpectedStructure)?)?;

    Ok(Declaration {
        name,
        type_name: "CURSOR".to_string(),
        init_value: None,
        cursor_query: Some(query),
        trigger: None,
    })
}

pub(super) fn build_open_statement(
    pair: pest::iterators::Pair<Rule>,
) -> Result<Statement, ParseError> {
    let name = pair
        .into_inner()
        .next()
        .ok_or(ParseError::UnexpectedStructure)?
        .as_str()
        .to_string();

    Ok(Statement::OpenCursor { name })
}

pub(super) fn build_fetch_statement(
    pair: pest::iterators::Pair<Rule>,
) -> Result<Statement, ParseError> {
    let mut inner = pair.into_inner();
    let name = inner
        .next()
        .ok_or(ParseError::UnexpectedStructure)?
        .as_str()
        .to_string();
    let targets = build_target_list(inner.next().ok_or(ParseError::UnexpectedStructure)?)?;

    Ok(Statement::FetchCursor { name, targets })
}

pub(super) fn build_close_statement(
    pair: pest::iterators::Pair<Rule>,
) -> Result<Statement, ParseError> {
    let name = pair
        .into_inner()
        .next()
        .ok_or(ParseError::UnexpectedStructure)?
        .as_str()
        .to_string();

    Ok(Statement::CloseCursor { name })
}

pub(super) fn build_cursor_for_statement(
    pair: pest::iterators::Pair<Rule>,
) -> Result<Statement, ParseError> {
    let mut inner = pair.into_inner();
    let targets = build_target_list(inner.next().ok_or(ParseError::UnexpectedStructure)?)?;
    let cursor = inner
        .next()
        .ok_or(ParseError::UnexpectedStructure)?
        .as_str()
        .to_string();

    let mut body = Vec::new();
    for child in inner {
        body.push(super::statement::build_statement(child)?);
    }

    Ok(Statement::CursorFor {
        targets,
        cursor,
        body,
    })
}

fn build_target_list(pair: pest::iterators::Pair<Rule>) -> Result<Vec<String>, ParseError> {
    if pair.as_rule() != Rule::target_list {
        return Err(ParseError::UnexpectedStructure);
    }

    let mut targets = Vec::new();
    for child in pair.into_inner() {
        if child.as_rule() == Rule::ident {
            targets.push(child.as_str().to_string());
        }
    }

    Ok(targets)
}

fn build_cursor_query(pair: pest::iterators::Pair<Rule>) -> Result<CursorQuery, ParseError> {
    let mut inner = pair.into_inner();
    let select_list = build_expression_list(inner.next().ok_or(ParseError::UnexpectedStructure)?)?;
    let _ignored_target = inner
        .next()
        .ok_or(ParseError::UnexpectedStructure)?
        .as_str()
        .to_string();
    let source = inner
        .next()
        .ok_or(ParseError::UnexpectedStructure)?
        .as_str()
        .to_string();

    let where_clause = match inner.next() {
        Some(pair) if pair.as_rule() == Rule::where_clause => Some(build_where_clause(pair)?),
        None => None,
        _ => return Err(ParseError::UnexpectedStructure),
    };

    Ok(CursorQuery {
        select_list,
        source,
        where_clause,
    })
}

fn build_expression_list(
    pair: pest::iterators::Pair<Rule>,
) -> Result<Vec<crate::ast::Expr>, ParseError> {
    let mut expressions = Vec::new();

    if pair.as_rule() != Rule::select_list {
        return Err(ParseError::UnexpectedStructure);
    }

    for child in pair.into_inner() {
        if child.as_rule() == Rule::expression {
            expressions.push(build_expr(child)?);
        }
    }

    Ok(expressions)
}

fn build_where_clause(pair: pest::iterators::Pair<Rule>) -> Result<crate::ast::Expr, ParseError> {
    let expr = pair
        .into_inner()
        .next()
        .ok_or(ParseError::UnexpectedStructure)?;
    build_expr(expr)
}
