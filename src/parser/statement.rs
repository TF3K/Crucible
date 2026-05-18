use super::{
    ParseError, Rule, block::build_block, ddl, dml, expr::build_expr, routine,
    tx,
};
use crate::ast::{ExceptionCondition, ExceptionHandler, IfBranch, Statement};

pub(super) fn build_statement(pair: pest::iterators::Pair<Rule>) -> Result<Statement, ParseError> {
    match pair.as_rule() {
        Rule::if_statement => build_if_statement(pair),

        Rule::while_statement => build_while_statement(pair),

        Rule::loop_statement => build_loop_statement(pair),

        Rule::range_for_statement => build_for_statement(pair),

        Rule::cursor_for_statement => super::cursor::build_cursor_for_statement(pair),

        Rule::exit_statement => build_exit_statement(pair),

        Rule::raise_statement => build_raise_statement(pair),

        Rule::commit_statement => tx::build_commit_statement(pair),

        Rule::rollback_statement => tx::build_rollback_statement(pair),

        Rule::open_statement => super::cursor::build_open_statement(pair),

        Rule::fetch_statement => super::cursor::build_fetch_statement(pair),

        Rule::close_statement => super::cursor::build_close_statement(pair),

        Rule::select_into_statement => dml::build_select_into_statement(pair),

        Rule::insert_statement => dml::build_insert_statement(pair),

        Rule::update_statement => dml::build_update_statement(pair),

        Rule::delete_statement => dml::build_delete_statement(pair),

        Rule::create_table_statement => ddl::build_create_table_statement(pair),

        Rule::alter_table_statement => ddl::build_alter_table_statement(pair),

        Rule::drop_table_statement => ddl::build_drop_table_statement(pair),

        Rule::routine_call_statement => routine::build_routine_call_statement(pair),

        Rule::return_statement => routine::build_return_statement(pair),

        Rule::block => Ok(Statement::Block(Box::new(build_block(pair)?))),

        Rule::assignment => {
            let mut inner = pair.into_inner();
            let name = inner
                .next()
                .ok_or(ParseError::UnexpectedStructure)?
                .as_str()
                .to_string();
            let value = build_expr(inner.next().ok_or(ParseError::UnexpectedStructure)?)?;
            Ok(Statement::Assignment { name, value })
        }
        Rule::expression_stmt => {
            let inner = pair
                .into_inner()
                .next()
                .ok_or(ParseError::UnexpectedStructure)?;
            let expr = build_expr(inner)?;
            Ok(Statement::Expression(expr))
        }
        _ => Err(ParseError::UnexpectedStructure),
    }
}

fn build_if_statement(pair: pest::iterators::Pair<Rule>) -> Result<Statement, ParseError> {
    let mut inner = pair.into_inner();

    let condition = build_expr(inner.next().ok_or(ParseError::UnexpectedStructure)?)?;

    let mut then_branch = Vec::new();
    let mut elsif_branches = Vec::new();
    let mut else_branch = None;
    let mut in_then_branch = true;

    for child in inner {
        match child.as_rule() {
            Rule::if_statement
            | Rule::while_statement
            | Rule::loop_statement
            | Rule::range_for_statement
            | Rule::cursor_for_statement
            | Rule::exit_statement
            | Rule::raise_statement
            | Rule::commit_statement
            | Rule::rollback_statement
            | Rule::open_statement
            | Rule::fetch_statement
            | Rule::close_statement
            | Rule::select_into_statement
            | Rule::insert_statement
            | Rule::update_statement
            | Rule::delete_statement
            | Rule::create_table_statement
            | Rule::alter_table_statement
            | Rule::drop_table_statement
            | Rule::routine_call_statement
            | Rule::return_statement
            | Rule::block
            | Rule::assignment
            | Rule::expression_stmt => {
                if !in_then_branch {
                    return Err(ParseError::UnexpectedStructure);
                }

                then_branch.push(build_statement(child)?);
            }
            Rule::elsif_branch => {
                in_then_branch = false;
                elsif_branches.push(build_elsif_branch(child)?);
            }
            Rule::else_branch => {
                in_then_branch = false;
                if else_branch.is_some() {
                    return Err(ParseError::UnexpectedStructure);
                }
                else_branch = Some(build_else_branch(child)?);
            }
            _ => return Err(ParseError::UnexpectedStructure),
        }
    }

    Ok(Statement::If {
        condition,
        then_branch,
        elsif_branches,
        else_branch,
    })
}

