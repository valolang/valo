use std::{cell::RefCell, collections::HashMap, fmt, rc::Rc};

#[cfg(windows)]
use windows::Win32::System::Com::IDispatch;
#[cfg(windows)]
use windows::core::Interface;

use crate::TypeName;
use crate::runtime::ArrayBound;

#[derive(Debug, Clone, PartialEq)]
pub struct ArrayValue {
    pub element_type: TypeName,
    pub elements: Vec<Value>,
    pub bounds: Vec<ArrayBound>,
    pub allocated: bool,
    pub dynamic: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecordValue {
    pub type_name: String,
    pub fields: HashMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    String(String),
    Byte(u8),
    Int16(i16),
    Int32(i32),
    Int64(i64),
    UInt32(u32),
    UInt64(u64),
    Single(f32),
    Double(f64),
    Currency(i64), // Fixed-point: 4 decimal places
    Decimal(i128),
    Boolean(bool),
    Date(f64), // VBA serial date
    Ptr(usize),
    FuncPtr(usize),
    Array(Rc<ArrayValue>),
    Collection(Rc<RefCell<CollectionValue>>),
    Record(Rc<RecordValue>),
    BoxedRecord(Rc<RecordValue>, String),
    Object(Rc<RefCell<ObjectValue>>),
    ComObject(Rc<ComObjectValue>),
    Error(i32),
    Nullable(Box<Value>),
    Lambda(Rc<LambdaValue>),
    Nothing,
    Null,
    Missing,
    Empty,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LambdaValue {
    pub params: Vec<crate::Parameter>,
    pub body: crate::frontend::ast::Expr,
    // For now, closures are not fully implemented, we'll just store the code.
}

#[derive(Debug, Clone, PartialEq)]
pub struct CollectionItem {
    pub key: Option<String>,
    pub value: Value,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct CollectionValue {
    pub items: Vec<CollectionItem>,
    pub key_map: HashMap<String, usize>,
}

impl CollectionValue {
    pub fn add(
        &mut self,
        value: Value,
        key: Option<String>,
        before: Option<Value>,
        after: Option<Value>,
    ) -> Result<(), String> {
        if let Some(ref k) = key {
            let lower_key = k.to_lowercase();
            if self.key_map.contains_key(&lower_key) {
                return Err(format!(
                    "This key is already associated with an element of this collection: '{}'",
                    k
                ));
            }
        }

        let index = if let Some(b) = before {
            if after.is_some() {
                return Err("Cannot specify both Before and After".to_string());
            }
            self.resolve_index(&b)?
        } else if let Some(a) = after {
            self.resolve_index(&a)? + 1
        } else {
            self.items.len()
        };

        if let Some(ref k) = key {
            self.key_map.insert(k.to_lowercase(), index);
        }

        // Shift existing key indices
        for idx in self.key_map.values_mut() {
            if *idx >= index && (key.is_none() || *idx != index) {
                *idx += 1;
            }
        }

        self.items.insert(index, CollectionItem { key, value });
        Ok(())
    }

    pub fn count(&self) -> i64 {
        self.items.len() as i64
    }

    pub fn remove(&mut self, index_or_key: &Value) -> Result<(), String> {
        let index = self.resolve_index(index_or_key)?;
        if index < self.items.len() {
            let removed = self.items.remove(index);
            if let Some(k) = removed.key {
                self.key_map.remove(&k.to_lowercase());
            }
            // Update indices for all keys pointing to items after the removed one
            for idx in self.key_map.values_mut() {
                if *idx > index {
                    *idx -= 1;
                }
            }
            Ok(())
        } else {
            Err("Index out of range".to_string())
        }
    }

    pub fn item(&self, index_or_key: &Value) -> Result<Value, String> {
        let index = self.resolve_index(index_or_key)?;
        self.items
            .get(index)
            .map(|item| item.value.clone())
            .ok_or_else(|| "Index out of range".to_string())
    }

    fn resolve_index(&self, index_or_key: &Value) -> Result<usize, String> {
        match index_or_key {
            Value::String(k) => {
                let lower_key = k.to_lowercase();
                self.key_map
                    .get(&lower_key)
                    .copied()
                    .ok_or_else(|| format!("Key '{}' not found in collection", k))
            }
            _ => {
                let idx = crate::runtime::numeric::value_to_i64(index_or_key)
                    .ok_or_else(|| "Collection index must be Integer or String".to_string())?;
                if idx < 1 {
                    return Err("Collection index must be 1-based".to_string());
                }
                let usize_idx = (idx - 1) as usize;
                if usize_idx >= self.items.len() {
                    return Err("Index out of range".to_string());
                }
                Ok(usize_idx)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectValue {
    pub class_name: String,
    pub fields: HashMap<String, Value>,
    pub event_bindings: Vec<EventBinding>,
    pub terminated: bool,
}

#[derive(Clone)]
pub struct ComObjectValue {
    pub prog_id: String,
    #[cfg(windows)]
    pub dispatch: IDispatch,
}

impl fmt::Debug for ComObjectValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ComObjectValue")
            .field("prog_id", &self.prog_id)
            .finish_non_exhaustive()
    }
}

impl PartialEq for ComObjectValue {
    fn eq(&self, other: &Self) -> bool {
        if !self.prog_id.eq_ignore_ascii_case(&other.prog_id) {
            return false;
        }
        #[cfg(windows)]
        {
            self.dispatch.as_raw() == other.dispatch.as_raw()
        }
        #[cfg(not(windows))]
        {
            true
        }
    }
}

#[derive(Debug, Clone)]
pub struct EventBinding {
    pub event_name: String,
    pub target: Value,
    pub handler_name: String,
}

impl PartialEq for EventBinding {
    fn eq(&self, other: &Self) -> bool {
        self.event_name.eq_ignore_ascii_case(&other.event_name)
            && self.handler_name.eq_ignore_ascii_case(&other.handler_name)
            && match (&self.target, &other.target) {
                (Value::Object(a), Value::Object(b)) => Rc::ptr_eq(a, b),
                (Value::Collection(a), Value::Collection(b)) => Rc::ptr_eq(a, b),
                (Value::Nothing, Value::Nothing) => true,
                (a, b) => a == b,
            }
    }
}

impl Eq for EventBinding {}

impl Value {
    pub fn type_name(&self) -> TypeName {
        match self {
            Value::String(_) => TypeName::String,
            Value::Byte(_) => TypeName::Byte,
            Value::Int16(_) => TypeName::Integer,
            Value::Int32(_) => TypeName::Long,
            Value::Int64(_) => TypeName::Int64,
            Value::UInt32(_) => TypeName::UInt32,
            Value::UInt64(_) => TypeName::UInt64,
            Value::Single(_) => TypeName::Single,
            Value::Double(_) => TypeName::Double,
            Value::Currency(_) => TypeName::Currency,
            Value::Decimal(_) => TypeName::Decimal,
            Value::Boolean(_) => TypeName::Boolean,
            Value::Date(_) => TypeName::Date,
            Value::Ptr(_) => TypeName::Ptr,
            Value::FuncPtr(_) => TypeName::FuncPtr,
            Value::Array(array) => TypeName::Array(Box::new(array.element_type.clone())),
            Value::Record(record) => TypeName::User(record.type_name.clone()),
            Value::BoxedRecord(_, interface_name) => TypeName::User(interface_name.clone()),
            Value::Object(object) => TypeName::User(object.borrow().class_name.clone()),
            Value::Collection(_) => TypeName::User("Collection".to_string()),
            Value::ComObject(com) => TypeName::User(com.prog_id.clone()),
            Value::Error(_) => TypeName::Variant,
            Value::Nullable(value) => TypeName::Nullable(Box::new(value.type_name())),
            Value::Lambda(_) => TypeName::User("Func".to_string()),
            Value::Nothing | Value::Null | Value::Missing => TypeName::Variant,
            Value::Empty => TypeName::Variant,
        }
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Boolean(value) => *value,
            Value::Byte(value) => *value != 0,
            Value::Int16(value) => *value != 0,
            Value::Int32(value) => *value != 0,
            Value::Int64(value) => *value != 0,
            Value::UInt32(value) => *value != 0,
            Value::UInt64(value) => *value != 0,
            Value::Single(value) => *value != 0.0,
            Value::Double(value) => *value != 0.0,
            Value::Currency(value) => *value != 0,
            Value::Decimal(value) => *value != 0,
            Value::Date(value) => *value != 0.0,
            Value::Ptr(value) => *value != 0,
            Value::FuncPtr(value) => *value != 0,
            Value::String(value) => !value.is_empty(),
            Value::Array(array) => array.allocated && !array.elements.is_empty(),
            Value::Collection(collection) => !collection.borrow().items.is_empty(),
            Value::Record(_) | Value::BoxedRecord(_, _) => true,
            Value::Object(_) | Value::ComObject(_) => true,
            Value::Error(code) => *code != 0,
            Value::Nullable(value) => value.is_truthy(),
            Value::Lambda(_) => true,
            Value::Nothing | Value::Null | Value::Missing => false,
            Value::Empty => false,
        }
    }

    pub fn to_output_string(&self) -> String {
        match self {
            Value::String(value) => value.clone(),
            Value::Byte(value) => value.to_string(),
            Value::Int16(value) => value.to_string(),
            Value::Int32(value) => value.to_string(),
            Value::Int64(value) => value.to_string(),
            Value::UInt32(value) => value.to_string(),
            Value::UInt64(value) => value.to_string(),
            Value::Single(value) => value.to_string(),
            Value::Double(value) => value.to_string(),
            Value::Currency(value) => {
                let major = value / 10000;
                let minor = (value % 10000).abs();
                format!("{}.{:04}", major, minor)
            }
            Value::Decimal(value) => value.to_string(),
            Value::Boolean(value) => {
                if *value {
                    "True".to_string()
                } else {
                    "False".to_string()
                }
            }
            Value::Date(value) => {
                // VBA serial date to YYYY-MM-DD HH:MM:SS (simplified)
                let days = value.floor() as i64;
                let seconds =
                    ((value.fract().rem_euclid(1.0) * 86400.0).round() as i64).rem_euclid(86400);

                // Base date: 1899-12-30
                // For now, let's just output the serial date if it's too complex,
                // but a simple "Date: serial" is better than nothing.
                if *value == 0.0 {
                    "00:00:00".to_string()
                } else if seconds == 0 {
                    format!("{days} (serial days)")
                } else if days == 0 {
                    let h = seconds / 3600;
                    let m = (seconds % 3600) / 60;
                    let s = seconds % 60;
                    format!("{:02}:{:02}:{:02}", h, m, s)
                } else {
                    let h = seconds / 3600;
                    let m = (seconds % 3600) / 60;
                    let s = seconds % 60;
                    format!("{days} {:02}:{:02}:{:02}", h, m, s)
                }
            }
            Value::Ptr(value) => format!("0x{:X}", value),
            Value::FuncPtr(value) => format!("<FuncPtr: 0x{:X}>", value),
            Value::Array(_) => "<Array>".to_string(),
            Value::Collection(_) => "<Collection>".to_string(),
            Value::Record(record) => format!("<{}>", record.type_name),
            Value::BoxedRecord(record, interface) => {
                format!("<{} as {}>", record.type_name, interface)
            }
            Value::Object(object) => format!("<{}>", object.borrow().class_name),
            Value::ComObject(object) => format!("<COM:{}>", object.prog_id),
            Value::Error(value) => format!("Error {}", value),
            Value::Nullable(value) => value.to_output_string(),
            Value::Lambda(_) => "<Lambda>".to_string(),
            Value::Nothing => "Nothing".to_string(),
            Value::Null => "Null".to_string(),
            Value::Missing => "Missing".to_string(),
            Value::Empty => "".to_string(),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_output_string())
    }
}
