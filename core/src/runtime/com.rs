//! COM/OLE Automation architecture foundation.
//!
//! COM is Windows-native. These types model ownership and import planning
//! without pretending that a full COM runtime exists on every platform.

#[cfg(windows)]
use crate::runtime::ComObjectValue;
use crate::runtime::TypeName;
use crate::runtime::{Diagnostic, Span, Value};
#[cfg(windows)]
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComImport {
    pub namespace: String,
    pub source: ComImportSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComImportSource {
    ProgId(String),
    TypeLibraryGuid(String),
    TypeLibraryPath(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComTypeLibrary {
    pub namespace: String,
    pub types: Vec<ComTypeInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComTypeInfo {
    pub name: String,
    pub kind: ComTypeKind,
    pub members: Vec<ComMemberInfo>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComTypeKind {
    DispatchInterface,
    VTableInterface,
    CoClass,
    Enum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComMemberInfo {
    pub name: String,
    pub dispatch_id: i32,
    pub invoke_kind: ComInvokeKind,
    pub params: Vec<ComParamInfo>,
    pub return_type: TypeName,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComInvokeKind {
    Method,
    PropertyGet,
    PropertyPut,
    PropertySet,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComParamInfo {
    pub name: String,
    pub ty: TypeName,
    pub optional: bool,
    pub by_ref: bool,
}

#[cfg(windows)]
pub fn create_object(prog_id: &str, span: Span) -> Result<Value, Diagnostic> {
    use windows::Win32::System::Com::{
        CLSCTX_ALL, CLSIDFromProgID, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
        IDispatch,
    };
    use windows::core::{GUID, HSTRING};

    const RPC_E_CHANGED_MODE: i32 = 0x8001_0106_u32 as i32;

    let hr = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    if hr.is_err() && hr.0 != RPC_E_CHANGED_MODE {
        return Err(com_error(
            format!("CreateObject could not initialize COM: {:?}", hr),
            span,
        ));
    }

    let clsid = unsafe { CLSIDFromProgID(&HSTRING::from(prog_id)) }.map_err(|error| {
        com_error(
            format!("CreateObject could not resolve '{prog_id}': {error}"),
            span,
        )
    })?;
    let dispatch: IDispatch =
        unsafe { CoCreateInstance(&clsid, None, CLSCTX_ALL) }.map_err(|error| {
            com_error(
                format!("CreateObject could not create '{prog_id}': {error}"),
                span,
            )
        })?;

    Ok(Value::ComObject(Rc::new(ComObjectValue {
        prog_id: prog_id.to_string(),
        dispatch,
    })))
}

#[cfg(windows)]
pub fn invoke_com(
    com_obj: &crate::runtime::ComObjectValue,
    name: &str,
    args: &[Value],
    invoke_type: u16,
    span: Span,
) -> Result<Value, Diagnostic> {
    use windows::Win32::System::Com::{
        DISPATCH_METHOD, DISPATCH_PROPERTYGET, DISPATCH_PROPERTYPUT, DISPATCH_PROPERTYPUTREF,
        DISPPARAMS, EXCEPINFO, IDispatch,
    };
    use windows::Win32::System::Variant::VARIANT;
    use windows::core::{BSTR, GUID, HSTRING, PCWSTR};

    let dispatch = &com_obj.dispatch;

    let name_hstring = HSTRING::from(name);
    let mut dispid: i32 = 0;

    let hr = unsafe {
        let name_ptr = PCWSTR::from_raw(name_hstring.as_ptr());
        dispatch.GetIDsOfNames(
            &GUID::zeroed(),
            &name_ptr,
            1,
            2048, // LOCALE_SYSTEM_DEFAULT
            &mut dispid,
        )
    };

    if hr.is_err() {
        return Err(com_error(
            format!("COM object has no member '{}': {:?}", name, hr),
            span,
        ));
    }

    let mut variants: Vec<VARIANT> = args.iter().map(value_to_variant).collect();
    variants.reverse(); // COM expects args in reverse order

    let mut dispid_named: i32 = -3; // DISPID_PROPERTYPUT
    let is_put = invoke_type == 4 || invoke_type == 8; // DISPATCH_PROPERTYPUT | DISPATCH_PROPERTYPUTREF

    let mut dispparams = DISPPARAMS {
        rgvarg: variants.as_mut_ptr(),
        cArgs: variants.len() as u32,
        rgdispidNamedArgs: if is_put {
            &mut dispid_named
        } else {
            std::ptr::null_mut()
        },
        cNamedArgs: if is_put { 1 } else { 0 },
    };

    let mut result = VARIANT::default();
    let mut excepinfo = EXCEPINFO::default();
    let mut argerr: u32 = 0;

    let hr = unsafe {
        dispatch.Invoke(
            dispid,
            &GUID::zeroed(),
            2048,
            invoke_type,
            &mut dispparams,
            Some(&mut result),
            Some(&mut excepinfo),
            Some(&mut argerr),
        )
    };

    if hr.is_err() {
        return Err(com_error(
            format!("COM Invoke failed for '{}': {:?}", name, hr),
            span,
        ));
    }

    Ok(variant_to_value(&result))
}

#[cfg(windows)]
fn value_to_variant(value: &Value) -> windows::Win32::System::Variant::VARIANT {
    use windows::Win32::System::Variant::VARIANT;
    use windows::core::BSTR;

    match value {
        Value::Int16(n) => VARIANT::from(*n),
        Value::Int32(n) => VARIANT::from(*n),
        Value::Int64(n) => VARIANT::from(*n),
        Value::Double(n) => VARIANT::from(*n),
        Value::String(s) => VARIANT::from(BSTR::from(s.as_str())),
        Value::Boolean(b) => VARIANT::from(*b),
        Value::ComObject(obj) => VARIANT::from(obj.dispatch.clone()),
        _ => VARIANT::default(),
    }
}

#[cfg(windows)]
fn variant_to_value(var: &windows::Win32::System::Variant::VARIANT) -> Value {
    use windows::Win32::System::Com::IDispatch;
    use windows::core::BSTR;

    if let Ok(b) = BSTR::try_from(var) {
        return Value::String(b.to_string());
    }
    if let Ok(d) = f64::try_from(var) {
        return Value::Double(d);
    }
    if let Ok(i) = i32::try_from(var) {
        return Value::Int32(i);
    }
    if let Ok(b) = bool::try_from(var) {
        return Value::Boolean(b);
    }
    if let Ok(i) = i16::try_from(var) {
        return Value::Int16(i);
    }
    if let Ok(i) = i64::try_from(var) {
        return Value::Int64(i);
    }
    if let Ok(dispatch) = IDispatch::try_from(var) {
        return Value::ComObject(std::rc::Rc::new(crate::runtime::ComObjectValue {
            prog_id: "UnknownCOMObject".to_string(),
            dispatch,
        }));
    }
    Value::Empty
}

#[cfg(not(windows))]
pub fn create_object(prog_id: &str, span: Span) -> Result<Value, Diagnostic> {
    let _ = prog_id;
    Err(com_error(
        "CreateObject is only available on Windows COM/OLE Automation hosts",
        span,
    ))
}

#[cfg(not(windows))]
pub fn invoke_com(
    _com_obj: &crate::runtime::ComObjectValue,
    name: &str,
    _args: &[Value],
    _invoke_type: u16,
    span: Span,
) -> Result<Value, Diagnostic> {
    Err(com_error(
        format!("COM Invoke '{}' is only available on Windows", name),
        span,
    ))
}

fn com_error(message: impl Into<String>, span: Span) -> Diagnostic {
    Diagnostic::new(
        crate::runtime::DiagnosticCode::GENERIC,
        message.into(),
        Some(span),
    )
}
