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
        self.tables
            .borrow_mut()
            .insert(name.into(), Table::new(columns));
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
