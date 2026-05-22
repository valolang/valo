use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::{CString, c_char, c_int, c_void};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use libffi::middle::{Arg as FfiArg, Cif, CodePtr, Ret, Type as FfiType, arg as ffi_arg};

use crate::runtime::{
    ArrayValue, Diagnostic, RecordValue, Span, TypeName, Value, coerce_assignment,
};
use crate::{CallingConvention, DeclareDecl, DeclareKind, Expr, ExprKind, PassingMode};

use super::frame::Variable;
use super::records::RuntimeType;
use super::values::key;
use super::{Frame, Interpreter};

thread_local! {
    static ACTIVE_INTERPRETER: RefCell<Option<*mut Interpreter>> = const { RefCell::new(None) };
}

struct ActiveInterpreterGuard {
    previous: Option<*mut Interpreter>,
}

impl ActiveInterpreterGuard {
    fn enter(interpreter: *mut Interpreter) -> Self {
        let previous = ACTIVE_INTERPRETER.with(|i| {
            let previous = *i.borrow();
            *i.borrow_mut() = Some(interpreter);
            previous
        });
        Self { previous }
    }
}

impl Drop for ActiveInterpreterGuard {
    fn drop(&mut self) {
        ACTIVE_INTERPRETER.with(|i| *i.borrow_mut() = self.previous);
    }
}

pub(crate) struct CallbackTrampoline {
    pub(crate) _cif: Cif,
    pub(crate) alloc: *mut c_void,
    #[allow(dead_code)]
    pub(crate) code: *mut c_void,
    pub(crate) userdata: Box<CallbackUserData>,
}

impl Drop for CallbackTrampoline {
    fn drop(&mut self) {
        unsafe {
            libffi::low::closure_free(self.alloc as *mut _);
        }
    }
}

impl std::fmt::Debug for CallbackTrampoline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CallbackTrampoline")
            .field("function_name", &self.userdata.function_name)
            .finish()
    }
}

pub(crate) struct CallbackUserData {
    function_name: String,
    params: Vec<CallbackParam>,
    return_type: TypeName,
    is_sub: bool,
}

impl std::fmt::Debug for CallbackUserData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CallbackUserData")
            .field("function_name", &self.function_name)
            .field("params", &self.params.len())
            .field("return_type", &self.return_type)
            .field("is_sub", &self.is_sub)
            .finish()
    }
}

#[derive(Clone, Debug)]
struct CallbackParam {
    ty: TypeName,
}

#[derive(Default)]
pub(crate) struct NativeLibraries {
    handles: HashMap<String, NativeLibrary>,
}

impl std::fmt::Debug for NativeLibraries {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NativeLibraries")
            .field("handles", &self.handles)
            .finish()
    }
}

#[derive(Debug)]
struct NativeLibrary {
    display_name: String,
    handle: *mut c_void,
    symbols: HashMap<String, *mut c_void>,
}

impl Drop for NativeLibraries {
    fn drop(&mut self) {
        for library in self.handles.values() {
            unsafe {
                close_library(library.handle);
            }
        }
    }
}

struct MarshaledArgs {
    arg_types: Vec<FfiType>,
    arg_kinds: Vec<ArgKind>,
    storage: Vec<ArgumentStorage>,
    byrefs: Vec<ByRefUpdate>,
}

#[derive(Debug, Clone, Copy)]
enum ArgKind {
    Value,
    PointerValue,
    ByRefPointer,
}

enum ArgumentStorage {
    CString(CString),
    I16(Box<i16>),
    I32(Box<i32>),
    I64(Box<i64>),
    U32(Box<u32>),
    U64(Box<u64>),
    U8(Box<u8>),
    F32(Box<f32>),
    F64(Box<f64>),
    Bool(Box<i16>),
    Ptr(Box<usize>),
    Record(Vec<u8>),
    Array(Vec<u8>),
}

struct ByRefUpdate {
    variable: Variable,
    ty: TypeName,
    storage_index: usize,
    span: Span,
}

impl Interpreter {
    pub(crate) fn register_declares(&mut self, declares: &[DeclareDecl], module_key: Option<&str>) {
        for declare in declares {
            let name = match module_key {
                Some(module_key) => {
                    super::calls::qualified_key_for_ffi(Some(module_key), &declare.name)
                }
                None => key(&declare.name),
            };
            self.declares.insert(name, declare.clone());
            let exports_unqualified =
                module_key.is_none() || crate::modules::is_public(declare.visibility);
            if matches!(declare.kind, DeclareKind::Function) {
                if exports_unqualified {
                    self.function_modules
                        .entry(key(&declare.name))
                        .or_default()
                        .push(module_key.unwrap_or_default().to_string());
                }
            } else {
                if exports_unqualified {
                    self.sub_modules
                        .entry(key(&declare.name))
                        .or_default()
                        .push(module_key.unwrap_or_default().to_string());
                }
            }
        }
    }

    pub(crate) fn has_declared_function(&self, name: &str) -> bool {
        self.declares
            .get(&key(name))
            .is_some_and(|declare| matches!(declare.kind, DeclareKind::Function))
    }

    pub(crate) fn call_declared_function(
        &mut self,
        name: &str,
        args: &[Expr],
        frame: &mut Frame,
        span: Span,
    ) -> Result<Option<Value>, Diagnostic> {
        let Some(declare) = self.resolve_declare(name, frame, span, DeclareKind::Function)? else {
            return Ok(None);
        };
        self.call_native(&declare, args, frame, span).map(Some)
    }

    pub(crate) fn call_declared_sub(
        &mut self,
        name: &str,
        args: &[Expr],
        frame: &mut Frame,
        span: Span,
    ) -> Result<bool, Diagnostic> {
        let declare =
            if let Some(declare) = self.resolve_declare(name, frame, span, DeclareKind::Sub)? {
                declare
            } else if let Some(declare) =
                self.resolve_declare(name, frame, span, DeclareKind::Function)?
            {
                declare
            } else {
                return Ok(false);
            };
        let _ = self.call_native(&declare, args, frame, span)?;
        Ok(true)
    }

