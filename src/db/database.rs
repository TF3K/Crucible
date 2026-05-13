use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::tx::TransactionState;

use super::{DbError, Table};

#[derive(Debug, Clone)]
pub struct Database {
    tables: Rc<RefCell<HashMap<String, Table>>>,
    transaction: Rc<RefCell<TransactionState>>,
}

impl Database {
    pub fn create_table(
        &self,
        name: impl Into<String>,
        columns: impl IntoIterator<Item = impl Into<String>>,
    ) {
        let name = name.into();
        let table = Table::new(columns);

        self.with_visible_tables_mut(|tables| {
            let _ = tables.insert(name, table);
        });
    }

    pub fn drop_table(&self, name: &str) -> Result<(), DbError> {
        self.with_visible_tables_mut(|tables| {
            tables
                .remove(name)
                .ok_or_else(|| DbError::TableNotFound(name.to_string()))
                .map(|_| ())
        })
    }

    pub fn alter_table_add_constraint(
        &self,
        table_name: &str,
        constraint_name: String,
        columns: Vec<String>,
    ) -> Result<(), DbError> {
        self.with_visible_tables_mut(|tables| {
            let table = tables
                .get_mut(table_name)
                .ok_or_else(|| DbError::TableNotFound(table_name.to_string()))?;
            table.add_constraint(table_name, constraint_name, columns)
        })
    }

    pub fn table(&self, name: &str) -> Option<Table> {
        let tables = self.tables.borrow();
        let transaction = self.transaction.borrow();
        transaction.visible_tables(&tables).get(name).cloned()
    }

    pub fn with_table_mut<R>(
        &self,
        name: &str,
        f: impl FnOnce(&mut Table) -> R,
    ) -> Result<R, DbError> {
        let mut tables = self.tables.borrow_mut();
        let table = tables
            .get_mut(name)
            .ok_or_else(|| DbError::TableNotFound(name.to_string()))?;
        Ok(f(table))
    }

    pub fn with_table<R>(&self, name: &str, f: impl FnOnce(&Table) -> R) -> Result<R, DbError> {
        let tables = self.tables.borrow();
        let transaction = self.transaction.borrow();
        let visible_tables = transaction.visible_tables(&tables);
        let table = visible_tables
            .get(name)
            .ok_or_else(|| DbError::TableNotFound(name.to_string()))?;
        Ok(f(table))
    }

    pub fn begin_transaction(&self) {
        self.ensure_transaction();
    }

    pub fn commit_transaction(&self) {
        if !self.is_in_transaction() {
            return;
        }

        let mut tables = self.tables.borrow_mut();
        let mut transaction = self.transaction.borrow_mut();
        transaction.commit_into(&mut tables);
    }

    pub fn rollback_transaction(&self) {
        self.transaction.borrow_mut().rollback();
    }

    pub fn is_in_transaction(&self) -> bool {
        self.transaction.borrow().is_active()
    }

    pub fn with_transaction_table_mut<R>(
        &self,
        name: &str,
        f: impl FnOnce(&mut Table) -> R,
    ) -> Result<R, DbError> {
        self.ensure_transaction();

        let mut transaction = self.transaction.borrow_mut();
        let tables = transaction
            .working_tables_mut()
            .expect("transaction must be active after ensure_transaction");
        let table = tables
            .get_mut(name)
            .ok_or_else(|| DbError::TableNotFound(name.to_string()))?;
        Ok(f(table))
    }

    pub fn replace_table(&self, name: &str, table: Table) -> Result<(), DbError> {
        self.ensure_transaction();

        let mut transaction = self.transaction.borrow_mut();
        let tables = transaction
            .working_tables_mut()
            .expect("transaction must be active after ensure_transaction");
        let existing = tables
            .get_mut(name)
            .ok_or_else(|| DbError::TableNotFound(name.to_string()))?;
        *existing = table;
        Ok(())
    }

    fn with_visible_tables_mut<R>(&self, f: impl FnOnce(&mut HashMap<String, Table>) -> R) -> R {
        if self.is_in_transaction() {
            let mut transaction = self.transaction.borrow_mut();
            let tables = transaction
                .working_tables_mut()
                .expect("transaction must be active after check");
            f(tables)
        } else {
            let mut tables = self.tables.borrow_mut();
            f(&mut tables)
        }
    }

    fn ensure_transaction(&self) {
        if self.is_in_transaction() {
            return;
        }

        let snapshot = self.tables.borrow().clone();
        self.transaction.borrow_mut().begin(snapshot);
    }
}

impl Default for Database {
    fn default() -> Self {
        Self {
            tables: Rc::new(RefCell::new(HashMap::new())),
            transaction: Rc::new(RefCell::new(TransactionState::default())),
        }
    }
}
