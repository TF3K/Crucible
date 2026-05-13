use super::{ParseError, Rule, expr::build_expr};
use crate::ast::{
    DeleteStatement, Expr, InsertStatement, SelectIntoTarget, Statement, UpdateAssignment,
    UpdateStatement,
};

pub(super) fn build_select_into_statement(
    pair: pest::iterators::Pair<Rule>,
) -> Result<Statement, ParseError> {
    let mut inner = pair.into_inner();

    let select_list = build_expression_list(inner.next().ok_or(ParseError::UnexpectedStructure)?)?;
    let targets = build_ident_list(inner.next().ok_or(ParseError::UnexpectedStructure)?)?;
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

    Ok(Statement::SelectInto(SelectIntoTarget {
        targets,
        select_list,
        source,
        where_clause,
    }))
}

pub(super) fn build_insert_statement(
    pair: pest::iterators::Pair<Rule>,
) -> Result<Statement, ParseError> {
    let mut inner = pair.into_inner();

    let table = inner
        .next()
        .ok_or(ParseError::UnexpectedStructure)?
        .as_str()
        .to_string();

    let next = inner.next().ok_or(ParseError::UnexpectedStructure)?;
    let (columns, values_pair) = match next.as_rule() {
        Rule::column_list => {
            let columns = build_ident_list(next)?;
            let values_pair = inner.next().ok_or(ParseError::UnexpectedStructure)?;
            (Some(columns), values_pair)
        }
        Rule::expression_list => (None, next),
        _ => return Err(ParseError::UnexpectedStructure),
    };

    let values = build_expression_list(values_pair)?;

    Ok(Statement::Insert(InsertStatement {
        table,
        columns,
        values,
    }))
}

pub(super) fn build_update_statement(
    pair: pest::iterators::Pair<Rule>,
) -> Result<Statement, ParseError> {
    let mut inner = pair.into_inner();

    let table = inner
        .next()
        .ok_or(ParseError::UnexpectedStructure)?
        .as_str()
        .to_string();
    let assignments =
        build_update_assignments(inner.next().ok_or(ParseError::UnexpectedStructure)?)?;

    let where_clause = match inner.next() {
        Some(pair) if pair.as_rule() == Rule::where_clause => Some(build_where_clause(pair)?),
        None => None,
        _ => return Err(ParseError::UnexpectedStructure),
    };

    Ok(Statement::Update(UpdateStatement {
        table,
        assignments,
        where_clause,
    }))
}

pub(super) fn build_delete_statement(
    pair: pest::iterators::Pair<Rule>,
) -> Result<Statement, ParseError> {
    let mut inner = pair.into_inner();

    let table = inner
        .next()
        .ok_or(ParseError::UnexpectedStructure)?
        .as_str()
        .to_string();

    let where_clause = match inner.next() {
        Some(pair) if pair.as_rule() == Rule::where_clause => Some(build_where_clause(pair)?),
        None => None,
        _ => return Err(ParseError::UnexpectedStructure),
    };

    Ok(Statement::Delete(DeleteStatement {
        table,
        where_clause,
    }))
}

fn build_ident_list(pair: pest::iterators::Pair<Rule>) -> Result<Vec<String>, ParseError> {
    let mut idents = Vec::new();

    match pair.as_rule() {
        Rule::target_list | Rule::ident_list => {
            for child in pair.into_inner() {
                if child.as_rule() == Rule::ident {
                    idents.push(child.as_str().to_string());
                }
            }
            Ok(idents)
        }
        Rule::column_list => {
            let inner = pair
                .into_inner()
                .next()
                .ok_or(ParseError::UnexpectedStructure)?;
            build_ident_list(inner)
        }
        _ => Err(ParseError::UnexpectedStructure),
    }
}

fn build_expression_list(pair: pest::iterators::Pair<Rule>) -> Result<Vec<Expr>, ParseError> {
    let mut expressions = Vec::new();

    match pair.as_rule() {
        Rule::select_list | Rule::expression_list => {
            for child in pair.into_inner() {
                if child.as_rule() == Rule::expression {
                    expressions.push(build_expr(child)?);
                }
            }
            Ok(expressions)
        }
        _ => Err(ParseError::UnexpectedStructure),
    }
}

fn build_update_assignments(
    pair: pest::iterators::Pair<Rule>,
) -> Result<Vec<UpdateAssignment>, ParseError> {
    let mut assignments = Vec::new();

    if pair.as_rule() != Rule::update_assignment_list {
        return Err(ParseError::UnexpectedStructure);
    }

    for child in pair.into_inner() {
        if child.as_rule() != Rule::update_assignment {
            continue;
        }

        let mut inner = child.into_inner();
        let column = inner
            .next()
            .ok_or(ParseError::UnexpectedStructure)?
            .as_str()
            .to_string();
        inner.next().ok_or(ParseError::UnexpectedStructure)?;
        let value = build_expr(inner.next().ok_or(ParseError::UnexpectedStructure)?)?;
        assignments.push(UpdateAssignment { column, value });
    }

    Ok(assignments)
}

fn build_where_clause(pair: pest::iterators::Pair<Rule>) -> Result<Expr, ParseError> {
    let expr = pair
        .into_inner()
        .next()
        .ok_or(ParseError::UnexpectedStructure)?;
    build_expr(expr)
}