fn build_while_statement(pair: pest::iterators::Pair<Rule>) -> Result<Statement, ParseError> {
    let mut inner = pair.into_inner();

    let condition = build_expr(inner.next().ok_or(ParseError::UnexpectedStructure)?)?;
    let mut body = Vec::new();

    for child in inner {
        body.push(build_statement(child)?);
    }

    Ok(Statement::While { condition, body })
}

fn build_loop_statement(pair: pest::iterators::Pair<Rule>) -> Result<Statement, ParseError> {
    let mut body = Vec::new();

    for child in pair.into_inner() {
        body.push(build_statement(child)?);
    }

    Ok(Statement::Loop { body })
}

fn build_for_statement(pair: pest::iterators::Pair<Rule>) -> Result<Statement, ParseError> {
    let mut inner = pair.into_inner();

    let variable = inner
        .next()
        .ok_or(ParseError::UnexpectedStructure)?
        .as_str()
        .to_string();
    let start = build_expr(inner.next().ok_or(ParseError::UnexpectedStructure)?)?;
    let end = build_expr(inner.next().ok_or(ParseError::UnexpectedStructure)?)?;

    let mut body = Vec::new();
    for child in inner {
        body.push(build_statement(child)?);
    }

    Ok(Statement::For {
        variable,
        start,
        end,
        body,
    })
}

fn build_exit_statement(pair: pest::iterators::Pair<Rule>) -> Result<Statement, ParseError> {
    let mut inner = pair.into_inner();
    let condition = match inner.next() {
        Some(expr_pair) => Some(build_expr(expr_pair)?),
        None => None,
    };

    Ok(Statement::Exit { condition })
}

fn build_raise_statement(pair: pest::iterators::Pair<Rule>) -> Result<Statement, ParseError> {
    let name = pair
        .into_inner()
        .next()
        .ok_or(ParseError::UnexpectedStructure)?
        .as_str()
        .to_string();

    Ok(Statement::Raise { name })
}

pub(super) fn build_elsif_branch(
    pair: pest::iterators::Pair<Rule>,
) -> Result<IfBranch, ParseError> {
    let mut inner = pair.into_inner();

    let condition = build_expr(inner.next().ok_or(ParseError::UnexpectedStructure)?)?;

    let mut statements = Vec::new();
    for child in inner {
        statements.push(build_statement(child)?);
    }

    Ok(IfBranch {
        condition,
        statements,
    })
}

pub(super) fn build_else_branch(
    pair: pest::iterators::Pair<Rule>,
) -> Result<Vec<Statement>, ParseError> {
    let mut statements = Vec::new();

    for child in pair.into_inner() {
        statements.push(build_statement(child)?);
    }

    Ok(statements)
}

pub(super) fn build_exception_handler(
    pair: pest::iterators::Pair<Rule>,
) -> Result<ExceptionHandler, ParseError> {
    let mut inner = pair.into_inner();
    let selector = inner.next().ok_or(ParseError::UnexpectedStructure)?;
    let selector = match selector.as_rule() {
        Rule::exception_selector => selector
            .into_inner()
            .next()
            .ok_or(ParseError::UnexpectedStructure)?,
        _ => selector,
    };

    let condition = match selector.as_rule() {
        Rule::OTHERS => ExceptionCondition::Others,
        Rule::ident => ExceptionCondition::Named(selector.as_str().to_string()),
        _ => return Err(ParseError::UnexpectedStructure),
    };

    let mut statements = Vec::new();
    for child in inner {
        statements.push(build_statement(child)?);
    }

    Ok(ExceptionHandler {
        condition,
        statements,
    })
}
