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
        CLSCTX_ALL, CLSCTX_INPROC_SERVER, CLSCTX_LOCAL_SERVER, CLSIDFromProgID, CoCreateInstance,
        IDispatch,
    };
    use windows::Win32::System::Ole::OleInitialize;
    use windows::core::HSTRING;

    unsafe {
        let _ = OleInitialize(None);
    }

    let clsid = unsafe { CLSIDFromProgID(&HSTRING::from(prog_id)) }.map_err(|error| {
        com_error(
            format!("CreateObject could not resolve '{prog_id}': {error}"),
            span,
        )
    })?;

    // Try to create the object and get IDispatch
    let dispatch: IDispatch = unsafe {
        match CoCreateInstance(&clsid, None, CLSCTX_LOCAL_SERVER | CLSCTX_INPROC_SERVER) {
            Ok(d) => d,
            Err(_) => {
                // Try with CLSCTX_ALL as fallback
                CoCreateInstance(&clsid, None, CLSCTX_ALL).map_err(|error| {
                    com_error(
                        format!("CreateObject: '{prog_id}' does not support automation (IDispatch): {error}"),
                        span,
                    )
                })?
            }
        }
    };

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
    let dispatch = &com_obj.dispatch;

    use windows::core::{GUID, HSTRING, PCWSTR};

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
        // Try fallback to default member if name is empty or similar
        if name.is_empty() {
            return invoke_default_com(com_obj, args, invoke_type, span);
        }

        return Err(com_error(
            format!("COM object has no member '{}': {:?}", name, hr),
            span,
        ));
    }

    invoke_com_dispid(com_obj, dispid, name, args, invoke_type, span)
}

#[cfg(windows)]
pub fn invoke_default_com(
    com_obj: &crate::runtime::ComObjectValue,
    args: &[Value],
    invoke_type: u16,
    span: Span,
) -> Result<Value, Diagnostic> {
    invoke_com_dispid(com_obj, 0, "<default>", args, invoke_type, span)
}

#[cfg(windows)]
fn invoke_com_dispid(
    com_obj: &crate::runtime::ComObjectValue,
    dispid: i32,
    name: &str,
    args: &[Value],
    invoke_type: u16,
    span: Span,
) -> Result<Value, Diagnostic> {
    use windows::Win32::System::Com::{DISPATCH_FLAGS, DISPPARAMS, EXCEPINFO};
    use windows::core::{GUID, VARIANT};

    let dispatch = &com_obj.dispatch;
    let mut variants: Vec<VARIANT> = args.iter().map(value_to_variant).collect();
    variants.reverse(); // COM expects args in reverse order

    let mut dispid_named: i32 = -3; // DISPID_PROPERTYPUT
    let is_put = invoke_type == 4 || invoke_type == 8; // DISPATCH_PROPERTYPUT | DISPATCH_PROPERTYPUTREF

    let dispparams = DISPPARAMS {
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
            DISPATCH_FLAGS(invoke_type),
            &dispparams,
            Some(&mut result),
            Some(&mut excepinfo),
            Some(&mut argerr),
        )
    };

    if let Err(err) = hr {
        let mut message = format!("COM Invoke failed for '{}': {:?}", name, err);
        if err.code().0 == -2147352567 {
            // 0x80020009 DISP_E_EXCEPTION
            let description = excepinfo.bstrDescription.to_string();
            if !description.is_empty() {
                message = format!(
                    "COM Invoke failed for '{}': {} ({:?})",
                    name, description, err
                );
            }
        }
        return Err(com_error(message, span));
    }

    Ok(variant_to_value(&result))
}

