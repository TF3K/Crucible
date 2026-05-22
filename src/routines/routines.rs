use std::cell::RefCell;
use std::collections::HashMap;

use crate::ast::RoutineDeclaration;
use crate::expr::{DataType, EvalError};

#[derive(Debug, Default)]
pub struct Routines {
    declarations: RefCell<HashMap<String, RoutineDeclaration>>,
}

impl Routines {
    pub fn declare(&self, routine: RoutineDeclaration) -> Result<(), EvalError> {
        let name = routine.name.clone();

        if self.declarations.borrow().contains_key(&name) {
            return Err(EvalError::RoutineAlreadyDeclared(name));
        }

        if let Some(return_type) = &routine.return_type {
            DataType::from_name(return_type).ok_or_else(|| {
                EvalError::TypeError(format!(
                    "unknown return type `{}` for routine `{}`",
                    return_type, routine.name
                ))
            })?;
        }

        for parameter in &routine.parameters {
            DataType::from_name(&parameter.type_name).ok_or_else(|| {
                EvalError::TypeError(format!(
                    "unknown type `{}` for parameter `{}` in routine `{}`",
                    parameter.type_name, parameter.name, routine.name
                ))
            })?;
        }

        self.declarations.borrow_mut().insert(name, routine);
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<RoutineDeclaration> {
        self.declarations.borrow().get(name).cloned()
    }
}
