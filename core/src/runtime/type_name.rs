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
    Enum(String),
    GenericInstance { name: String, args: Vec<TypeName> },
    Array(Box<TypeName>),
    Nullable(Box<TypeName>),
}

impl TypeName {
    pub fn substitute_generics(&self, bindings: &[(String, TypeName)]) -> TypeName {
        match self {
            TypeName::User(name) => bindings
                .iter()
                .find(|(param, _)| param.eq_ignore_ascii_case(name))
                .map(|(_, ty)| ty.clone())
                .unwrap_or_else(|| self.clone()),
            TypeName::GenericInstance { name, args } => TypeName::GenericInstance {
                name: name.clone(),
                args: args
                    .iter()
                    .map(|arg| arg.substitute_generics(bindings))
                    .collect(),
            },
            TypeName::Array(inner) => {
                TypeName::Array(Box::new(inner.substitute_generics(bindings)))
            }
            TypeName::Nullable(inner) => {
                TypeName::Nullable(Box::new(inner.substitute_generics(bindings)))
            }
            _ => self.clone(),
        }
    }

    pub fn base_user_name(&self) -> Option<&str> {
        match self {
            TypeName::User(name)
            | TypeName::Enum(name)
            | TypeName::GenericInstance { name, .. } => Some(name),
            TypeName::Nullable(inner) => inner.base_user_name(),
            _ => None,
        }
    }

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
        }
    }

    pub fn same_type(&self, other: &TypeName) -> bool {
        match (self, other) {
            (TypeName::User(left), TypeName::User(right)) => left.eq_ignore_ascii_case(right),
            (TypeName::User(left), right @ TypeName::GenericInstance { .. })
            | (right @ TypeName::GenericInstance { .. }, TypeName::User(left)) => {
                left.eq_ignore_ascii_case(&right.display_name())
            }
            (TypeName::Enum(left), TypeName::Enum(right)) => left.eq_ignore_ascii_case(right),
            (
                TypeName::GenericInstance {
                    name: left_name,
                    args: left_args,
                },
                TypeName::GenericInstance {
                    name: right_name,
                    args: right_args,
                },
            ) => {
                left_name.eq_ignore_ascii_case(right_name)
                    && left_args.len() == right_args.len()
                    && left_args
                        .iter()
                        .zip(right_args)
                        .all(|(left, right)| left.same_type(right))
            }
            (TypeName::Array(left), TypeName::Array(right)) => left.same_type(right),
            (TypeName::Nullable(left), TypeName::Nullable(right)) => left.same_type(right),
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
            TypeName::Enum(name) => name.clone(),
            TypeName::GenericInstance { name, args } => format!(
                "{}(Of {})",
                name,
                args.iter()
                    .map(TypeName::display_name)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            TypeName::Array(inner) => format!("{}()", inner.display_name()),
            TypeName::Nullable(inner) => format!("{}?", inner.display_name()),
        }
    }

    pub fn vba_hint_char(&self) -> String {
        match self {
            TypeName::String => "$".to_string(),
            TypeName::Integer => "%".to_string(),
            TypeName::Long => "&".to_string(),
            TypeName::Single => "!".to_string(),
            TypeName::Double => "#".to_string(),
            TypeName::Currency => "@".to_string(),
            _ => String::new(),
        }
    }
}
