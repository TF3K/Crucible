use indexmap::IndexMap;
use std::fmt;

use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DataType {
    Number,
    Text,
    Bool,
    Date,
    Timestamp,
    DateTime,
    Record,
    Null,
}

impl DataType {
    pub fn accept(&self, value: &Value) -> bool {
        value.is_null() || Self::from_value(value) == *self
    }

    pub fn name(&self) -> &'static str {
        match self {
            DataType::Number => "NUMBER",
            DataType::Text => "TEXT",
            DataType::Bool => "BOOLEAN",
            DataType::Date => "DATE",
            DataType::Timestamp => "TIMESTAMP",
            DataType::DateTime => "DATETIME",
            DataType::Record => "RECORD",
            DataType::Null => "NULL",
        }
    }

    pub fn from_name(type_name: &str) -> Option<Self> {
        if type_name.eq_ignore_ascii_case("NUMBER") {
            Some(Self::Number)
        } else if type_name.eq_ignore_ascii_case("TEXT") {
            Some(Self::Text)
        } else if type_name.eq_ignore_ascii_case("BOOL")
            || type_name.eq_ignore_ascii_case("BOOLEAN")
        {
            Some(Self::Bool)
        } else if type_name.eq_ignore_ascii_case("DATE") {
            Some(Self::Date)
        } else if type_name.eq_ignore_ascii_case("TIMESTAMP") {
            Some(Self::Timestamp)
        } else if type_name.eq_ignore_ascii_case("DATETIME") {
            Some(Self::DateTime)
        } else if type_name.eq_ignore_ascii_case("RECORD") {
            Some(Self::Record)
        } else if type_name.eq_ignore_ascii_case("NULL") {
            Some(Self::Null)
        } else {
            None
        }
    }

    pub fn from_value(value: &Value) -> Self {
        match value {
            Value::Number(_) => DataType::Number,
            Value::Text(_) => DataType::Text,
            Value::Bool(_) => DataType::Bool,
            Value::Date(_) => DataType::Date,
            Value::Timestamp(_) => DataType::Timestamp,
            Value::DateTime(_) => DataType::DateTime,
            Value::Record(_) => DataType::Record,
            Value::Null => DataType::Null,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Number(f64),
    Text(String),
    Bool(bool),
    Date(NaiveDate),
    Timestamp(NaiveDateTime),
    DateTime(DateTime<FixedOffset>),
    Record(IndexMap<String, Value>),
    Null,
}

impl Value {
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    pub fn type_name(&self) -> &'static str {
        DataType::from_value(self).name()
    }

    pub fn as_number(&self) -> Option<f64> {
        match self {
            Value::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_text(&self) -> Option<&str> {
        match self {
            Value::Text(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_date(&self) -> Option<&NaiveDate> {
        match self {
            Value::Date(d) => Some(d),
            _ => None,
        }
    }

    pub fn as_timestamp(&self) -> Option<&NaiveDateTime> {
        match self {
            Value::Timestamp(dt) => Some(dt),
            _ => None,
        }
    }

    pub fn as_datetime(&self) -> Option<&DateTime<FixedOffset>> {
        match self {
            Value::DateTime(dt) => Some(dt),
            _ => None,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Number(n) => write!(f, "{}", n),
            Value::Text(s) => write!(f, "'{}'", s.replace('\'', "''")),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Date(d) => write!(f, "{}", d),
            Value::Timestamp(dt) => write!(f, "{}", dt.format("%H:%M:%S")),
            Value::DateTime(dt) => write!(f, "{}", dt.format("%Y-%m-%d %H:%M:%S")),
            Value::Record(fields) => {
                let mut entries = fields.iter().collect::<Vec<_>>();
                entries.sort_by(|(left_key, _), (right_key, _)| left_key.cmp(right_key));

                write!(f, "{{")?;
                for (index, (key, value)) in entries.into_iter().enumerate() {
                    if index > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", key, value)?;
                }
                write!(f, "}}")
            }
            Value::Null => write!(f, "NULL"),
        }
    }
}
