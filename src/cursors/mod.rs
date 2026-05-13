use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::ast::CursorQuery;
use crate::expr::Value;

#[derive(Debug, Clone)]
pub struct Cursors {
    scope: Rc<CursorScope>,
}

#[derive(Debug)]
struct CursorScope {
    declarations: RefCell<HashMap<String, CursorQuery>>,
    instances: RefCell<HashMap<String, CursorInstance>>,
    parent: Option<Cursors>,
}

#[derive(Debug, Clone)]
struct CursorInstance {
    rows: Vec<Vec<Value>>,
    position: usize,
}

impl Cursors {
    pub fn default() -> Self {
        Self {
            scope: Rc::new(CursorScope {
                declarations: RefCell::new(HashMap::new()),
                instances: RefCell::new(HashMap::new()),
                parent: None,
            }),
        }
    }

    pub fn child(&self) -> Cursors {
        Cursors {
            scope: Rc::new(CursorScope {
                declarations: RefCell::new(HashMap::new()),
                instances: RefCell::new(HashMap::new()),
                parent: Some(self.clone()),
            }),
        }
    }

    pub fn declare(
        &self,
        name: impl Into<String>,
        query: CursorQuery,
    ) -> Result<(), crate::expr::EvalError> {
        let name = name.into();

        if self.scope.declarations.borrow().contains_key(&name) {
            return Err(crate::expr::EvalError::CursorAlreadyDeclared(name));
        }

        self.scope.declarations.borrow_mut().insert(name, query);
        Ok(())
    }

    pub fn query(&self, name: &str) -> Option<CursorQuery> {
        if let Some(query) = self.scope.declarations.borrow().get(name) {
            return Some(query.clone());
        }

        self.scope.parent.as_ref()?.query(name)
    }

    pub fn open_rows(
        &self,
        name: &str,
        rows: Vec<Vec<Value>>,
    ) -> Result<(), crate::expr::EvalError> {
        let owner = self
            .owner_scope(name)
            .ok_or_else(|| crate::expr::EvalError::CursorNotDeclared(name.to_string()))?;

        if owner.scope.instances.borrow().contains_key(name) {
            return Err(crate::expr::EvalError::CursorAlreadyOpen(name.to_string()));
        }

        owner
            .scope
            .instances
            .borrow_mut()
            .insert(name.to_string(), CursorInstance { rows, position: 0 });
        Ok(())
    }

    pub fn next_row(&self, name: &str) -> Result<Option<Vec<Value>>, crate::expr::EvalError> {
        let owner = self
            .owner_scope(name)
            .ok_or_else(|| crate::expr::EvalError::CursorNotOpen(name.to_string()))?;

        let mut instances = owner.scope.instances.borrow_mut();
        let instance = instances
            .get_mut(name)
            .ok_or_else(|| crate::expr::EvalError::CursorNotOpen(name.to_string()))?;

        if instance.position >= instance.rows.len() {
            return Ok(None);
        }

        let row = instance.rows[instance.position].clone();
        instance.position += 1;
        Ok(Some(row))
    }

    pub fn close(&self, name: &str) -> Result<(), crate::expr::EvalError> {
        let owner = self
            .owner_scope(name)
            .ok_or_else(|| crate::expr::EvalError::CursorNotOpen(name.to_string()))?;

        if owner.scope.instances.borrow_mut().remove(name).is_none() {
            return Err(crate::expr::EvalError::CursorNotOpen(name.to_string()));
        }

        Ok(())
    }

    fn owner_scope(&self, name: &str) -> Option<Cursors> {
        if self.scope.declarations.borrow().contains_key(name) {
            return Some(self.clone());
        }

        self.scope.parent.as_ref()?.owner_scope(name)
    }
}
