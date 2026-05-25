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

#[cfg(not(windows))]
pub fn create_object(prog_id: &str, span: Span) -> Result<Value, Diagnostic> {
    let _ = prog_id;
    Err(com_error(
        "CreateObject is only available on Windows COM/OLE Automation hosts",
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
