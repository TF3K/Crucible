use super::{ParseError, Rule};
use crate::ast::{
    AlterTableAction, AlterTableStatement, CreateTableStatement, DropTableStatement, Statement,
};

pub(super) fn build_create_table_statement(
    pair: pest::iterators::Pair<Rule>,
) -> Result<Statement, ParseError> {
    let mut inner = pair.into_inner();

    let table = inner
        .next()
        .ok_or(ParseError::UnexpectedStructure)?
        .as_str()
        .to_string();
    let columns = build_column_list(inner.next().ok_or(ParseError::UnexpectedStructure)?)?;

    Ok(Statement::CreateTable(CreateTableStatement {
        table,
        columns,
    }))
}

pub(super) fn build_alter_table_statement(
    pair: pest::iterators::Pair<Rule>,
) -> Result<Statement, ParseError> {
    let mut inner = pair.into_inner();

    let table = inner
        .next()
        .ok_or(ParseError::UnexpectedStructure)?
        .as_str()
        .to_string();
    let constraint_name = inner
        .next()
        .ok_or(ParseError::UnexpectedStructure)?
        .as_str()
        .to_string();
    let columns = build_column_list(inner.next().ok_or(ParseError::UnexpectedStructure)?)?;

    Ok(Statement::AlterTable(AlterTableStatement {
        table,
        action: AlterTableAction::AddConstraint {
            name: constraint_name,
            columns,
        },
    }))
}

pub(super) fn build_drop_table_statement(
    pair: pest::iterators::Pair<Rule>,
) -> Result<Statement, ParseError> {
    let table = pair
        .into_inner()
        .next()
        .ok_or(ParseError::UnexpectedStructure)?
        .as_str()
        .to_string();

    Ok(Statement::DropTable(DropTableStatement { table }))
}

fn build_column_list(pair: pest::iterators::Pair<Rule>) -> Result<Vec<String>, ParseError> {
    if pair.as_rule() != Rule::column_list {
        return Err(ParseError::UnexpectedStructure);
    }

    let ident_list = pair
        .into_inner()
        .next()
        .ok_or(ParseError::UnexpectedStructure)?;

    if ident_list.as_rule() != Rule::ident_list {
        return Err(ParseError::UnexpectedStructure);
    }

    let mut columns = Vec::new();
    for child in ident_list.into_inner() {
        if child.as_rule() == Rule::ident {
            columns.push(child.as_str().to_string());
        }
    }

    Ok(columns)
}
