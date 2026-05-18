use super::{
    ParseError, Rule, cursor::build_cursor_declaration, expr::build_expr,
    routine::build_routine_declaration, statement::build_statement,
    trigger::build_trigger_declaration,
};
use crate::ast::{Block, Declaration};

pub(super) fn build_block(pair: pest::iterators::Pair<Rule>) -> Result<Block, ParseError> {
    let mut declarations = Vec::new();
    let mut statements = Vec::new();
    let mut exception_handlers = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::declare_section => {
                for decl_pair in inner.into_inner() {
                    declarations.push(build_declaration(decl_pair)?);
                }
            }
            Rule::begin_section => {
                for stmt_pair in inner.into_inner() {
                    statements.push(build_statement(stmt_pair)?);
                }
            }
            Rule::exception_section => {
                for handler_pair in inner.into_inner() {
                    exception_handlers
                        .push(super::statement::build_exception_handler(handler_pair)?);
                }
            }
            _ => {}
        }
    }

    Ok(Block {
        declarations,
        statements,
        exception_handlers,
    })
}

fn build_declaration(pair: pest::iterators::Pair<Rule>) -> Result<Declaration, ParseError> {
    let declaration = pair
        .into_inner()
        .next()
        .ok_or(ParseError::UnexpectedStructure)?;

    match declaration.as_rule() {
        Rule::variable_declaration => {
            let mut inner = declaration.into_inner();

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

            let init_value = match inner.next() {
                Some(expr_pair) => Some(build_expr(expr_pair)?),
                None => None,
            };

            Ok(Declaration {
                name,
                type_name,
                init_value,
                cursor_query: None,
                trigger: None,
                routine: None,
            })
        }
        Rule::exception_declaration => {
            let name = declaration
                .into_inner()
                .next()
                .ok_or(ParseError::UnexpectedStructure)?
                .as_str()
                .to_string();

            Ok(Declaration {
                name,
                type_name: "EXCEPTION".to_string(),
                init_value: None,
                cursor_query: None,
                trigger: None,
                routine: None,
            })
        }
        Rule::cursor_declaration => build_cursor_declaration(declaration),
        Rule::trigger_declaration => build_trigger_declaration(declaration),
        Rule::procedure_declaration | Rule::function_declaration => {
            build_routine_declaration(declaration)
        }
        _ => Err(ParseError::UnexpectedStructure),
    }
}