    fn resolve_declare(
        &self,
        name: &str,
        frame: &Frame,
        span: Span,
        kind_: DeclareKind,
    ) -> Result<Option<DeclareDecl>, Diagnostic> {
        if let Some(current) = frame.module_key()
            && let Some(declare) = self
                .declares
                .get(&super::calls::qualified_key_for_ffi(Some(current), name))
            && declare.kind == kind_
        {
            return Ok(Some(declare.clone()));
        }
        if let Some(declare) = self.declares.get(&key(name))
            && declare.kind == kind_
        {
            return Ok(Some(declare.clone()));
        }
        let modules = match kind_ {
            DeclareKind::Function => &self.function_modules,
            DeclareKind::Sub => &self.sub_modules,
        };
        let Some(candidates) = modules.get(&key(name)) else {
            return Ok(None);
        };
        let candidates: Vec<_> = candidates
            .iter()
            .filter(|module| !module.is_empty())
            .filter_map(|module| {
                self.declares
                    .get(&super::calls::qualified_key_for_ffi(Some(module), name))
            })
            .filter(|declare| declare.kind == kind_)
            .collect();
        if candidates.len() == 1 {
            return Ok(Some(candidates[0].clone()));
        }
        if candidates.len() > 1 {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::AMBIGUOUS_IMPORT,
                format!(
                    "{} '{}' is declared in multiple modules; use a module qualifier",
                    if kind_ == DeclareKind::Function {
                        "Function"
                    } else {
                        "Sub"
                    },
                    name
                ),
                Some(span),
            ));
        }
        Ok(None)
    }

    pub(crate) fn create_callback(&mut self, name: &str, span: Span) -> Result<usize, Diagnostic> {
        if let Some(&ptr) = self.ffi_callbacks.get(&key(name)) {
            return Ok(ptr);
        }

        // Try to find the function or sub to get its signature.
        let mut params = None;
        let mut return_type = None;
        let mut is_sub = false;

        let lookup = key(name);
        if let Some(function) = self.functions.get(&lookup) {
            params = Some(function.params.clone());
            return_type = Some(function.return_type.clone());
        } else if let Some(procedure) = self.procedures.get(&lookup) {
            params = Some(procedure.params.clone());
            return_type = Some(TypeName::Variant); // Subs return void
            is_sub = true;
        } else {
            // Check in function/sub modules.
            if let Some(modules) = self.function_modules.get(&lookup) {
                if let Some(first_mod) = modules.first() {
                    let qualified = super::calls::qualified_key(Some(first_mod), name);
                    if let Some(function) = self.functions.get(&qualified) {
                        params = Some(function.params.clone());
                        return_type = Some(function.return_type.clone());
                    }
                }
            } else if let Some(modules) = self.sub_modules.get(&lookup)
                && let Some(first_mod) = modules.first()
            {
                let qualified = super::calls::qualified_key(Some(first_mod), name);
                if let Some(procedure) = self.procedures.get(&qualified) {
                    params = Some(procedure.params.clone());
                    return_type = Some(TypeName::Variant);
                    is_sub = true;
                }
            }
        }

        let Some(params) = params else {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::UNKNOWN_NAME,
                format!("Cannot find function or sub '{}' for AddressOf", name),
                Some(span),
            ));
        };
        let return_type = return_type.unwrap();

        let mut arg_types = Vec::new();
        let mut callback_params = Vec::new();
        for param in &params {
            let ty = self.resolve_type_name(&param.ty, &Frame::default(), span)?;
            if !matches!(param.mode, PassingMode::ByVal) {
                return Err(unsupported(
                    "AddressOf callbacks currently require ByVal parameters",
                    param.span,
                )
                .with_help("pass pointer-sized values as ByVal LongPtr"));
            }
            let ffi_type = callback_ffi_type(&ty, param.span)?;
            arg_types.push(ffi_type);
            callback_params.push(CallbackParam { ty });
        }

        let ret_ffi_type = if is_sub {
            FfiType::void()
        } else {
            return_ffi_type(&return_type, false, span)?
        };

        let cif = Cif::new(arg_types, ret_ffi_type);

        let (alloc, code) = libffi::low::closure_alloc();
        if alloc.is_null() {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::GENERIC,
                "Failed to allocate libffi closure",
                Some(span),
            ));
        }

        let userdata = Box::new(CallbackUserData {
            function_name: name.to_string(),
            params: callback_params,
            return_type: return_type.clone(),
            is_sub,
        });
        let userdata_ptr = userdata.as_ref() as *const CallbackUserData as *mut c_void;

        unsafe extern "C" fn ffi_callback_trampoline(
            _cif: &libffi::low::ffi_cif,
            result: &mut c_void,
            args: *const *const c_void,
            userdata: &mut c_void,
        ) {
            let userdata = unsafe { &*(userdata as *mut c_void as *const CallbackUserData) };

            let interpreter_ptr =
                ACTIVE_INTERPRETER.with(|i| i.borrow().unwrap_or(std::ptr::null_mut()));

            if interpreter_ptr.is_null() {
                write_callback_default(result, &userdata.return_type, userdata.is_sub);
                eprintln!("Valo callback diagnostic: callback invoked without active interpreter");
                return;
            }

            let interpreter = unsafe { &mut *interpreter_ptr };
            let span = callback_span();
            let call_res = catch_unwind(AssertUnwindSafe(|| {
                let valo_args = read_callback_args(args, &userdata.params, span)?;
                let mut frame = Frame::default();
                interpreter.call_function(&userdata.function_name, &valo_args, &mut frame, span)
            }));

            match call_res {
                Ok(Ok(value)) => {
                    write_callback_result(result, value, &userdata.return_type, userdata.is_sub)
                }
                Ok(Err(error)) => {
                    write_callback_default(result, &userdata.return_type, userdata.is_sub);
                    interpreter.set_err(&error, 0);
                    eprintln!("Valo callback diagnostic: {}", error.message);
                }
                Err(_) => {
                    write_callback_default(result, &userdata.return_type, userdata.is_sub);
                    eprintln!("Valo callback diagnostic: callback panicked");
                }
            }
        }

        let prep_result = unsafe {
            libffi::low::prep_closure_mut(
                alloc as *mut _,
                cif.as_raw_ptr(),
                ffi_callback_trampoline,
                userdata_ptr,
                code,
            )
        };
        if prep_result.is_err() {
            unsafe {
                libffi::low::closure_free(alloc as *mut _);
            }
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::GENERIC,
                "Failed to prepare libffi callback trampoline",
                Some(span),
            )
            .with_help("the target platform may not support executable libffi closures"));
        }

        self.callback_trampolines.push(CallbackTrampoline {
            _cif: cif,
            alloc: alloc as *mut c_void,
            code: code.0,
            userdata,
        });

        self.ffi_callbacks.insert(key(name), code.0 as usize);
        Ok(code.0 as usize)
    }

    pub(crate) fn call_native(
        &mut self,
        declare: &DeclareDecl,
        args: &[Expr],
        frame: &mut Frame,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        if !declare.ptr_safe && usize::BITS == 64 && declare_uses_pointer(declare) {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::FFI_UNSUPPORTED_MARSHALING,
                format!(
                    "Declare '{}' uses pointer-sized values and must be marked PtrSafe on 64-bit targets",
                    declare.name
                ),
                Some(declare.span),
            )
            .with_help("add PtrSafe and use LongPtr for pointer values"));
        }
        let mut marshaled = self.marshal_args(declare, args, frame, span)?;
        let symbol_name = declare.alias.as_deref().unwrap_or(&declare.name);
        let symbol = self
            .native_libraries
            .symbol(&declare.lib, symbol_name, span)?;
        let return_type = declare.return_type.as_ref().unwrap_or(&TypeName::Variant);
        let cif_key = native_cif_key(declare, &marshaled, return_type);
        if !self.native_cifs.contains_key(&cif_key) {
            let return_ffi = return_ffi_type(return_type, declare.kind == DeclareKind::Sub, span)?;
            self.native_cifs.insert(
                cif_key.clone(),
                Rc::new(Cif::new(marshaled.arg_types.clone(), return_ffi)),
            );
        }
        let cif = self
            .native_cifs
            .get(&cif_key)
            .expect("native CIF inserted before call")
            .clone();

        let _active_interpreter = ActiveInterpreterGuard::enter(self as *mut Interpreter);

        let value_result = call_symbol(
            symbol,
            &marshaled,
            &cif,
            return_type,
            declare.kind == DeclareKind::Sub,
            declare.calling_convention,
            span,
        );

        let value = value_result?;
        marshaled.write_back(&self.types)?;
        if declare.kind == DeclareKind::Sub {
            return Ok(Value::Empty);
        }
        Ok(value)
    }

    fn marshal_args(
        &mut self,
        declare: &DeclareDecl,
        args: &[Expr],
        frame: &mut Frame,
        span: Span,
    ) -> Result<MarshaledArgs, Diagnostic> {
        if args.len() != declare.params.len() {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::PARSE,
                format!(
                    "Expected {} argument(s), got {}",
                    declare.params.len(),
                    args.len()
                ),
                Some(span),
            ));
        }
        let mut marshaled = MarshaledArgs {
            arg_types: Vec::new(),
            arg_kinds: Vec::new(),
            storage: Vec::new(),
            byrefs: Vec::new(),
        };
        for (param, arg) in declare.params.iter().zip(args.iter()) {
            let ty = self.resolve_type_name(&param.ty, frame, param.span)?;
            match param.mode {
                PassingMode::ByVal => {
                    let value = self.eval_expr(arg, frame)?;
                    marshal_byval(&ty, value, &mut marshaled, arg.span)?;
                }
                PassingMode::ByRef => {
                    let variable = if let ExprKind::Variable(name) = &arg.kind {
                        Some(frame.variable(name, arg.span)?)
                    } else {
                        None
                    };
                    let value = if let Some(variable) = &variable {
                        variable.borrow().clone()
                    } else {
                        self.eval_expr(arg, frame)?
                    };
                    let storage_index = marshaled.storage.len();
                    marshal_byref(&ty, value, &mut marshaled, &self.types, arg.span)?;
                    if let Some(variable) = variable {
                        marshaled.byrefs.push(ByRefUpdate {
                            variable,
                            ty,
                            storage_index,
                            span: arg.span,
                        });
                    }
                }
            }
        }
        Ok(marshaled)
    }
}

