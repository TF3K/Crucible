use super::Block;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TriggerTiming {
    Before,
    After,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TriggerEvent {
    Insert,
    Update,
    Delete,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TriggerDeclaration {
    pub name: String,
    pub timing: TriggerTiming,
    pub event: TriggerEvent,
    pub table: String,
    pub body: Block,
}