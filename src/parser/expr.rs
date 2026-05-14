use super::{ParseError, Rule};
use crate::ast::{BinaryOp, Expr, UnaryOp};
use crate::expr::Value;

pub(super) fn build_expr(pair: pest::iterators::Pair<Rule>) -> Result<Expr, ParseError> {
    match pair.as_rule() {
        Rule::expression => {
            let inner = pair
                .into_inner()
                .next()
                .ok_or(ParseError::UnexpectedStructure)?;
            build_expr(inner)
        }

        Rule::logical_or
        | Rule::logical_and
        | Rule::equality
        | Rule::comparison
        | Rule::addition
        | Rule::multiplication => build_left_associative(pair),

        Rule::unary => build_unary(pair),

        Rule::primary => {
            let inner = pair
                .into_inner()
                .next()
                .ok_or(ParseError::UnexpectedStructure)?;
            build_expr(inner)
        }

        Rule::number => {
            let n: f64 = pair
                .as_str()
                .parse()
                .map_err(|_| ParseError::UnexpectedStructure)?;
            Ok(Expr::Literal(Value::Number(n)))
        }

        Rule::string => Ok(Expr::Literal(Value::Text(parse_string_literal(
            pair.as_str(),
        )))),

        Rule::boolean => {
            let b = match pair.as_str().to_uppercase().as_str() {
                "TRUE" => true,
                "FALSE" => false,
                _ => return Err(ParseError::UnexpectedStructure),
            };
            Ok(Expr::Literal(Value::Bool(b)))
        }

        Rule::path_ident | Rule::ident => Ok(Expr::Var(pair.as_str().to_string())),

        _ => Err(ParseError::UnexpectedStructure),
    }
}

fn build_unary(pair: pest::iterators::Pair<Rule>) -> Result<Expr, ParseError> {
    let mut ops = Vec::new();
    let mut expr = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::MINUS => ops.push(UnaryOp::Neg),
            Rule::NOT => ops.push(UnaryOp::Not),
            _ => expr = Some(build_expr(inner)?),
        }
    }

    let mut result = expr.ok_or(ParseError::UnexpectedStructure)?;
    for op in ops.into_iter().rev() {
        result = Expr::Unary {
            op,
            expr: Box::new(result),
        };
    }
    Ok(result)
}

fn build_left_associative(pair: pest::iterators::Pair<Rule>) -> Result<Expr, ParseError> {
    let mut inner = pair.into_inner();
    let first = inner.next().ok_or(ParseError::UnexpectedStructure)?;
    let mut expr = build_expr(first)?;

    while let Some(op_or_rhs) = inner.next() {
        let op_rule = op_or_rhs.as_rule();

        let rhs_pair = inner.next().ok_or(ParseError::UnexpectedStructure)?;
        let rhs = build_expr(rhs_pair)?;

        let op = match op_rule {
            Rule::OR => BinaryOp::Or,
            Rule::AND => BinaryOp::And,
            Rule::EQ => BinaryOp::Eq,
            Rule::NEQ => BinaryOp::Neq,
            Rule::LT => BinaryOp::Lt,
            Rule::LTE => BinaryOp::Lte,
            Rule::GT => BinaryOp::Gt,
            Rule::GTE => BinaryOp::Gte,
            Rule::PLUS => BinaryOp::Add,
            Rule::MINUS => BinaryOp::Sub,
            Rule::STAR => BinaryOp::Mul,
            Rule::SLASH => BinaryOp::Div,
            _ => return Err(ParseError::UnexpectedStructure),
        };

        expr = Expr::Binary {
            left: Box::new(expr),
            op,
            right: Box::new(rhs),
        };
    }

    Ok(expr)
}

fn parse_string_literal(literal: &str) -> String {
    let quote = literal.chars().next().unwrap_or('\0');
    let inner = &literal[1..literal.len() - 1];

    match quote {
        '\'' => inner.replace("''", "'"),
        '"' => inner.replace("\"\"", "\""),
        _ => inner.to_string(),
    }
}
