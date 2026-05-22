use crate::expr::Value;
use std::sync::Arc;

use super::{Column, DbError, Row};

#[derive(Debug, Clone, PartialEq)]
pub struct TableConstraint {
    pub name: String,
    pub columns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Table {
    columns: Arc<Vec<Column>>,
    constraints: Vec<TableConstraint>,
    rows: Vec<Row>,
}

impl Table {
    pub fn new(columns: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            columns: Arc::new(
                columns
                    .into_iter()
                    .map(|name| Column::new(name.into()))
                    .collect(),
            ),
            constraints: Vec::new(),
            rows: Vec::new(),
        }
    }

    pub fn from_columns(columns: impl IntoIterator<Item = Column>) -> Self {
        Self {
            columns: Arc::new(columns.into_iter().collect()),
            constraints: Vec::new(),
            rows: Vec::new(),
        }
    }

    pub fn columns(&self) -> &[Column] {
        &self.columns
    }

    pub fn column_names(&self) -> impl Iterator<Item = &str> {
        self.columns.iter().map(|column| column.name.as_str())
    }

    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    pub fn rows_mut(&mut self) -> &mut Vec<Row> {
        &mut self.rows
    }

    pub fn constraints(&self) -> &[TableConstraint] {
        &self.constraints
    }

    pub fn add_constraint(
        &mut self,
        table_name: &str,
        name: String,
        columns: Vec<String>,
    ) -> Result<(), DbError> {
        if self
            .constraints
            .iter()
            .any(|constraint| constraint.name == name)
        {
            return Err(DbError::ConstraintAlreadyExists {
                table: table_name.to_string(),
                constraint: name,
            });
        }

        for column in &columns {
            if !self
                .columns
                .iter()
                .any(|table_column| table_column.name == *column)
            {
                return Err(DbError::ColumnNotFound {
                    table: table_name.to_string(),
                    column: column.clone(),
                });
            }
        }

        self.constraints.push(TableConstraint { name, columns });
        Ok(())
    }

    pub fn insert_row(&mut self, values: Vec<Value>) -> Result<(), DbError> {
        if values.len() != self.columns.len() {
            return Err(DbError::ColumnCountMismatch {
                expected: self.columns.len(),
                found: values.len(),
            });
        }

        let row = Row::from_values(&self.columns, values);
        self.rows.push(row);
        Ok(())
    }

    pub fn insert_row_with_columns(
        &mut self,
        table_name: &str,
        columns: &[String],
        values: Vec<Value>,
    ) -> Result<(), DbError> {
        if columns.len() != values.len() {
            return Err(DbError::ColumnCountMismatch {
                expected: columns.len(),
                found: values.len(),
            });
        }

        let mut row = Row::from_schema_with_defaults(&self.columns);

        for (column, value) in columns.iter().cloned().zip(values) {
            if !self
                .columns
                .iter()
                .any(|table_column| table_column.name == column)
            {
                return Err(DbError::ColumnNotFound {
                    table: table_name.to_string(),
                    column,
                });
            }
            row.insert(column, value);
        }

        self.rows.push(row);
        Ok(())
    }
}
