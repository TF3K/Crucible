use super::{ParseError, Rule, block::build_block};
use crate::ast::{Declaration, TriggerDeclaration, TriggerEvent, TriggerTiming};

pub(crate) fn build_trigger_declaration(
    pair: pest::iterators::Pair<Rule>,
) -> Result<Declaration, ParseError> {
    let mut inner = pair.into_inner();

    let name = inner
        .next()
        .ok_or(ParseError::UnexpectedStructure)?
        .as_str()
        .to_string();
    let timing_pair = inner.next().ok_or(ParseError::UnexpectedStructure)?;
    let timing = match timing_pair.as_rule() {
        Rule::trigger_timing => match timing_pair.as_str().to_uppercase().as_str() {
            "BEFORE" => TriggerTiming::Before,
            "AFTER" => TriggerTiming::After,
            _ => return Err(ParseError::UnexpectedStructure),
        },
        _ => return Err(ParseError::UnexpectedStructure),
    };
    let event_pair = inner.next().ok_or(ParseError::UnexpectedStructure)?;
    let event = match event_pair.as_rule() {
        Rule::trigger_event => match event_pair.as_str().to_uppercase().as_str() {
            "INSERT" => TriggerEvent::Insert,
            "UPDATE" => TriggerEvent::Update,
            "DELETE" => TriggerEvent::Delete,
            _ => return Err(ParseError::UnexpectedStructure),
        },
        _ => return Err(ParseError::UnexpectedStructure),
    };
    let table = inner
        .next()
        .ok_or(ParseError::UnexpectedStructure)?
        .as_str()
        .to_string();
    let body = build_block(inner.next().ok_or(ParseError::UnexpectedStructure)?)?;

    Ok(Declaration {
        name: name.clone(),
        type_name: "TRIGGER".to_string(),
        init_value: None,
        cursor_query: None,
        trigger: Some(TriggerDeclaration {
            name,
            timing,
            event,
            table,
            body,
        }),
    })
}
