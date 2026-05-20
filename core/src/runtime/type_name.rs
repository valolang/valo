#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeName {
    String,
    Byte,
    Integer, // 16-bit
    Long,    // 32-bit
    Int64,   // 64-bit
    UInt32,
    UInt64,
    Single,
    Double,
    Currency,
    Decimal,
    Boolean,
    Date,
    Variant,
    Ptr,
    FuncPtr,
    User(String),
}

impl TypeName {
    pub fn builtin_default_value(&self) -> Option<crate::Value> {
        match self {
            TypeName::String => Some(crate::Value::String(String::new())),
            TypeName::Byte => Some(crate::Value::Byte(0)),
            TypeName::Integer => Some(crate::Value::Int16(0)),
            TypeName::Long => Some(crate::Value::Int32(0)),
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
            TypeName::Byte => "Byte".to_string(),
            TypeName::Integer => "Integer".to_string(),
            TypeName::Long => "Long".to_string(),
            TypeName::Int64 => "Int64".to_string(),
            TypeName::UInt32 => "UInt32".to_string(),
            TypeName::UInt64 => "UInt64".to_string(),
            TypeName::Single => "Single".to_string(),
            TypeName::Double => "Double".to_string(),
            TypeName::Currency => "Currency".to_string(),
            TypeName::Decimal => "Decimal".to_string(),
            TypeName::Boolean => "Boolean".to_string(),
            TypeName::Date => "Date".to_string(),
            TypeName::Variant => "Variant".to_string(),
            TypeName::Ptr => "Ptr".to_string(),
            TypeName::FuncPtr => "FuncPtr".to_string(),
            TypeName::User(name) => name.clone(),
        }
    }
}
