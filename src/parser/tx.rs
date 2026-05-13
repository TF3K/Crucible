use super::{ParseError, Rule};
use crate::ast::Statement;

pub(super) fn build_commit_statement(
    _: pest::iterators::Pair<Rule>,
) -> Result<Statement, ParseError> {
    Ok(Statement::Commit)
}

pub(super) fn build_rollback_statement(
    _: pest::iterators::Pair<Rule>,
) -> Result<Statement, ParseError> {
    Ok(Statement::Rollback)
}