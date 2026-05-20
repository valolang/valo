use std::collections::HashMap;

use crate::runtime::{Diagnostic, Span, TypeName, Value};

use super::records::{RuntimeEnum, RuntimeType};

pub(crate) fn default_value(
    ty: &TypeName,
    types: &HashMap<String, RuntimeType>,
    enums: &HashMap<String, RuntimeEnum>,
    span: Span,
) -> Result<Value, Diagnostic> {
    if let Some(value) = ty.builtin_default_value() {
        return Ok(value);
    }

    let TypeName::User(name) = ty else {
        unreachable!("builtin types are handled above");
    };
    if name.eq_ignore_ascii_case("Object") {
        return Ok(Value::Nothing);
    }
    if enums.contains_key(&key(name)) {
        return Ok(Value::Int64(0));
    }
    let type_def = types.get(&key(name)).ok_or_else(|| {
        Diagnostic::new(
            crate::runtime::DiagnosticCode::UNKNOWN_NAME,
            format!("Type '{}' is not defined", name),
            Some(span),
        )
    });
    let Ok(type_def) = type_def else {
        return Ok(Value::Nothing);
    };

    let mut fields = HashMap::new();
    for field in &type_def.fields {
        fields.insert(
            key(&field.name),
            default_value(&field.ty, types, enums, span)?,
        );
    }

    Ok(Value::Record {
        type_name: type_def.name.clone(),
        fields,
    })
}

pub(crate) fn key(name: &str) -> String {
    name.to_ascii_lowercase()
}
