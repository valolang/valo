//! Native collection foundations.
//!
//! These are runtime-library data structures, not VBA compatibility shims. They
//! are intentionally generic over the runtime value model so later parser,
//! semantic, and stdlib-module work can expose `List(Of T)` and
//! `Dictionary(Of K, V)` without changing their ownership semantics.

use std::collections::HashMap;

use crate::runtime::{TypeName, Value};

#[derive(Debug, Clone, PartialEq)]
pub struct ListValue {
    pub element_type: TypeName,
    elements: Vec<Value>,
}

impl ListValue {
    pub fn new(element_type: TypeName) -> Self {
        Self {
            element_type,
            elements: Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.elements.len()
    }

    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    pub fn push(&mut self, value: Value) {
        self.elements.push(value);
    }

    pub fn get(&self, index: usize) -> Option<&Value> {
        self.elements.get(index)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Value> {
        self.elements.iter()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DictionaryValue {
    pub key_type: TypeName,
    pub value_type: TypeName,
    entries: HashMap<DictionaryKey, Value>,
}

impl DictionaryValue {
    pub fn new(key_type: TypeName, value_type: TypeName) -> Self {
        Self {
            key_type,
            value_type,
            entries: HashMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn insert(&mut self, key: DictionaryKey, value: Value) -> Option<Value> {
        self.entries.insert(key, value)
    }

    pub fn get(&self, key: &DictionaryKey) -> Option<&Value> {
        self.entries.get(key)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&DictionaryKey, &Value)> {
        self.entries.iter()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DictionaryKey {
    String(String),
    Integer(i64),
    Boolean(bool),
}

impl TryFrom<&Value> for DictionaryKey {
    type Error = &'static str;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        match value {
            Value::String(value) => Ok(Self::String(value.clone())),
            Value::Byte(value) => Ok(Self::Integer(i64::from(*value))),
            Value::Int16(value) => Ok(Self::Integer(i64::from(*value))),
            Value::Int32(value) => Ok(Self::Integer(i64::from(*value))),
            Value::Int64(value) => Ok(Self::Integer(*value)),
            Value::Boolean(value) => Ok(Self::Boolean(*value)),
            _ => Err("dictionary keys must be String, Integer, Long, Int64, Byte, or Boolean"),
        }
    }
}
