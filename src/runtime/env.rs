use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
};

use indexmap::IndexMap;

use crate::cursors::Cursors;
use crate::db::Database;
use crate::expr::{DataType, EvalError, Value};
use crate::routines::Routines;
use crate::triggers::Triggers;

#[derive(Debug, Clone)]
pub struct Environment {
    scope: Rc<Scope>,
    database: Rc<Database>,
    cursors: Rc<Cursors>,
    triggers: Rc<Triggers>,
    routines: Rc<Routines>,
}

#[derive(Debug)]
struct Scope {
    vars: RefCell<HashMap<String, Value>>,
    types: RefCell<HashMap<String, DataType>>,
    query_bindings: RefCell<HashMap<String, Value>>,
    query_columns: RefCell<HashMap<String, Value>>,
    query_ambiguous: RefCell<HashSet<String>>,
    parent: Option<Environment>,
}

impl Environment {
    pub fn child(&self) -> Environment {
        Environment {
            scope: Rc::new(Scope {
                vars: RefCell::new(HashMap::new()),
                types: RefCell::new(HashMap::new()),
                query_bindings: RefCell::new(HashMap::new()),
                query_columns: RefCell::new(HashMap::new()),
                query_ambiguous: RefCell::new(HashSet::new()),
                parent: Some(self.clone()),
            }),
            database: self.database.clone(),
            cursors: Rc::new(self.cursors.child()),
            triggers: Rc::new(self.triggers.child()),
            routines: self.routines.clone(),
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

    pub fn routines(&self) -> &Routines {
        self.routines.as_ref()
    }

    pub fn declare(
        &self,
        name: impl Into<String>,
        value: Value,
        type_name: impl Into<String>,
    ) -> Result<(), EvalError> {
        let name = name.into();
        let type_name = type_name.into();
        let declared_type = DataType::from_name(&type_name).ok_or_else(|| {
            EvalError::TypeError(format!(
                "unknown type `{}` for variable `{}`",
                type_name, name
            ))
        })?;

        if !declared_type.accept(&value) {
            return Err(EvalError::TypeError(format!(
                "variable `{}` expects {}, got {}",
                name,
                declared_type.name(),
                value.type_name(),
            )));
        }

        self.scope.vars.borrow_mut().insert(name.clone(), value);
        self.scope.types.borrow_mut().insert(name, declared_type);
        Ok(())
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

    pub fn bind_query_binding(&self, name: impl Into<String>, value: Value) {
        self.scope
            .query_bindings
            .borrow_mut()
            .insert(name.into(), value);
    }

    pub fn bind_query_column(&self, name: impl Into<String>, value: Value) {
        self.scope
            .query_columns
            .borrow_mut()
            .insert(name.into(), value);
    }

    pub fn mark_query_ambiguous(&self, name: impl Into<String>) {
        self.scope.query_ambiguous.borrow_mut().insert(name.into());
    }

    pub fn resolve(&self, name: &str) -> Result<Option<Value>, EvalError> {
        if let Some((head, tail)) = name.split_once('.') {
            let value = self.resolve(head)?;
            return Ok(value.and_then(|value| get_path_value(value, tail)));
        }

        if self.scope.query_ambiguous.borrow().contains(name) {
            return Err(EvalError::AmbiguousColumn(name.to_string()));
        }

        if let Some(value) = self.scope.query_bindings.borrow().get(name) {
            return Ok(Some(value.clone()));
        }

        if let Some(value) = self.scope.query_columns.borrow().get(name) {
            return Ok(Some(value.clone()));
        }

        if let Some(value) = self.scope.vars.borrow().get(name) {
            return Ok(Some(value.clone()));
        }

        self.scope
            .parent
            .as_ref()
            .map_or(Ok(None), |parent| parent.resolve(name))
    }

    pub fn assign(&self, name: &str, value: Value) -> Result<(), EvalError> {
        if let Some((head, tail)) = name.split_once('.') {
            return self.assign_path(head, tail, value);
        }

        if self.scope.vars.borrow().contains_key(name) {
            self.validate_assignment(name, &value)?;
            self.scope.vars.borrow_mut().insert(name.to_string(), value);
            return Ok(());
        }

        if let Some(parent) = &self.scope.parent {
            if parent.has_name(name) {
                return parent.assign(name, value);
            }
        }

        self.scope.vars.borrow_mut().insert(name.to_string(), value);
        Ok(())
    }

    fn assign_path(&self, head: &str, tail: &str, value: Value) -> Result<(), EvalError> {
        if self.scope.vars.borrow().contains_key(head) {
            self.validate_path_assignment(head)?;
            let mut vars = self.scope.vars.borrow_mut();
            let target = vars
                .get_mut(head)
                .expect("key should exist after contains_key check");
            assign_path_value(target, tail, value);
            return Ok(());
        }

        if let Some(parent) = &self.scope.parent {
            if parent.has_name(head) {
                return parent.assign_path(head, tail, value);
            }
        }

        let mut record = Value::Record(IndexMap::new());
        assign_path_value(&mut record, tail, value);
        self.scope
            .vars
            .borrow_mut()
            .insert(head.to_string(), record);
        Ok(())
    }

    fn validate_assignment(&self, name: &str, value: &Value) -> Result<(), EvalError> {
        if let Some(expected_type) = self.scope.types.borrow().get(name) {
            validate_value_type(name, expected_type, value)?;
        }

        Ok(())
    }

    fn validate_path_assignment(&self, name: &str) -> Result<(), EvalError> {
        if let Some(expected_type) = self.scope.types.borrow().get(name)
            && *expected_type != DataType::Record
        {
            return Err(EvalError::TypeError(format!(
                "variable `{}` expects RECORD, got {}",
                name,
                expected_type.name()
            )));
        }

        Ok(())
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
                types: RefCell::new(HashMap::new()),
                vars: RefCell::new(HashMap::new()),
                query_bindings: RefCell::new(HashMap::new()),
                query_columns: RefCell::new(HashMap::new()),
                query_ambiguous: RefCell::new(HashSet::new()),
                parent: None,
            }),
            database: Rc::new(Database::default()),
            cursors: Rc::new(Cursors::default()),
            triggers: Rc::new(Triggers::default()),
            routines: Rc::new(Routines::default()),
        }
    }
}

fn validate_value_type(
    name: &str,
    expected_type: &DataType,
    value: &Value,
) -> Result<(), EvalError> {
    if expected_type.accept(value) {
        return Ok(());
    }

    Err(EvalError::TypeError(format!(
        "variable `{}` expects {}, got {}",
        name,
        expected_type.name(),
        value.type_name()
    )))
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
            .or_insert_with(|| Value::Record(IndexMap::new()));
        assign_path_value(entry, tail, value);
    } else {
        let fields = ensure_record(target);
        fields.insert(path.to_string(), value);
    }
}

fn ensure_record(target: &mut Value) -> &mut IndexMap<String, Value> {
    if !matches!(target, Value::Record(_)) {
        *target = Value::Record(IndexMap::new());
    }

    match target {
        Value::Record(fields) => fields,
        _ => unreachable!(),
    }
}
