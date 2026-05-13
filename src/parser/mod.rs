use pest::Parser;
use pest_derive::Parser;

mod block;
mod cursor;
mod ddl;
mod dml;
mod expr;
mod statement;
pub mod trigger;
mod tx;

use self::block::build_block;
use self::expr::build_expr;
use crate::ast::{Block, Expr};

#[derive(Parser)]
#[grammar = "parser/plsql.pest"]
struct PlSqlParser;

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ParseError {
    #[error("parse error: {0}")]
    Pest(String),

    #[error("unexpected expression structure")]
    UnexpectedStructure,
}

impl From<pest::error::Error<Rule>> for ParseError {
    fn from(err: pest::error::Error<Rule>) -> Self {
        ParseError::Pest(err.to_string())
    }
}

pub fn parse_expression(input: &str) -> Result<Expr, ParseError> {
    let mut pairs = PlSqlParser::parse(Rule::expression, input)?;
    let pair = pairs.next().ok_or(ParseError::UnexpectedStructure)?;
    build_expr(pair)
}

pub fn parse_block(input: &str) -> Result<Block, ParseError> {
    let mut pairs = PlSqlParser::parse(Rule::block, input)?;
    let pair = pairs.next().ok_or(ParseError::UnexpectedStructure)?;
    build_block(pair)
}