impl NativeLibraries {
    fn symbol(&mut self, lib: &str, symbol: &str, span: Span) -> Result<*mut c_void, Diagnostic> {
        let library = self.load_mut(lib, span)?;
        if let Some(ptr) = library.symbols.get(symbol).copied() {
            return Ok(ptr);
        }
        let c_symbol = CString::new(symbol).map_err(|_| {
            Diagnostic::new(
                crate::runtime::DiagnosticCode::FFI_SYMBOL_NOT_FOUND,
                format!("symbol `{symbol}` contains an interior NUL byte"),
                Some(span),
            )
        })?;
        let ptr = unsafe { find_symbol(library.handle, c_symbol.as_ptr()) };
        if ptr.is_null() {
            return Err(Diagnostic::new(
                crate::runtime::DiagnosticCode::FFI_SYMBOL_NOT_FOUND,
                format!(
                    "symbol `{symbol}` was not found in `{}`",
                    library.display_name
                ),
                Some(span),
            ));
        }
        library.symbols.insert(symbol.to_string(), ptr);
        Ok(ptr)
    }

    fn load_mut(&mut self, lib: &str, span: Span) -> Result<&mut NativeLibrary, Diagnostic> {
        let key = lib.to_ascii_lowercase();
        if !self.handles.contains_key(&key) {
            let candidates = library_candidates(lib);
            let mut attempted = Vec::new();
            let mut loaded = None;
            for candidate in &candidates {
                attempted.push(candidate.display().to_string());
                if let Ok(handle) = unsafe { open_library(candidate) } {
                    loaded = Some(NativeLibrary {
                        display_name: candidate.display().to_string(),
                        handle,
                        symbols: HashMap::new(),
                    });
                    break;
                }
            }
            let Some(library) = loaded else {
                let diagnostic = Diagnostic::new(
                    crate::runtime::DiagnosticCode::FFI_LIBRARY_NOT_FOUND,
                    format!("native library `{lib}` could not be loaded"),
                    Some(span),
                )
                .with_note(format!("attempted: {}", attempted.join(", ")));

                #[cfg(target_os = "macos")]
                {
                    if lib.eq_ignore_ascii_case("libc") || lib.eq_ignore_ascii_case("libm") {
                        return Err(diagnostic.with_help("try using `libSystem.B.dylib` or just `libc` / `libm` for automatic resolution"));
                    }
                }
                #[cfg(windows)]
                {
                    if lib.eq_ignore_ascii_case("libc") || lib.eq_ignore_ascii_case("libm") {
                        return Err(diagnostic.with_help("try using `msvcrt.dll` or just `libc` / `libm` for automatic resolution"));
                    }
                }

                return Err(diagnostic);
            };
            self.handles.insert(key.clone(), library);
        }
        Ok(self.handles.get_mut(&key).expect("library inserted"))
    }
}

impl MarshaledArgs {
    fn write_back(&mut self, types: &HashMap<String, RuntimeType>) -> Result<(), Diagnostic> {
        for update in &self.byrefs {
            let original_value = update.variable.borrow().clone();
            let value = value_from_storage(
                &self.storage[update.storage_index],
                &update.ty,
                &original_value,
                types,
                update.span,
            )?;
            *update.variable.borrow_mut() =
                coerce_assignment(&update.variable.ty, value, update.span)?;
        }
        Ok(())
    }
}

fn declare_uses_pointer(declare: &DeclareDecl) -> bool {
    declare
        .params
        .iter()
        .any(|param| matches!(param.ty, TypeName::Ptr | TypeName::FuncPtr))
        || declare
            .return_type
            .as_ref()
            .is_some_and(|ty| matches!(ty, TypeName::Ptr | TypeName::FuncPtr))
}

fn callback_ffi_type(ty: &TypeName, span: Span) -> Result<FfiType, Diagnostic> {
    match ty {
        TypeName::Byte => Ok(FfiType::u8()),
        TypeName::Integer => Ok(FfiType::i16()),
        TypeName::Long => Ok(FfiType::i32()),
        TypeName::Int64 | TypeName::Currency => Ok(FfiType::i64()),
        TypeName::UInt32 => Ok(FfiType::u32()),
        TypeName::UInt64 => Ok(FfiType::u64()),
        TypeName::Boolean => Ok(FfiType::i16()),
        TypeName::Single => Ok(FfiType::f32()),
        TypeName::Double => Ok(FfiType::f64()),
        TypeName::Ptr | TypeName::FuncPtr => Ok(FfiType::pointer()),
        TypeName::String
        | TypeName::Variant
        | TypeName::Decimal
        | TypeName::Date
        | TypeName::User(_)
        | TypeName::Enum(_)
        | TypeName::Array(_) => Err(unsupported(
            format!(
                "callback parameter type '{}' is not supported by AddressOf marshaling",
                ty.display_name()
            ),
            span,
        )),
    }
}

