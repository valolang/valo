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
    Record(Rc<RecordValue>),
    Object(Rc<RefCell<ObjectValue>>),
    ComObject(Rc<ComObjectValue>),
    Error(i32),
    Nothing,
    Null,
    Missing,
    Empty,
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
    pub target: Rc<RefCell<ObjectValue>>,
    pub handler_name: String,
}

impl PartialEq for EventBinding {
    fn eq(&self, other: &Self) -> bool {
        self.event_name.eq_ignore_ascii_case(&other.event_name)
            && Rc::ptr_eq(&self.target, &other.target)
            && self.handler_name.eq_ignore_ascii_case(&other.handler_name)
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
            Value::Object(object) => TypeName::User(object.borrow().class_name.clone()),
            Value::ComObject(_) => TypeName::User("Object".to_string()),
            Value::Error(_) => TypeName::Variant,
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
            Value::Record(_) => true,
            Value::Object(_) | Value::ComObject(_) => true,
            Value::Error(code) => *code != 0,
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
            Value::Date(value) => format!("<Date: {}>", value),
            Value::Ptr(value) => format!("0x{:X}", value),
            Value::FuncPtr(value) => format!("<FuncPtr: 0x{:X}>", value),
            Value::Array(_) => "<Array>".to_string(),
            Value::Record(record) => format!("<{}>", record.type_name),
            Value::Object(object) => format!("<{}>", object.borrow().class_name),
            Value::ComObject(object) => format!("<COM:{}>", object.prog_id),
            Value::Error(code) => format!("Error {}", code),
            Value::Nothing => "Nothing".to_string(),
            Value::Null => "Null".to_string(),
            Value::Missing => "<Missing>".to_string(),
            Value::Empty => String::new(),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_output_string())
    }
}
