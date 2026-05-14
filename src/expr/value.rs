use std::{collections::HashMap, fmt};

use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime};

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Number(f64),
    Text(String),
    Bool(bool),
    Date(NaiveDate),
    Timestamp(NaiveDateTime),
    DateTime(DateTime<FixedOffset>),
    Record(HashMap<String, Value>),
    Null,
}

impl Value {
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Number(_) => "NUMBER",
            Value::Text(_) => "TEXT",
            Value::Bool(_) => "BOOLEAN",
            Value::Date(_) => "DATE",
            Value::Timestamp(_) => "TIMESTAMP",
            Value::DateTime(_) => "DATETIME",
            Value::Record(_) => "RECORD",
            Value::Null => "NULL",
        }
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
