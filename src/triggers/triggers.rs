use std::{cell::RefCell, rc::Rc};

use crate::ast::{TriggerDeclaration, TriggerEvent, TriggerTiming};
use crate::expr::EvalError;

#[derive(Debug, Clone)]
pub struct Triggers {
    scope: Rc<TriggerScope>,
}

#[derive(Debug)]
struct TriggerScope {
    declarations: RefCell<Vec<TriggerDeclaration>>,
    parent: Option<Triggers>,
}

impl Triggers {
    pub fn default() -> Self {
        Self {
            scope: Rc::new(TriggerScope {
                declarations: RefCell::new(Vec::new()),
                parent: None,
            }),
        }
    }

    pub fn child(&self) -> Triggers {
        Triggers {
            scope: Rc::new(TriggerScope {
                declarations: RefCell::new(Vec::new()),
                parent: Some(self.clone()),
            }),
        }
    }

    pub fn declare(&self, trigger: TriggerDeclaration) -> Result<(), EvalError> {
        let name = trigger.name.clone();

        if self.has_name(&name) {
            return Err(EvalError::TriggerAlreadyDeclared(name));
        }

        self.scope.declarations.borrow_mut().push(trigger);
        Ok(())
    }

    pub fn matching(
        &self,
        table: &str,
        timing: TriggerTiming,
        event: TriggerEvent,
    ) -> Vec<TriggerDeclaration> {
        let mut triggers = self
            .scope
            .parent
            .as_ref()
            .map(|parent| parent.matching(table, timing, event))
            .unwrap_or_default();

        triggers.extend(
            self.scope
                .declarations
                .borrow()
                .iter()
                .filter(|trigger| {
                    trigger.table == table && trigger.timing == timing && trigger.event == event
                })
                .cloned(),
        );

        triggers
    }

    fn has_name(&self, name: &str) -> bool {
        if self
            .scope
            .declarations
            .borrow()
            .iter()
            .any(|trigger| trigger.name == name)
        {
            return true;
        }

        self.scope
            .parent
            .as_ref()
            .is_some_and(|parent| parent.has_name(name))
    }
}