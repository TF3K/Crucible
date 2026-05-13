use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::cursors::Cursors;
use crate::db::Database;
use crate::expr::Value;
use crate::triggers::Triggers;

#[derive(Debug, Clone)]
pub struct Environment {
    scope: Rc<Scope>,
    database: Rc<Database>,
    cursors: Rc<Cursors>,
    triggers: Rc<Triggers>,
}

#[derive(Debug)]
struct Scope {
    vars: RefCell<HashMap<String, Value>>,
    parent: Option<Environment>,
}

impl Environment {
    pub fn child(&self) -> Environment {
        Environment {
            scope: Rc::new(Scope {
                vars: RefCell::new(HashMap::new()),
                parent: Some(self.clone()),
            }),
            database: self.database.clone(),
            cursors: Rc::new(self.cursors.child()),
            triggers: Rc::new(self.triggers.child()),
        }
    }

    pub fn database(&self) -> &Database {
        self.database.as_ref()
    }

    pub fn cursors(&self) -> &Cursors {
        self.cursors.as_ref()
    }

    pub fn triggers(&self) -> &Triggers {
        self.triggers.as_ref()
    }

    pub fn get(&self, name: &str) -> Option<Value> {
        if let Some((head, tail)) = name.split_once('.') {
            let value = self.get(head)?;
            return get_path_value(value, tail);
        }

        if let Some(value) = self.scope.vars.borrow().get(name) {
            return Some(value.clone());
        }

        self.scope.parent.as_ref()?.get(name)
    }

    pub fn set(&self, name: impl Into<String>, value: Value) {
        self.scope.vars.borrow_mut().insert(name.into(), value);
    }

    pub fn assign(&self, name: &str, value: Value) {
        if let Some((head, tail)) = name.split_once('.') {
            self.assign_path(head, tail, value);
            return;
        }

        if self.scope.vars.borrow().contains_key(name) {
            self.scope.vars.borrow_mut().insert(name.to_string(), value);
        } else if let Some(parent) = &self.scope.parent {
            parent.assign(name, value);
        } else {
            self.scope.vars.borrow_mut().insert(name.to_string(), value);
        }
    }

    fn assign_path(&self, head: &str, tail: &str, value: Value) {
        if self.scope.vars.borrow().contains_key(head) {
            let mut vars = self.scope.vars.borrow_mut();
            let target = vars
                .get_mut(head)
                .expect("key should exist after contains_key check");
            assign_path_value(target, tail, value);
            return;
        }

        if let Some(parent) = &self.scope.parent {
            if parent.has_name(head) {
                parent.assign_path(head, tail, value);
                return;
            }
        }

        let mut record = Value::Record(HashMap::new());
        assign_path_value(&mut record, tail, value);
        self.scope
            .vars
            .borrow_mut()
            .insert(head.to_string(), record);
    }

    fn has_name(&self, name: &str) -> bool {
        if self.scope.vars.borrow().contains_key(name) {
            return true;
        }

        self.scope
            .parent
            .as_ref()
            .is_some_and(|parent| parent.has_name(name))
    }
}

impl Default for Environment {
    fn default() -> Self {
        Self {
            scope: Rc::new(Scope {
                vars: RefCell::new(HashMap::new()),
                parent: None,
            }),
            database: Rc::new(Database::default()),
            cursors: Rc::new(Cursors::default()),
            triggers: Rc::new(Triggers::default()),
        }
    }
}

fn get_path_value(value: Value, path: &str) -> Option<Value> {
    match value {
        Value::Record(fields) => {
            if let Some((head, tail)) = path.split_once('.') {
                let next = fields.get(head).cloned().unwrap_or(Value::Null);
                get_path_value(next, tail)
            } else {
                Some(fields.get(path).cloned().unwrap_or(Value::Null))
            }
        }
        Value::Null => Some(Value::Null),
        _ => None,
    }
}

fn assign_path_value(target: &mut Value, path: &str, value: Value) {
    if let Some((head, tail)) = path.split_once('.') {
        let fields = ensure_record(target);
        let entry = fields
            .entry(head.to_string())
            .or_insert_with(|| Value::Record(HashMap::new()));
        assign_path_value(entry, tail, value);
    } else {
        let fields = ensure_record(target);
        fields.insert(path.to_string(), value);
    }
}

fn ensure_record(target: &mut Value) -> &mut HashMap<String, Value> {
    if !matches!(target, Value::Record(_)) {
        *target = Value::Record(HashMap::new());
    }

    match target {
        Value::Record(fields) => fields,
        _ => unreachable!(),
    }
}