fn callback_span() -> Span {
    Span::new(
        crate::runtime::FileId(0),
        crate::runtime::SourcePos { line: 0, column: 0 },
        crate::runtime::SourcePos { line: 0, column: 0 },
    )
}

fn read_callback_args(
    args: *const *const c_void,
    params: &[CallbackParam],
    span: Span,
) -> Result<Vec<Expr>, Diagnostic> {
    if args.is_null() && !params.is_empty() {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::FFI_CALL,
            "callback trampoline received a null argument vector",
            Some(span),
        ));
    }
    params
        .iter()
        .enumerate()
        .map(|(index, param)| {
            let slot = unsafe { *args.add(index) };
            if slot.is_null() {
                return Err(Diagnostic::new(
                    crate::runtime::DiagnosticCode::FFI_CALL,
                    "callback trampoline received a null argument slot",
                    Some(span),
                ));
            }
            let kind = unsafe { read_callback_value(slot, &param.ty) };
            Ok(Expr { kind, span })
        })
        .collect()
}

unsafe fn read_callback_value(slot: *const c_void, ty: &TypeName) -> ExprKind {
    match ty {
        TypeName::Byte => ExprKind::Integer(unsafe { *(slot as *const u8) } as i64),
        TypeName::Integer => ExprKind::Integer(unsafe { *(slot as *const i16) } as i64),
        TypeName::Long => ExprKind::Integer(unsafe { *(slot as *const i32) } as i64),
        TypeName::Int64 | TypeName::Currency => ExprKind::Integer(unsafe { *(slot as *const i64) }),
        TypeName::UInt32 => ExprKind::Integer(unsafe { *(slot as *const u32) } as i64),
        TypeName::UInt64 => ExprKind::Integer(unsafe { *(slot as *const u64) } as i64),
        TypeName::Boolean => ExprKind::Boolean(unsafe { *(slot as *const i16) } != 0),
        TypeName::Single => ExprKind::Double(unsafe { *(slot as *const f32) } as f64),
        TypeName::Double => ExprKind::Double(unsafe { *(slot as *const f64) }),
        TypeName::Ptr | TypeName::FuncPtr => {
            ExprKind::Integer(unsafe { *(slot as *const usize) } as i64)
        }
        TypeName::String
        | TypeName::Variant
        | TypeName::Decimal
        | TypeName::Date
        | TypeName::User(_)
        | TypeName::Enum(_)
        | TypeName::Array(_) => ExprKind::Empty,
    }
}

fn write_callback_default(result: &mut c_void, return_type: &TypeName, is_sub: bool) {
    if is_sub {
        return;
    }
    unsafe {
        match return_type {
            TypeName::Byte => *(result as *mut c_void as *mut u8) = 0,
            TypeName::Integer | TypeName::Boolean => {
                *(result as *mut c_void as *mut i16) = 0;
            }
            TypeName::Long => *(result as *mut c_void as *mut i32) = 0,
            TypeName::Int64 | TypeName::Currency | TypeName::Variant => {
                *(result as *mut c_void as *mut i64) = 0;
            }
            TypeName::UInt32 => *(result as *mut c_void as *mut u32) = 0,
            TypeName::UInt64 => *(result as *mut c_void as *mut u64) = 0,
            TypeName::Single => *(result as *mut c_void as *mut f32) = 0.0,
            TypeName::Double => *(result as *mut c_void as *mut f64) = 0.0,
            TypeName::Ptr | TypeName::FuncPtr => {
                *(result as *mut c_void as *mut *mut c_void) = std::ptr::null_mut();
            }
            TypeName::String
            | TypeName::Decimal
            | TypeName::Date
            | TypeName::User(_)
            | TypeName::Enum(_)
            | TypeName::Array(_) => {}
        }
    }
}

fn write_callback_result(result: &mut c_void, value: Value, return_type: &TypeName, is_sub: bool) {
    if is_sub {
        return;
    }
    let span = callback_span();
    let value = coerce_assignment(return_type, value, span).unwrap_or(Value::Empty);
    unsafe {
        match value {
            Value::Byte(v) => *(result as *mut c_void as *mut u8) = v,
            Value::Int16(v) => *(result as *mut c_void as *mut i16) = v,
            Value::Int32(v) => *(result as *mut c_void as *mut i32) = v,
            Value::Int64(v) => *(result as *mut c_void as *mut i64) = v,
            Value::UInt32(v) => *(result as *mut c_void as *mut u32) = v,
            Value::UInt64(v) => *(result as *mut c_void as *mut u64) = v,
            Value::Boolean(v) => *(result as *mut c_void as *mut i16) = if v { -1 } else { 0 },
            Value::Single(v) => *(result as *mut c_void as *mut f32) = v,
            Value::Double(v) => *(result as *mut c_void as *mut f64) = v,
            Value::Currency(v) => *(result as *mut c_void as *mut i64) = v,
            Value::Ptr(v) | Value::FuncPtr(v) => {
                *(result as *mut c_void as *mut usize) = v;
            }
            _ => write_callback_default(result, return_type, is_sub),
        }
    }
}

fn marshal_byval(
    ty: &TypeName,
    value: Value,
    marshaled: &mut MarshaledArgs,
    span: Span,
) -> Result<(), Diagnostic> {
    let coerced = coerce_assignment(ty, value, span)?;
    let (storage, ffi_type) = match coerced {
        Value::Byte(v) => (ArgumentStorage::U8(Box::new(v)), FfiType::u8()),
        Value::Int16(v) => (ArgumentStorage::I16(Box::new(v)), FfiType::i16()),
        Value::Int32(v) => (ArgumentStorage::I32(Box::new(v)), FfiType::i32()),
        Value::Int64(v) => (ArgumentStorage::I64(Box::new(v)), FfiType::i64()),
        Value::UInt32(v) => (ArgumentStorage::U32(Box::new(v)), FfiType::u32()),
        Value::UInt64(v) => (ArgumentStorage::U64(Box::new(v)), FfiType::u64()),
        Value::Boolean(v) => (
            ArgumentStorage::Bool(Box::new(if v { -1 } else { 0 })),
            FfiType::i16(),
        ),
        Value::Ptr(v) | Value::FuncPtr(v) => {
            (ArgumentStorage::Ptr(Box::new(v)), FfiType::pointer())
        }
        Value::Currency(v) => (ArgumentStorage::I64(Box::new(v)), FfiType::i64()),
        Value::Single(v) => (ArgumentStorage::F32(Box::new(v)), FfiType::f32()),
        Value::Double(v) => (ArgumentStorage::F64(Box::new(v)), FfiType::f64()),
        Value::String(text) => {
            let c_string = CString::new(text)
                .map_err(|_| unsupported("String contains an interior NUL byte", span))?;
            (ArgumentStorage::CString(c_string), FfiType::pointer())
        }
        Value::Null | Value::Nothing | Value::Empty => {
            (ArgumentStorage::Ptr(Box::new(0)), FfiType::pointer())
        }
        Value::Array(_) => Err(unsupported(
            "ByVal arrays are not supported by native marshaling",
            span,
        ))?,
        Value::Record(_) => Err(unsupported(
            "ByVal structures are not supported; pass structures ByRef",
            span,
        ))?,
        Value::Object(_) => Err(unsupported(
            "object pointer marshaling is not enabled for this value",
            span,
        ))?,
        Value::Decimal(_) | Value::Date(_) | Value::Error(_) | Value::Missing => Err(unsupported(
            "value is not supported by native marshaling",
            span,
        ))?,
    };
    let arg_kind = if matches!(
        storage,
        ArgumentStorage::CString(_) | ArgumentStorage::Ptr(_)
    ) {
        ArgKind::PointerValue
    } else {
        ArgKind::Value
    };
    marshaled.storage.push(storage);
    marshaled.arg_types.push(ffi_type);
    marshaled.arg_kinds.push(arg_kind);
    Ok(())
}

