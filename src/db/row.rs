use std::iter::FromIterator;

use indexmap::IndexMap;

use crate::expr::{DataType, Value};

#[derive(Debug, Clone, PartialEq)]
pub struct Column {
    pub name: String,
    pub data_type: DataType,
    pub nullable: bool,
    pub default: Option<Value>,
}

impl Column {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            data_type: DataType::Text,
            nullable: true,
            default: None,
        }
    }

    pub fn with_type(name: impl Into<String>, data_type: DataType) -> Self {
        Self {
            name: name.into(),
            data_type,
            nullable: true,
            default: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub schema: Vec<Column>,
    pub values: IndexMap<String, Value>,
}

impl Row {
    pub fn new(schema: Vec<Column>, values: IndexMap<String, Value>) -> Self {
        Self { schema, values }
    }

    pub fn from_values(schema: &[Column], values: Vec<Value>) -> Self {
        assert_eq!(
            schema.len(),
            values.len(),
            "schema and values must have the same length"
        );

        let values = schema
            .iter()
            .cloned()
            .zip(values)
            .map(|(column, value)| (column.name, value))
            .collect();

        Self {
            schema: schema.to_vec(),
            values,
        }
    }

    pub fn blank_from_schema(schema: &[Column]) -> Self {
        let values = schema
            .iter()
            .cloned()
            .map(|column| (column.name, Value::Null))
            .collect();

        Self {
            schema: schema.to_vec(),
            values,
        }
    }

    pub fn from_schema_with_defaults(schema: &[Column]) -> Self {
        let values = schema
            .iter()
            .cloned()
            .map(|column| (column.name, column.default.unwrap_or(Value::Null)))
            .collect();

        Self {
            schema: schema.to_vec(),
            values,
        }
    }

    pub fn get(&self, column: &str) -> Option<&Value> {
        self.values.get(column)
    }

    pub fn insert(&mut self, column: String, value: Value) {
        self.values.insert(column, value);
    }

    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.values.keys()
    }
}

impl FromIterator<(String, Value)> for Row {
    fn from_iter<T: IntoIterator<Item = (String, Value)>>(iter: T) -> Self {
        let values = iter.into_iter().collect::<IndexMap<_, _>>();
        Self {
            schema: Vec::new(),
            values,
        }
    }
}

impl<'a> IntoIterator for &'a Row {
    type Item = (&'a String, &'a Value);
    type IntoIter = indexmap::map::Iter<'a, String, Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.values.iter()
    }
}