#[cfg(windows)]
pub(crate) fn value_to_variant(value: &Value) -> windows::core::VARIANT {
    use windows::Win32::System::Variant::{VT_DATE, VT_DISPATCH, VT_ERROR, VT_NULL};
    use windows::core::{BSTR, VARIANT};

    match value {
        Value::Int16(n) => VARIANT::from(*n),
        Value::Int32(n) => VARIANT::from(*n),
        Value::Int64(n) => VARIANT::from(*n),
        Value::Ptr(n) | Value::FuncPtr(n) => VARIANT::from(*n as i64),
        Value::UInt32(n) => VARIANT::from(*n as i64),
        Value::UInt64(n) => VARIANT::from(*n as i64),
        Value::Double(n) => VARIANT::from(*n),
        Value::String(s) => VARIANT::from(BSTR::from(s.as_str())),
        Value::Boolean(b) => VARIANT::from(*b),
        Value::Date(d) => {
            let var = VARIANT::default();
            unsafe {
                let raw = var.as_raw();
                *(std::ptr::addr_of!(raw.Anonymous.Anonymous.vt) as *mut u16) = VT_DATE.0;
                *(std::ptr::addr_of!(raw.Anonymous.Anonymous.Anonymous.date) as *mut f64) = *d;
            }
            var
        }
        Value::ComObject(obj) => VARIANT::from(obj.dispatch.clone()),
        Value::Nothing => {
            let var = VARIANT::default();
            unsafe {
                let raw = var.as_raw();
                *(std::ptr::addr_of!(raw.Anonymous.Anonymous.vt) as *mut u16) = VT_DISPATCH.0;
                *(std::ptr::addr_of!(raw.Anonymous.Anonymous.Anonymous.pdispVal)
                    as *mut *mut std::ffi::c_void) = std::ptr::null_mut();
            }
            var
        }
        Value::Null => {
            let var = VARIANT::default();
            unsafe {
                let raw = var.as_raw();
                *(std::ptr::addr_of!(raw.Anonymous.Anonymous.vt) as *mut u16) = VT_NULL.0;
            }
            var
        }
        Value::Missing => {
            let var = VARIANT::default();
            unsafe {
                let raw = var.as_raw();
                *(std::ptr::addr_of!(raw.Anonymous.Anonymous.vt) as *mut u16) = VT_ERROR.0;
                *(std::ptr::addr_of!(raw.Anonymous.Anonymous.Anonymous.scode) as *mut i32) =
                    0x80020004_u32 as i32; // DISP_E_PARAMNOTFOUND
            }
            var
        }
        _ => VARIANT::default(),
    }
}

#[cfg(windows)]
fn get_com_type_name(dispatch: &windows::Win32::System::Com::IDispatch) -> String {
    use windows::core::BSTR;
    unsafe {
        if let Ok(ti) = dispatch.GetTypeInfo(0, 2048) {
            let mut name = BSTR::default();
            if ti
                .GetDocumentation(-1, Some(&mut name), None, std::ptr::null_mut(), None)
                .is_ok()
            {
                return name.to_string();
            }
        }
    }
    "COMObject".to_string()
}

