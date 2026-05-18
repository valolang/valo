#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeName {
    String,
    Integer,
    Boolean,
    Variant,
    User(String),
}

impl TypeName {
    pub fn builtin_default_value(&self) -> Option<crate::Value> {
        match self {
            TypeName::String => Some(crate::Value::String(String::new())),
            TypeName::Integer => Some(crate::Value::Integer(0)),
            TypeName::Boolean => Some(crate::Value::Boolean(false)),
            TypeName::Variant => Some(crate::Value::Empty),
            TypeName::User(_) => None,
        }
    }

    pub fn same_type(&self, other: &TypeName) -> bool {
        match (self, other) {
            (TypeName::User(left), TypeName::User(right)) => left.eq_ignore_ascii_case(right),
            _ => self == other,
        }
    }

    pub fn display_name(&self) -> String {
        match self {
            TypeName::String => "String".to_string(),
            TypeName::Integer => "Integer".to_string(),
            TypeName::Boolean => "Boolean".to_string(),
            TypeName::Variant => "Variant".to_string(),
            TypeName::User(name) => name.clone(),
        }
    }
}