fn marshal_byref(
    ty: &TypeName,
    value: Value,
    marshaled: &mut MarshaledArgs,
    types: &HashMap<String, RuntimeType>,
    span: Span,
) -> Result<(), Diagnostic> {
    let coerced = if matches!(ty, TypeName::User(_)) {
        value
    } else {
        coerce_assignment(ty, value, span)?
    };
    let item = match coerced {
        Value::Byte(v) => ArgumentStorage::U8(Box::new(v)),
        Value::Int16(v) => ArgumentStorage::I16(Box::new(v)),
        Value::Int32(v) => ArgumentStorage::I32(Box::new(v)),
        Value::Int64(v) => ArgumentStorage::I64(Box::new(v)),
        Value::UInt32(v) => ArgumentStorage::U32(Box::new(v)),
        Value::UInt64(v) => ArgumentStorage::U64(Box::new(v)),
        Value::Boolean(v) => ArgumentStorage::Bool(Box::new(if v { -1 } else { 0 })),
        Value::Ptr(v) | Value::FuncPtr(v) => ArgumentStorage::Ptr(Box::new(v)),
        Value::Currency(v) => ArgumentStorage::I64(Box::new(v)),
        Value::Single(v) => ArgumentStorage::F32(Box::new(v)),
        Value::Double(v) => ArgumentStorage::F64(Box::new(v)),
        Value::String(_) => {
            return Err(unsupported(
                "ByRef String buffers are not supported yet",
                span,
            ));
        }
        Value::Array(array) => {
            if !array.allocated {
                return Err(unsupported(
                    "unallocated arrays cannot be passed to native code",
                    span,
                ));
            }
            ArgumentStorage::Array(pack_array(&array.element_type, &array.elements, span)?)
        }
        Value::Record(record) => {
            let ty = types.get(&key(&record.type_name)).ok_or_else(|| {
                unsupported(
                    format!("structure type '{}' is not available", record.type_name),
                    span,
                )
            })?;
            ArgumentStorage::Record(pack_record(ty, &record.fields, span)?)
        }
        Value::Null | Value::Nothing | Value::Empty => ArgumentStorage::Ptr(Box::new(0)),
        Value::Object(_)
        | Value::Decimal(_)
        | Value::Date(_)
        | Value::Error(_)
        | Value::Missing => {
            return Err(unsupported(
                "value is not supported by ByRef native marshaling",
                span,
            ));
        }
    };
    marshaled.storage.push(item);
    marshaled.arg_types.push(FfiType::pointer());
    marshaled.arg_kinds.push(ArgKind::ByRefPointer);
    Ok(())
}

fn storage_pointer_value(storage: &ArgumentStorage) -> usize {
    match storage {
        ArgumentStorage::CString(value) => value.as_ptr() as usize,
        ArgumentStorage::Ptr(value) => **value,
        ArgumentStorage::Record(bytes) | ArgumentStorage::Array(bytes) => bytes.as_ptr() as usize,
        _ => storage_byref_pointer(storage),
    }
}

fn storage_byref_pointer(storage: &ArgumentStorage) -> usize {
    match storage {
        ArgumentStorage::CString(value) => value.as_ptr() as usize,
        ArgumentStorage::I16(value) => (&**value as *const i16) as usize,
        ArgumentStorage::I32(value) => (&**value as *const i32) as usize,
        ArgumentStorage::I64(value) => (&**value as *const i64) as usize,
        ArgumentStorage::U32(value) => (&**value as *const u32) as usize,
        ArgumentStorage::U64(value) => (&**value as *const u64) as usize,
        ArgumentStorage::U8(value) => (&**value as *const u8) as usize,
        ArgumentStorage::F32(value) => (&**value as *const f32) as usize,
        ArgumentStorage::F64(value) => (&**value as *const f64) as usize,
        ArgumentStorage::Bool(value) => (&**value as *const i16) as usize,
        ArgumentStorage::Ptr(value) => (&**value as *const usize) as usize,
        ArgumentStorage::Record(bytes) | ArgumentStorage::Array(bytes) => bytes.as_ptr() as usize,
    }
}

fn storage_value_arg(storage: &ArgumentStorage) -> FfiArg<'_> {
    match storage {
        ArgumentStorage::CString(_) | ArgumentStorage::Ptr(_) => {
            unreachable!("pointer values are marshaled through a separate pointer-value table")
        }
        ArgumentStorage::I16(value) => ffi_arg(&**value),
        ArgumentStorage::I32(value) => ffi_arg(&**value),
        ArgumentStorage::I64(value) => ffi_arg(&**value),
        ArgumentStorage::U32(value) => ffi_arg(&**value),
        ArgumentStorage::U64(value) => ffi_arg(&**value),
        ArgumentStorage::U8(value) => ffi_arg(&**value),
        ArgumentStorage::F32(value) => ffi_arg(&**value),
        ArgumentStorage::F64(value) => ffi_arg(&**value),
        ArgumentStorage::Bool(value) => ffi_arg(&**value),
        ArgumentStorage::Record(_) | ArgumentStorage::Array(_) => {
            unreachable!("record and array values are passed by pointer")
        }
    }
}