#[cfg(windows)]
pub(crate) fn variant_to_value(var: &windows::core::VARIANT) -> Value {
    use windows::Win32::System::Com::IDispatch;
    use windows::Win32::System::Variant::*;
    use windows::core::{BSTR, IUnknown, Interface};

    let vt = unsafe { var.as_raw().Anonymous.Anonymous.vt };
    let vt = VARENUM(vt);

    match vt {
        VT_EMPTY => Value::Empty,
        VT_NULL => Value::Null,
        VT_I2 => Value::Int16(unsafe { var.as_raw().Anonymous.Anonymous.Anonymous.iVal }),
        VT_I4 => Value::Int32(unsafe { var.as_raw().Anonymous.Anonymous.Anonymous.lVal }),
        VT_R4 => Value::Single(unsafe { var.as_raw().Anonymous.Anonymous.Anonymous.fltVal }),
        VT_R8 => Value::Double(unsafe { var.as_raw().Anonymous.Anonymous.Anonymous.dblVal }),
        VT_CY => Value::Currency(unsafe { var.as_raw().Anonymous.Anonymous.Anonymous.cyVal.int64 }),
        VT_DATE => Value::Date(unsafe { var.as_raw().Anonymous.Anonymous.Anonymous.date }),
        VT_BSTR => {
            if let Ok(b) = BSTR::try_from(var) {
                Value::String(b.to_string())
            } else {
                Value::String(String::new())
            }
        }
        VT_DISPATCH => {
            if let Ok(dispatch) = IDispatch::try_from(var) {
                let prog_id = get_com_type_name(&dispatch);
                Value::ComObject(std::rc::Rc::new(crate::runtime::ComObjectValue {
                    prog_id,
                    dispatch,
                }))
            } else {
                Value::Nothing
            }
        }
        VT_ERROR => Value::Error(unsafe { var.as_raw().Anonymous.Anonymous.Anonymous.scode }),
        VT_BOOL => {
            Value::Boolean(unsafe { var.as_raw().Anonymous.Anonymous.Anonymous.boolVal != 0 })
        }
        VT_UNKNOWN => {
            if let Ok(unknown) = IUnknown::try_from(var) {
                if let Ok(dispatch) = unknown.cast::<IDispatch>() {
                    let prog_id = get_com_type_name(&dispatch);
                    Value::ComObject(std::rc::Rc::new(crate::runtime::ComObjectValue {
                        prog_id,
                        dispatch,
                    }))
                } else {
                    Value::Empty
                }
            } else {
                Value::Nothing
            }
        }
        VT_I1 => Value::Byte(unsafe { var.as_raw().Anonymous.Anonymous.Anonymous.cVal } as u8),
        VT_UI1 => Value::Byte(unsafe { var.as_raw().Anonymous.Anonymous.Anonymous.bVal }),
        VT_UI2 => Value::Int32(unsafe { var.as_raw().Anonymous.Anonymous.Anonymous.uiVal } as i32),
        VT_UI4 => Value::Int64(unsafe { var.as_raw().Anonymous.Anonymous.Anonymous.ulVal } as i64),
        VT_I8 => Value::Int64(unsafe { var.as_raw().Anonymous.Anonymous.Anonymous.llVal }),
        VT_UI8 => Value::UInt64(unsafe { var.as_raw().Anonymous.Anonymous.Anonymous.ullVal }),
        VT_INT => Value::Int32(unsafe { var.as_raw().Anonymous.Anonymous.Anonymous.intVal }),
        VT_UINT => {
            Value::Int64(unsafe { var.as_raw().Anonymous.Anonymous.Anonymous.uintVal } as i64)
        }
        _ => {
            if let Ok(b) = BSTR::try_from(var) {
                Value::String(b.to_string())
            } else if let Ok(d) = f64::try_from(var) {
                Value::Double(d)
            } else if let Ok(i) = i32::try_from(var) {
                Value::Int32(i)
            } else {
                Value::Empty
            }
        }
    }
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

#[cfg(not(windows))]
pub fn invoke_default_com(
    _com_obj: &crate::runtime::ComObjectValue,
    _args: &[Value],
    _invoke_type: u16,
    span: Span,
) -> Result<Value, Diagnostic> {
    Err(com_error(
        "COM default member Invoke is only available on Windows",
        span,
    ))
}

#[cfg(windows)]
pub fn enumerable_com_values(
    com_obj: &crate::runtime::ComObjectValue,
    span: Span,
) -> Result<Vec<Value>, Diagnostic> {
    use windows::Win32::System::Com::IDispatch;
    use windows::Win32::System::Com::{DISPATCH_FLAGS, DISPPARAMS, EXCEPINFO};
    use windows::Win32::System::Ole::IEnumVARIANT;
    use windows::core::{GUID, IUnknown, Interface, VARIANT};

    let dispatch = &com_obj.dispatch;
    let dispparams = DISPPARAMS::default();
    let mut result = VARIANT::default();
    let mut excepinfo = EXCEPINFO::default();
    let mut argerr: u32 = 0;

    // DISPID_NEWENUM = -4
    let hr = unsafe {
        dispatch.Invoke(
            -4,
            &GUID::zeroed(),
            2048,
            DISPATCH_FLAGS(3), // DISPATCH_METHOD | DISPATCH_PROPERTYGET
            &dispparams,
            Some(&mut result),
            Some(&mut excepinfo),
            Some(&mut argerr),
        )
    };

    if let Err(err) = hr {
        return Err(com_error(
            format!(
                "COM object does not support enumeration (_NewEnum failed): {:?}",
                err
            ),
            span,
        ));
    }

    let enum_variant: IEnumVARIANT = if let Ok(dispatch) = IDispatch::try_from(&result) {
        dispatch.cast().map_err(|error| {
            com_error(
                format!("COM object enumeration interface (IEnumVARIANT) not found: {error}"),
                span,
            )
        })?
    } else if let Ok(unknown) = IUnknown::try_from(&result) {
        unknown.cast().map_err(|error| {
            com_error(
                format!("COM object enumeration interface (IEnumVARIANT) not found: {error}"),
                span,
            )
        })?
    } else {
        return Err(com_error(
            "COM object does not support enumeration (_NewEnum did not return an object)",
            span,
        ));
    };

    let mut values = Vec::new();
    loop {
        let mut item = [VARIANT::default()];
        let mut fetched = 0;
        let hr = unsafe { enum_variant.Next(&mut item, &mut fetched) };

        if hr.is_err() {
            return Err(com_error(format!("COM enumeration failed: {:?}", hr), span));
        }

        if fetched == 0 {
            break;
        }

        values.push(variant_to_value(&item[0]));
    }

    Ok(values)
}

#[cfg(not(windows))]
pub fn enumerable_com_values(
    _com_obj: &crate::runtime::ComObjectValue,
    span: Span,
) -> Result<Vec<Value>, Diagnostic> {
    Err(com_error(
        "COM enumeration is only available on Windows",
        span,
    ))
}

#[cfg(windows)]
pub fn get_object(
    pathname: Option<&str>,
    prog_id: Option<&str>,
    span: Span,
) -> Result<Value, Diagnostic> {
    use windows::Win32::System::Com::{CLSIDFromProgID, CoGetObject, IDispatch};
    use windows::Win32::System::Ole::{GetActiveObject, OleInitialize};
    use windows::core::{HSTRING, Interface};

    unsafe {
        let _ = OleInitialize(None);
    }

    if let Some(path) = pathname {
        if path.is_empty() {
            if let Some(pid) = prog_id {
                // GetObject(, "ProgID")
                let clsid = unsafe { CLSIDFromProgID(&HSTRING::from(pid)) }.map_err(|error| {
                    com_error(
                        format!("GetObject could not resolve '{pid}': {error}"),
                        span,
                    )
                })?;

                let mut unknown = None;
                unsafe {
                    GetActiveObject(&clsid, None, &mut unknown).map_err(|error| {
                        com_error(
                            format!("GetObject: could not get active object for '{pid}': {error}"),
                            span,
                        )
                    })?;
                }

                let dispatch: IDispatch = unknown.unwrap().cast().map_err(|error| {
                    com_error(
                        format!("GetObject: object for '{pid}' does not support automation (IDispatch): {error}"),
                        span,
                    )
                })?;

                return Ok(Value::ComObject(Rc::new(ComObjectValue {
                    prog_id: pid.to_string(),
                    dispatch,
                })));
            }
        } else {
            // GetObject("path", ["ProgID"])
            let dispatch: IDispatch = unsafe {
                CoGetObject(&HSTRING::from(path), None).map_err(|error| {
                    com_error(
                        format!("GetObject could not bind to '{path}': {error}"),
                        span,
                    )
                })?
            };

            return Ok(Value::ComObject(Rc::new(ComObjectValue {
                prog_id: prog_id.unwrap_or(path).to_string(),
                dispatch,
            })));
        }
    } else if let Some(pid) = prog_id {
        return get_object(Some(""), Some(pid), span);
    }

    Err(com_error(
        "GetObject requires either a pathname or a class name",
        span,
    ))
}

#[cfg(not(windows))]
pub fn get_object(
    _pathname: Option<&str>,
    _prog_id: Option<&str>,
    span: Span,
) -> Result<Value, Diagnostic> {
    Err(com_error(
        "GetObject is only available on Windows COM/OLE Automation hosts",
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
