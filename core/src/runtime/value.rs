use std::{cell::RefCell, collections::HashMap, fmt, rc::Rc};

use crate::TypeName;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    String(String),
    Integer(i64),
    Boolean(bool),
    Array {
        element_type: TypeName,
        elements: Vec<Value>,
    },
    Record {
        type_name: String,
        fields: HashMap<String, Value>,
    },
    Object(Rc<RefCell<ObjectValue>>),
    Nothing,
    Empty,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectValue {
    pub class_name: String,
    pub fields: HashMap<String, Value>,
}

impl Value {
    pub fn type_name(&self) -> TypeName {
        match self {
            Value::String(_) => TypeName::String,
            Value::Integer(_) => TypeName::Integer,
            Value::Boolean(_) => TypeName::Boolean,
            Value::Array { .. } => TypeName::Variant,
            Value::Record { type_name, .. } => TypeName::User(type_name.clone()),
            Value::Object(object) => TypeName::User(object.borrow().class_name.clone()),
            Value::Nothing => TypeName::Variant,
            Value::Empty => TypeName::Variant,
        }
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Boolean(value) => *value,
            Value::Integer(value) => *value != 0,
            Value::String(value) => !value.is_empty(),
            Value::Array { elements, .. } => !elements.is_empty(),
            Value::Record { .. } => true,
            Value::Object(_) => true,
            Value::Nothing => false,
            Value::Empty => false,
        }
    }

    pub fn to_output_string(&self) -> String {
        match self {
            Value::String(value) => value.clone(),
            Value::Integer(value) => value.to_string(),
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
            Value::Empty => String::new(),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_output_string())
    }
}
