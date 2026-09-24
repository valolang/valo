//! Backend-independent source types and primitive alias resolution.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeName {
    /// Internal return type of a Sub. This is not a source value type.
    Void,
    String,
    Byte,
    Int16,
    Int32,
    Int64, // 64-bit
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
    GenericInstance {
        name: String,
        args: Vec<TypeName>,
    },
    Array(Box<TypeName>),
    Nullable(Box<TypeName>),
    /// `(X As Long, Y As String)`: a fixed group of values in one place.
    ///
    /// Tuples are structural: two of them are the same type when their
    /// elements are, whatever the elements happen to be called. Element names
    /// are there to read `point.X` instead of `point.Item1`, not to make one
    /// tuple type different from another.
    Tuple(Vec<TupleElement>),
}

/// One element of a tuple type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TupleElement {
    pub name: Option<String>,
    pub ty: TypeName,
}

impl TupleElement {
    /// The name this element answers to when none was given: `Item1`, `Item2`.
    pub fn positional_name(index: usize) -> String {
        format!("Item{}", index + 1)
    }
}

impl TypeName {
    /// Unsuffixed source integers have Int32 or Int64 type, never Int16.
    pub fn integer_literal(value: i64) -> Self {
        if i32::try_from(value).is_ok() {
            Self::Int32
        } else {
            Self::Int64
        }
    }

    /// Resolve source aliases without depending on the interpreter's value representation.
    pub fn from_primitive_name(name: &str) -> Option<Self> {
        Some(match name.to_ascii_lowercase().as_str() {
            "byte" | "uint8" => Self::Byte,
            "short" | "int16" => Self::Int16,
            "integer" | "int32" => Self::Int32,
            "long" | "int64" => Self::Int64,
            "uinteger" | "uint32" => Self::UInt32,
            "ulong" | "uint64" => Self::UInt64,
            "single" | "float32" => Self::Single,
            "double" | "float64" => Self::Double,
            "boolean" | "bool" => Self::Boolean,
            _ => return None,
        })
    }

    /// Fixed numeric storage width, independent of the compilation host.
    /// Pointer layouts require an explicit target and are not inferred here.
    pub fn numeric_bits(&self) -> Option<u16> {
        match self {
            Self::Byte => Some(8),
            Self::Int16 => Some(16),
            Self::Int32 | Self::UInt32 | Self::Single => Some(32),
            Self::Int64 | Self::UInt64 | Self::Double => Some(64),
            _ => None,
        }
    }

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
            TypeName::Tuple(elements) => TypeName::Tuple(
                elements
                    .iter()
                    .map(|element| TupleElement {
                        name: element.name.clone(),
                        ty: element.ty.substitute_generics(bindings),
                    })
                    .collect(),
            ),
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

    /// Reports whether this type holds whole numbers.
    ///
    /// Counting loops accept any integral width, so `For i As Long` is as valid
    /// as `For i As Integer`.
    pub fn is_integral(&self) -> bool {
        matches!(
            self,
            TypeName::Byte
                | TypeName::Int16
                | TypeName::Int32
                | TypeName::Int64
                | TypeName::UInt32
                | TypeName::UInt64
        )
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
            (TypeName::Tuple(left), TypeName::Tuple(right)) => {
                left.len() == right.len()
                    && left
                        .iter()
                        .zip(right)
                        .all(|(left, right)| left.ty.same_type(&right.ty))
            }
            _ => self == other,
        }
    }

    pub fn display_name(&self) -> String {
        match self {
            TypeName::Void => "Void".to_string(),
            TypeName::String => "String".to_string(),
            TypeName::Byte => "Byte".to_string(),
            TypeName::Int16 => "Short".to_string(),
            TypeName::Int32 => "Integer".to_string(),
            TypeName::Int64 => "Long".to_string(),
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
            TypeName::Tuple(elements) => format!(
                "({})",
                elements
                    .iter()
                    .map(|element| match &element.name {
                        Some(name) => format!("{} As {}", name, element.ty.display_name()),
                        None => element.ty.display_name(),
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }

    pub fn type_character(&self) -> String {
        match self {
            TypeName::String => "$".to_string(),
            TypeName::Int32 => "%".to_string(),
            TypeName::Int64 => "&".to_string(),
            TypeName::Single => "!".to_string(),
            TypeName::Double => "#".to_string(),
            TypeName::Currency => "@".to_string(),
            _ => String::new(),
        }
    }
}
