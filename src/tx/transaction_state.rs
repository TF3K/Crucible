use std::collections::HashMap;

use crate::db::Table;

#[derive(Debug, Clone, Default)]
pub struct TransactionState {
    working_tables: Option<HashMap<String, Table>>,
}

impl TransactionState {
    pub fn is_active(&self) -> bool {
        self.working_tables.is_some()
    }

    pub fn begin(&mut self, tables: HashMap<String, Table>) {
        if self.working_tables.is_none() {
            self.working_tables = Some(tables);
        }
    }

    pub fn commit_into(&mut self, tables: &mut HashMap<String, Table>) -> bool {
        match self.working_tables.take() {
            Some(working_tables) => {
                *tables = working_tables;
                true
            }
            None => false,
        }
    }

    pub fn rollback(&mut self) -> bool {
        self.working_tables.take().is_some()
    }

    pub fn working_tables_mut(&mut self) -> Option<&mut HashMap<String, Table>> {
        self.working_tables.as_mut()
    }

    pub fn visible_tables<'a>(
        &'a self,
        tables: &'a HashMap<String, Table>,
    ) -> &'a HashMap<String, Table> {
        self.working_tables.as_ref().unwrap_or(tables)
    }
}