fn value_from_storage(
    storage: &ArgumentStorage,
    ty: &TypeName,
    original_value: &Value,
    types: &HashMap<String, RuntimeType>,
    span: Span,
) -> Result<Value, Diagnostic> {
    let value = match (storage, ty) {
        (ArgumentStorage::U8(v), _) => Value::Byte(**v),
        (ArgumentStorage::I16(v), TypeName::Boolean)
        | (ArgumentStorage::Bool(v), TypeName::Boolean) => Value::Boolean(**v != 0),
        (ArgumentStorage::I16(v), _) => Value::Int16(**v),
        (ArgumentStorage::I32(v), TypeName::UInt32) => Value::UInt32(**v as u32),
        (ArgumentStorage::I32(v), _) => Value::Int32(**v),
        (ArgumentStorage::I64(v), TypeName::UInt64) => Value::UInt64(**v as u64),
        (ArgumentStorage::I64(v), _) => Value::Int64(**v),
        (ArgumentStorage::U32(v), _) => Value::UInt32(**v),
        (ArgumentStorage::U64(v), _) => Value::UInt64(**v),
        (ArgumentStorage::F32(v), _) => Value::Single(**v),
        (ArgumentStorage::F64(v), _) => Value::Double(**v),
        (ArgumentStorage::Ptr(v), _) => Value::Ptr(**v),
        (ArgumentStorage::Record(bytes), TypeName::User(type_name)) => {
            let rt = types.get(&key(type_name)).ok_or_else(|| {
                unsupported(
                    format!("structure type '{}' is not available", type_name),
                    span,
                )
            })?;
            unpack_record(rt, bytes, span)?
        }
        (ArgumentStorage::Array(bytes), _) => {
            if let Value::Array(array) = original_value {
                let elements = unpack_array(&array.element_type, bytes, span)?;
                Value::Array(Rc::new(ArrayValue {
                    element_type: array.element_type.clone(),
                    elements,
                    bounds: array.bounds.clone(),
                    allocated: array.allocated,
                    dynamic: array.dynamic,
                }))
            } else {
                return Err(unsupported("original value was not an array", span));
            }
        }
        (ArgumentStorage::CString(_), _) => {
            return Err(unsupported("String write-back is not supported yet", span));
        }
        (ArgumentStorage::Bool(v), _) => Value::Int16(**v),
        _ => return Err(unsupported("Invalid storage write-back", span)),
    };
    Ok(value)
}

fn unpack_array(
    element_type: &TypeName,
    bytes: &[u8],
    span: Span,
) -> Result<Vec<Value>, Diagnostic> {
    let mut elements = Vec::new();
    let mut offset = 0;
    while offset < bytes.len() {
        let (value, size) = read_value(&bytes[offset..], element_type, span)?;
        elements.push(value);
        offset += size;
    }
    Ok(elements)
}

fn unpack_record(ty: &RuntimeType, bytes: &[u8], span: Span) -> Result<Value, Diagnostic> {
    let mut fields = HashMap::new();
    let mut offset = 0;
    let mut max_align = 1;
    for field in &ty.fields {
        let align = native_type_align(&field.ty, span)?;
        max_align = max_align.max(align);
        offset = align_offset(offset, align);
        let (value, size) = read_value(&bytes[offset..], &field.ty, span)?;
        fields.insert(key(&field.name), value);
        offset += size;
    }
    let _ = align_offset(offset, max_align);
    Ok(Value::Record(Rc::new(RecordValue {
        type_name: ty.name.clone(),
        fields,
    })))
}

fn read_value(bytes: &[u8], ty: &TypeName, span: Span) -> Result<(Value, usize), Diagnostic> {
    match ty {
        TypeName::Byte => {
            if bytes.is_empty() {
                return Err(unsupported("Buffer too small", span));
            }
            Ok((Value::Byte(bytes[0]), 1))
        }
        TypeName::Integer | TypeName::Boolean => {
            if bytes.len() < 2 {
                return Err(unsupported("Buffer too small", span));
            }
            let mut buf = [0u8; 2];
            buf.copy_from_slice(&bytes[..2]);
            let v = i16::from_ne_bytes(buf);
            if matches!(ty, TypeName::Boolean) {
                Ok((Value::Boolean(v != 0), 2))
            } else {
                Ok((Value::Int16(v), 2))
            }
        }
        TypeName::Long => {
            if bytes.len() < 4 {
                return Err(unsupported("Buffer too small", span));
            }
            let mut buf = [0u8; 4];
            buf.copy_from_slice(&bytes[..4]);
            Ok((Value::Int32(i32::from_ne_bytes(buf)), 4))
        }
        TypeName::UInt32 => {
            if bytes.len() < 4 {
                return Err(unsupported("Buffer too small", span));
            }
            let mut buf = [0u8; 4];
            buf.copy_from_slice(&bytes[..4]);
            Ok((Value::UInt32(u32::from_ne_bytes(buf)), 4))
        }
        TypeName::Int64 | TypeName::Currency => {
            if bytes.len() < 8 {
                return Err(unsupported("Buffer too small", span));
            }
            let mut buf = [0u8; 8];
            buf.copy_from_slice(&bytes[..8]);
            let v = i64::from_ne_bytes(buf);
            if matches!(ty, TypeName::Currency) {
                Ok((Value::Currency(v), 8))
            } else {
                Ok((Value::Int64(v), 8))
            }
        }
        TypeName::UInt64 => {
            if bytes.len() < 8 {
                return Err(unsupported("Buffer too small", span));
            }
            let mut buf = [0u8; 8];
            buf.copy_from_slice(&bytes[..8]);
            Ok((Value::UInt64(u64::from_ne_bytes(buf)), 8))
        }
        TypeName::Single => {
            if bytes.len() < 4 {
                return Err(unsupported("Buffer too small", span));
            }
            let mut buf = [0u8; 4];
            buf.copy_from_slice(&bytes[..4]);
            Ok((Value::Single(f32::from_ne_bytes(buf)), 4))
        }
        TypeName::Double => {
            if bytes.len() < 8 {
                return Err(unsupported("Buffer too small", span));
            }
            let mut buf = [0u8; 8];
            buf.copy_from_slice(&bytes[..8]);
            Ok((Value::Double(f64::from_ne_bytes(buf)), 8))
        }
        TypeName::Ptr | TypeName::FuncPtr => {
            let size = std::mem::size_of::<usize>();
            if bytes.len() < size {
                return Err(unsupported("Buffer too small", span));
            }
            let mut buf = [0u8; std::mem::size_of::<usize>()];
            buf.copy_from_slice(&bytes[..size]);
            let v = usize::from_ne_bytes(buf);
            if matches!(ty, TypeName::FuncPtr) {
                Ok((Value::FuncPtr(v), size))
            } else {
                Ok((Value::Ptr(v), size))
            }
        }
        _ => Err(unsupported(
            format!(
                "field type '{}' is not blittable for write-back",
                ty.display_name()
            ),
            span,
        )),
    }
}

fn pack_array(
    element_type: &TypeName,
    elements: &[Value],
    span: Span,
) -> Result<Vec<u8>, Diagnostic> {
    let mut bytes = Vec::new();
    for element in elements {
        append_value(&mut bytes, element_type, element, span)?;
    }
    Ok(bytes)
}

fn pack_record(
    ty: &RuntimeType,
    fields: &HashMap<String, Value>,
    span: Span,
) -> Result<Vec<u8>, Diagnostic> {
    if !ty.is_structure {
        return Err(unsupported(
            "only Structure/Type records with sequential primitive fields are supported",
            span,
        ));
    }
    let mut bytes = Vec::new();
    let mut max_align = 1;
    for field in &ty.fields {
        if field.array.is_some() {
            return Err(unsupported(
                "fixed arrays inside structures are not supported by native marshaling yet",
                span,
            ));
        }
        let align = native_type_align(&field.ty, span)?;
        max_align = max_align.max(align);
        pad_to_align(&mut bytes, align);
        let Some(value) = fields.get(&key(&field.name)) else {
            return Err(unsupported(
                format!("structure field '{}' is missing", field.name),
                span,
            ));
        };
        append_value(&mut bytes, &field.ty, value, span)?;
    }
    pad_to_align(&mut bytes, max_align);
    Ok(bytes)
}

