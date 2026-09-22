//! Interpreter defaults for frontend types.
use crate::frontend::type_model::TypeName;
use std::rc::Rc;

impl TypeName {
    pub fn builtin_default_value(&self) -> Option<crate::Value> {
        match self {
            TypeName::String => Some(crate::Value::String(Rc::new(String::new()))),
            TypeName::Byte => Some(crate::Value::Byte(0)),
            TypeName::Int16 => Some(crate::Value::Int16(0)),
            TypeName::Int32 => Some(crate::Value::Int32(0)),
            TypeName::Int64 => Some(crate::Value::Int64(0)),
            TypeName::UInt32 => Some(crate::Value::UInt32(0)),
            TypeName::UInt64 => Some(crate::Value::UInt64(0)),
            TypeName::Single => Some(crate::Value::Single(0.0)),
            TypeName::Double => Some(crate::Value::Double(0.0)),
            TypeName::Currency => Some(crate::Value::Currency(0)),
            TypeName::Decimal => Some(crate::Value::Decimal(0)),
            TypeName::Boolean => Some(crate::Value::Boolean(false)),
            TypeName::Date => Some(crate::Value::Date(0.0)),
            TypeName::Variant => Some(crate::Value::Empty),
            TypeName::Ptr => Some(crate::Value::Ptr(0)),
            TypeName::FuncPtr => Some(crate::Value::FuncPtr(0)),
            TypeName::User(_) => None,
            TypeName::Enum(_) => None,
            TypeName::GenericInstance { .. } => None,
            TypeName::Array(inner) => Some(crate::Value::Array(std::rc::Rc::new(
                crate::runtime::ArrayValue {
                    element_type: (**inner).clone(),
                    elements: Vec::new(),
                    bounds: Vec::new(),
                    allocated: false,
                    dynamic: true,
                },
            ))),
            TypeName::Nullable(_) => Some(crate::Value::Nothing),
            // A tuple's default is built from its elements' defaults, which
            // needs the whole set rather than one answer per type.
            TypeName::Tuple(_) => None,
        }
    }
}
