use std::{cell::RefCell, collections::HashMap, fmt, rc::Rc};

use crate::TypeName;
use crate::runtime::ArrayBound;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    String(String),
    Integer(i64),
    Double(f64),
    Boolean(bool),
    Array {
        element_type: TypeName,
        elements: Vec<Value>,
        bounds: Vec<ArrayBound>,
        allocated: bool,
    },
    Record {
        type_name: String,
        fields: HashMap<String, Value>,
    },
    Object(Rc<RefCell<ObjectValue>>),
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
            Value::Integer(_) => TypeName::Integer,
            Value::Double(_) => TypeName::Double,
            Value::Boolean(_) => TypeName::Boolean,
            Value::Array { .. } => TypeName::Variant,
            Value::Record { type_name, .. } => TypeName::User(type_name.clone()),
            Value::Object(object) => TypeName::User(object.borrow().class_name.clone()),
            Value::Nothing | Value::Null | Value::Missing => TypeName::Variant,
            Value::Empty => TypeName::Variant,
        }
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Boolean(value) => *value,
            Value::Integer(value) => *value != 0,
            Value::Double(value) => *value != 0.0,
            Value::String(value) => !value.is_empty(),
            Value::Array {
                elements,
                allocated,
                ..
            } => *allocated && !elements.is_empty(),
            Value::Record { .. } => true,
            Value::Object(_) => true,
            Value::Nothing | Value::Null | Value::Missing => false,
            Value::Empty => false,
        }
    }

    pub fn to_output_string(&self) -> String {
        match self {
            Value::String(value) => value.clone(),
            Value::Integer(value) => value.to_string(),
            Value::Double(value) => value.to_string(),
            Value::Boolean(value) => {
                if *value {
                    "True".to_string()
                } else {
                    "False".to_string()
                }
            }
            Value::Array { .. } => "<Array>".to_string(),
            Value::Record { type_name, .. } => format!("<{}>", type_name),
            Value::Object(object) => format!("<{}>", object.borrow().class_name),
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