fn native_type_align(ty: &TypeName, span: Span) -> Result<usize, Diagnostic> {
    let size = native_type_size(ty, span)?;
    Ok(size.min(std::mem::size_of::<usize>()).max(1))
}

fn native_type_size(ty: &TypeName, span: Span) -> Result<usize, Diagnostic> {
    let size = match ty {
        TypeName::Byte => 1,
        TypeName::Integer | TypeName::Boolean => 2,
        TypeName::Long | TypeName::UInt32 | TypeName::Single => 4,
        TypeName::Int64 | TypeName::UInt64 | TypeName::Currency | TypeName::Double => 8,
        TypeName::Ptr | TypeName::FuncPtr => std::mem::size_of::<usize>(),
        _ => {
            return Err(unsupported(
                format!(
                    "field type '{}' is not blittable for native structure layout",
                    ty.display_name()
                ),
                span,
            ));
        }
    };
    Ok(size)
}

fn align_offset(offset: usize, align: usize) -> usize {
    let mask = align.saturating_sub(1);
    (offset + mask) & !mask
}

fn pad_to_align(bytes: &mut Vec<u8>, align: usize) {
    let aligned = align_offset(bytes.len(), align);
    bytes.resize(aligned, 0);
}

fn append_value(
    bytes: &mut Vec<u8>,
    ty: &TypeName,
    value: &Value,
    span: Span,
) -> Result<(), Diagnostic> {
    let coerced = coerce_assignment(ty, value.clone(), span)?;
    match coerced {
        Value::Byte(v) => bytes.push(v),
        Value::Int16(v) => bytes.extend_from_slice(&v.to_ne_bytes()),
        Value::Int32(v) => bytes.extend_from_slice(&v.to_ne_bytes()),
        Value::Int64(v) => bytes.extend_from_slice(&v.to_ne_bytes()),
        Value::UInt32(v) => bytes.extend_from_slice(&v.to_ne_bytes()),
        Value::UInt64(v) => bytes.extend_from_slice(&v.to_ne_bytes()),
        Value::Boolean(v) => bytes.extend_from_slice(&(if v { -1i16 } else { 0i16 }).to_ne_bytes()),
        Value::Single(v) => bytes.extend_from_slice(&v.to_ne_bytes()),
        Value::Double(v) => bytes.extend_from_slice(&v.to_ne_bytes()),
        Value::Currency(v) => bytes.extend_from_slice(&v.to_ne_bytes()),
        Value::Ptr(v) | Value::FuncPtr(v) => bytes.extend_from_slice(&v.to_ne_bytes()),
        _ => {
            return Err(unsupported(
                "field type is not blittable for native marshaling",
                span,
            ));
        }
    }
    Ok(())
}

fn unsupported(message: impl Into<String>, span: Span) -> Diagnostic {
    Diagnostic::new(
        crate::runtime::DiagnosticCode::FFI_UNSUPPORTED_MARSHALING,
        message,
        Some(span),
    )
}

fn call_symbol(
    symbol: *mut c_void,
    marshaled: &MarshaledArgs,
    cif: &Cif,
    return_type: &TypeName,
    is_sub: bool,
    convention: CallingConvention,
    span: Span,
) -> Result<Value, Diagnostic> {
    if matches!(convention, CallingConvention::StdCall) && !cfg!(all(windows, target_arch = "x86"))
    {
        return Err(Diagnostic::new(
            crate::runtime::DiagnosticCode::FFI_CALL,
            "stdcall is only distinct on 32-bit Windows; use the default ABI on this target",
            Some(span),
        ));
    }

    let pointer_values = marshaled
        .storage
        .iter()
        .zip(marshaled.arg_kinds.iter())
        .filter_map(|(storage, kind)| match kind {
            ArgKind::Value => None,
            ArgKind::PointerValue => Some(storage_pointer_value(storage)),
            ArgKind::ByRefPointer => Some(storage_byref_pointer(storage)),
        })
        .collect::<Vec<_>>();
    let mut pointer_index = 0;
    let args = marshaled
        .storage
        .iter()
        .zip(marshaled.arg_kinds.iter())
        .map(|(storage, kind)| match kind {
            ArgKind::Value => storage_value_arg(storage),
            ArgKind::PointerValue | ArgKind::ByRefPointer => {
                let arg = ffi_arg(&pointer_values[pointer_index]);
                pointer_index += 1;
                arg
            }
        })
        .collect::<Vec<_>>();
    let code = CodePtr(symbol);
    if is_sub {
        unsafe { cif.call_return_into(code, &args, Ret::void()) };
        return Ok(Value::Empty);
    }
    call_return_value(cif, code, &args, return_type, span)
}

fn native_cif_key(
    declare: &DeclareDecl,
    marshaled: &MarshaledArgs,
    return_type: &TypeName,
) -> String {
    let symbol = declare.alias.as_deref().unwrap_or(&declare.name);
    let mut key = format!(
        "{}\0{}\0{:?}\0{:?}\0{}",
        declare.lib,
        symbol,
        declare.kind,
        declare.calling_convention,
        return_type.display_name()
    );
    for (ffi_type, kind) in marshaled.arg_types.iter().zip(marshaled.arg_kinds.iter()) {
        key.push('\0');
        key.push_str(&format!("{ffi_type:?}:{kind:?}"));
    }
    key
}

fn return_ffi_type(ty: &TypeName, is_sub: bool, span: Span) -> Result<FfiType, Diagnostic> {
    if is_sub {
        return Ok(FfiType::void());
    }
    let ty = match ty {
        TypeName::Byte => FfiType::u8(),
        TypeName::Integer => FfiType::i16(),
        TypeName::Long => FfiType::i32(),
        TypeName::Int64 | TypeName::Currency | TypeName::Variant => FfiType::i64(),
        TypeName::UInt32 => FfiType::u32(),
        TypeName::UInt64 => FfiType::u64(),
        TypeName::Boolean => FfiType::i16(),
        TypeName::Ptr | TypeName::FuncPtr => FfiType::pointer(),
        TypeName::Single => FfiType::f32(),
        TypeName::Double => FfiType::f64(),
        TypeName::String => {
            return Err(unsupported(
                "String return values require an explicit pointer return and string conversion",
                span,
            ));
        }
        TypeName::Decimal
        | TypeName::Date
        | TypeName::User(_)
        | TypeName::Enum(_)
        | TypeName::Array(_) => {
            return Err(unsupported(
                format!(
                    "return type '{}' is not supported by native marshaling",
                    ty.display_name()
                ),
                span,
            ));
        }
    };
    Ok(ty)
}

