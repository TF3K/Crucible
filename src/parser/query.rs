use super::{ParseError, Rule, expr::build_expr};
use crate::ast::{CursorQuery, Expr, JoinClause, JoinKind, QuerySource};

pub(super) fn build_query_source(
    pair: pest::iterators::Pair<Rule>,
) -> Result<QuerySource, ParseError> {
    if pair.as_rule() != Rule::table_source {
        return Err(ParseError::UnexpectedStructure);
    }

    let mut inner = pair.into_inner();
    let table = inner
        .next()
        .ok_or(ParseError::UnexpectedStructure)?
        .as_str()
        .to_string();

    let alias = match inner.next() {
        Some(alias_pair) if alias_pair.as_rule() == Rule::table_alias => Some(
            alias_pair
                .into_inner()
                .next()
                .ok_or(ParseError::UnexpectedStructure)?
                .as_str()
                .to_string(),
        ),
        None => None,
        _ => return Err(ParseError::UnexpectedStructure),
    };

    Ok(QuerySource { table, alias })
}

pub(super) fn build_from_clause(
    pair: pest::iterators::Pair<Rule>,
) -> Result<(QuerySource, Vec<JoinClause>), ParseError> {
    if pair.as_rule() != Rule::from_clause {
        return Err(ParseError::UnexpectedStructure);
    }

    let mut inner = pair.into_inner();
    let from = build_query_source(inner.next().ok_or(ParseError::UnexpectedStructure)?)?;
    let mut joins = Vec::new();

    for child in inner {
        joins.push(build_join_clause(child)?);
    }

    Ok((from, joins))
}

pub(super) fn build_join_clause(
    pair: pest::iterators::Pair<Rule>,
) -> Result<JoinClause, ParseError> {
    if pair.as_rule() != Rule::join_clause {
        return Err(ParseError::UnexpectedStructure);
    }

    let mut inner = pair.into_inner();
    let first = inner.next().ok_or(ParseError::UnexpectedStructure)?;

    let (kind, source_pair) = match first.as_rule() {
        Rule::join_kind => (
            parse_join_kind(first)?,
            inner.next().ok_or(ParseError::UnexpectedStructure)?,
        ),
        Rule::table_source => (JoinKind::Inner, first),
        _ => return Err(ParseError::UnexpectedStructure),
    };

    let source = build_query_source(source_pair)?;
    let on = build_expr(inner.next().ok_or(ParseError::UnexpectedStructure)?)?;

    Ok(JoinClause { kind, source, on })
}

pub(super) fn build_query_spec(
    select_list: Vec<Expr>,
    from: QuerySource,
    joins: Vec<JoinClause>,
    where_clause: Option<Expr>,
) -> CursorQuery {
    CursorQuery {
        select_list,
        from,
        joins,
        where_clause,
    }
}

fn parse_join_kind(pair: pest::iterators::Pair<Rule>) -> Result<JoinKind, ParseError> {
    if pair.as_rule() != Rule::join_kind {
        return Err(ParseError::UnexpectedStructure);
    }

    let text = pair.as_str().to_uppercase();
    let first = text
        .split_whitespace()
        .next()
        .ok_or(ParseError::UnexpectedStructure)?;

    match first {
        "INNER" => Ok(JoinKind::Inner),
        "LEFT" => Ok(JoinKind::Left),
        "RIGHT" => Ok(JoinKind::Right),
        "OUTER" => Ok(JoinKind::Outer),
        _ => Err(ParseError::UnexpectedStructure),
    }
}