fn call_return_value(
    cif: &Cif,
    code: CodePtr,
    args: &[FfiArg<'_>],
    ty: &TypeName,
    span: Span,
) -> Result<Value, Diagnostic> {
    let value = match ty {
        TypeName::Byte => {
            let mut ret = 0u8;
            unsafe { cif.call_return_into(code, args, Ret::new(&mut ret)) };
            Value::Byte(ret)
        }
        TypeName::Integer => {
            let mut ret = 0i16;
            unsafe { cif.call_return_into(code, args, Ret::new(&mut ret)) };
            Value::Int16(ret)
        }
        TypeName::Long => {
            let mut ret = 0i32;
            unsafe { cif.call_return_into(code, args, Ret::new(&mut ret)) };
            Value::Int32(ret)
        }
        TypeName::Int64 | TypeName::Variant => {
            let mut ret = 0i64;
            unsafe { cif.call_return_into(code, args, Ret::new(&mut ret)) };
            Value::Int64(ret)
        }
        TypeName::UInt32 => {
            let mut ret = 0u32;
            unsafe { cif.call_return_into(code, args, Ret::new(&mut ret)) };
            Value::UInt32(ret)
        }
        TypeName::UInt64 => {
            let mut ret = 0u64;
            unsafe { cif.call_return_into(code, args, Ret::new(&mut ret)) };
            Value::UInt64(ret)
        }
        TypeName::Boolean => {
            let mut ret = 0i16;
            unsafe { cif.call_return_into(code, args, Ret::new(&mut ret)) };
            Value::Boolean(ret != 0)
        }
        TypeName::Ptr => {
            let mut ret = std::ptr::null_mut::<c_void>();
            unsafe { cif.call_return_into(code, args, Ret::new(&mut ret)) };
            Value::Ptr(ret as usize)
        }
        TypeName::FuncPtr => {
            let mut ret = std::ptr::null_mut::<c_void>();
            unsafe { cif.call_return_into(code, args, Ret::new(&mut ret)) };
            Value::FuncPtr(ret as usize)
        }
        TypeName::Single => {
            let mut ret = 0f32;
            unsafe { cif.call_return_into(code, args, Ret::new(&mut ret)) };
            Value::Single(ret)
        }
        TypeName::Double => {
            let mut ret = 0f64;
            unsafe { cif.call_return_into(code, args, Ret::new(&mut ret)) };
            Value::Double(ret)
        }
        TypeName::Currency => {
            let mut ret = 0i64;
            unsafe { cif.call_return_into(code, args, Ret::new(&mut ret)) };
            Value::Currency(ret)
        }
        TypeName::String
        | TypeName::Decimal
        | TypeName::Date
        | TypeName::User(_)
        | TypeName::Enum(_)
        | TypeName::Array(_) => {
            return Err(unsupported(
                format!(
                    "return type '{}' is not supported by native marshaling",
                    ty.display_name()
                ),
                span,
            ));
        }
    };
    Ok(value)
}

fn library_candidates(lib: &str) -> Vec<PathBuf> {
    let path = Path::new(lib);
    if path.components().count() > 1 || path.is_absolute() {
        return vec![path.to_path_buf()];
    }
    let mut candidates = Vec::new();
    candidates.push(PathBuf::from(lib));
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join(lib));
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        candidates.push(dir.join(lib));
    }
    if let Some(paths) = std::env::var_os("PATH") {
        for path in std::env::split_paths(&paths) {
            candidates.push(path.join(lib));
        }
    }
    for name in platform_names(lib) {
        candidates.push(PathBuf::from(name));
    }
    dedupe_paths(candidates)
}

fn platform_names(lib: &str) -> Vec<String> {
    let mut names = Vec::new();
    #[cfg(windows)]
    {
        if !lib.to_ascii_lowercase().ends_with(".dll") {
            names.push(format!("{lib}.dll"));
        }
        names.push(lib.to_string());
        if lib.eq_ignore_ascii_case("libc") || lib.eq_ignore_ascii_case("libm") {
            names.push("msvcrt.dll".to_string());
        }
    }
    #[cfg(target_os = "macos")]
    {
        if lib.eq_ignore_ascii_case("libc") || lib.eq_ignore_ascii_case("libm") {
            names.push("libSystem.B.dylib".to_string());
        }
        if !lib.ends_with(".dylib") {
            names.push(format!("lib{lib}.dylib"));
            names.push(format!("{lib}.dylib"));
        }
        names.push(format!("/usr/lib/lib{lib}.dylib"));
        names.push(format!("/System/Library/Frameworks/{lib}.framework/{lib}"));
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        match lib {
            #[cfg(target_os = "android")]
            "libc" => {
                names.push("libc.so".to_string());
                names.push("libm.so".to_string());
            }
            #[cfg(target_os = "android")]
            "libm" => {
                names.push("libm.so".to_string());
                names.push("libc.so".to_string());
            }
            #[cfg(not(target_os = "android"))]
            "libc" => names.push("libc.so.6".to_string()),
            #[cfg(not(target_os = "android"))]
            "libm" => names.push("libm.so.6".to_string()),
            _ => {}
        }
        if !lib.contains(".so") {
            names.push(format!("{lib}.so"));
            names.push(format!("lib{lib}.so"));
            if !lib.contains(".so.") {
                names.push(format!("{lib}.so.6"));
                names.push(format!("lib{lib}.so.6"));
            }
        }
        names.push(format!("/lib/{lib}"));
        names.push(format!("/usr/lib/{lib}"));
    }
    names
}

fn dedupe_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut seen = std::collections::HashSet::new();
    paths
        .into_iter()
        .filter(|path| seen.insert(path.display().to_string()))
        .collect()
}

#[cfg(windows)]
unsafe fn open_library(path: &Path) -> Result<*mut c_void, ()> {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn LoadLibraryA(name: *const c_char) -> *mut c_void;
    }
    let c_path = CString::new(path.display().to_string()).map_err(|_| ())?;
    let handle = unsafe { LoadLibraryA(c_path.as_ptr()) };
    if handle.is_null() {
        Err(())
    } else {
        Ok(handle)
    }
}

#[cfg(windows)]
unsafe fn find_symbol(handle: *mut c_void, symbol: *const c_char) -> *mut c_void {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetProcAddress(handle: *mut c_void, name: *const c_char) -> *mut c_void;
    }
    unsafe { GetProcAddress(handle, symbol) }
}

#[cfg(windows)]
unsafe fn close_library(handle: *mut c_void) {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn FreeLibrary(handle: *mut c_void) -> c_int;
    }
    let _ = unsafe { FreeLibrary(handle) };
}

#[cfg(unix)]
unsafe fn open_library(path: &Path) -> Result<*mut c_void, ()> {
    unsafe extern "C" {
        fn dlopen(filename: *const c_char, flags: c_int) -> *mut c_void;
    }
    const RTLD_NOW: c_int = 2;
    let c_path = CString::new(path.display().to_string()).map_err(|_| ())?;
    let handle = unsafe { dlopen(c_path.as_ptr(), RTLD_NOW) };
    if handle.is_null() {
        Err(())
    } else {
        Ok(handle)
    }
}

#[cfg(unix)]
unsafe fn find_symbol(handle: *mut c_void, symbol: *const c_char) -> *mut c_void {
    unsafe extern "C" {
        fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    }
    unsafe { dlsym(handle, symbol) }
}

#[cfg(unix)]
unsafe fn close_library(handle: *mut c_void) {
    unsafe extern "C" {
        fn dlclose(handle: *mut c_void) -> c_int;
    }
    let _ = unsafe { dlclose(handle) };
}
