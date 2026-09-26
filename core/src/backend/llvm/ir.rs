use std::collections::BTreeMap;
use std::fmt::Write;

use crate::frontend::semantics::arithmetic::ArithmeticOp;
use crate::frontend::semantics::type_properties::KnownProperty;
use crate::frontend::semantics::typed_hir::{ArgumentMode, ComparisonOp, Conversion, LocalStorage};
use crate::frontend::type_model::TypeName;
use crate::mir::{analysis::ownership, ir as m, verify};

use super::BackendError;
use super::layout::ArrayShapes;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub triple: String,
    pub data_layout: String,
    pub pointer_bits: u8,
}

impl Target {
    pub fn from_clang_ir(text: &str) -> Result<Self, BackendError> {
        let extract = |prefix: &str| {
            text.lines().find_map(|line| {
                line.strip_prefix(prefix)
                    .and_then(|value| value.strip_suffix('"'))
            })
        };
        let triple = extract("target triple = \"")
            .ok_or_else(|| BackendError::new("target", "clang did not report a target triple"))?;
        let data_layout = extract("target datalayout = \"").ok_or_else(|| {
            BackendError::new("target", "clang did not report an LLVM data layout")
        })?;
        let pointer_bits = if triple.starts_with("x86_64-") || triple.starts_with("aarch64-") {
            64
        } else if triple.starts_with("i686-") {
            32
        } else {
            return Err(BackendError::new(
                "target",
                format!("unsupported host architecture in {triple}"),
            ));
        };
        Ok(Self {
            triple: triple.into(),
            data_layout: data_layout.into(),
            pointer_bits,
        })
    }
}

#[derive(Debug, Clone)]
struct Signature {
    result: TypeName,
    parameters: Vec<(TypeName, LocalStorage)>,
    symbol: String,
}

struct NativeTypes<'a> {
    structures: &'a [TypeName],
    classes: &'a [TypeName],
    fields: &'a [m::Field],
}

impl NativeTypes<'_> {
    fn has_managed_field(&self, ty: &TypeName, visiting: &mut Vec<usize>) -> bool {
        match ty {
            TypeName::String | TypeName::Variant => true,
            TypeName::Tuple(elements) => elements
                .iter()
                .any(|element| self.has_managed_field(&element.ty, visiting)),
            TypeName::Array(element) => self.has_managed_field(element, visiting),
            TypeName::User(_) => {
                if matches!(ty, TypeName::User(name) if name.eq_ignore_ascii_case(crate::runtime::well_known::COLLECTION))
                {
                    return true;
                }
                if self.class_id(ty).is_some() {
                    return true;
                }
                let Some(id) = self.structure_id(ty) else {
                    return false;
                };
                if visiting.contains(&id) {
                    return false;
                }
                visiting.push(id);
                let result = self
                    .members(ty)
                    .iter()
                    .any(|field| self.has_managed_field(&field.ty, visiting));
                visiting.pop();
                result
            }
            _ => false,
        }
    }

    fn ty(&self, ty: &TypeName) -> Result<String, BackendError> {
        match ty {
            TypeName::User(name)
                if name.eq_ignore_ascii_case(crate::runtime::well_known::COLLECTION) =>
            {
                Ok("ptr".into())
            }
            TypeName::User(_) => self
                .structure_id(ty)
                .map(|id| format!("%valo_t{id}"))
                .or_else(|| self.class_id(ty).map(|_| "ptr".to_string()))
                .ok_or_else(|| {
                    BackendError::new(
                        "native eligibility",
                        format!("type {ty:?} has no native value layout"),
                    )
                }),
            TypeName::Tuple(elements) => Ok(format!(
                "{{ {} }}",
                elements
                    .iter()
                    .map(|element| self.ty(&element.ty))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ")
            )),
            TypeName::Array(element) => {
                self.ty(element)?;
                Ok("{ i64, ptr }".into())
            }
            _ => native_type(ty).map(str::to_string),
        }
    }

    fn structure_id(&self, owner: &TypeName) -> Option<usize> {
        self.structures
            .iter()
            .position(|candidate| candidate.same_type(owner))
    }

    fn class_id(&self, owner: &TypeName) -> Option<usize> {
        self.classes
            .iter()
            .position(|candidate| candidate.same_type(owner))
    }

    fn members(&self, owner: &TypeName) -> Vec<&m::Field> {
        self.fields
            .iter()
            .filter(|field| field.owner.same_type(owner))
            .collect()
    }

    fn field_index(
        &self,
        id: crate::frontend::semantics::typed_hir::FieldId,
        owner: &TypeName,
    ) -> Result<usize, BackendError> {
        self.members(owner)
            .iter()
            .position(|field| field.id == id)
            .ok_or_else(|| {
                BackendError::new(
                    "native eligibility",
                    "resolved field has no native layout index",
                )
            })
    }

    fn definitions(&self) -> Result<String, BackendError> {
        let mut out = String::new();
        for (id, structure) in self.structures.iter().enumerate() {
            self.check_acyclic(structure, &mut Vec::new())?;
            if self
                .members(structure)
                .iter()
                .any(|member| matches!(member.ty, TypeName::Array(_)))
            {
                return Err(BackendError::new(
                    "native eligibility",
                    "array fields in Structures need a native ownership contract",
                ));
            }
            let members = self
                .members(structure)
                .iter()
                .map(|member| self.ty(&member.ty))
                .collect::<Result<Vec<_>, _>>()?;
            writeln!(out, "%valo_t{id} = type {{ {} }}", members.join(", ")).unwrap();
        }
        for (id, class) in self.classes.iter().enumerate() {
            let members = self
                .members(class)
                .iter()
                .map(|member| self.ty(&member.ty))
                .collect::<Result<Vec<_>, _>>()?;
            let suffix = if members.is_empty() {
                String::new()
            } else {
                format!(", {}", members.join(", "))
            };
            writeln!(out, "%valo_c{id} = type {{ i64, ptr{suffix} }}").unwrap();
        }
        Ok(out)
    }

    fn check_acyclic(&self, ty: &TypeName, path: &mut Vec<String>) -> Result<(), BackendError> {
        match ty {
            TypeName::User(name) => {
                if self.class_id(ty).is_some() {
                    return Ok(());
                }
                if path.iter().any(|item| item.eq_ignore_ascii_case(name)) {
                    return Err(BackendError::new(
                        "native eligibility",
                        format!("recursive value Structure {name} has infinite size"),
                    ));
                }
                if self.structure_id(ty).is_none() {
                    return self.ty(ty).map(|_| ());
                }
                path.push(name.clone());
                for field in self.members(ty) {
                    self.check_acyclic(&field.ty, path)?;
                }
                path.pop();
            }
            TypeName::Tuple(elements) => {
                for element in elements {
                    self.check_acyclic(&element.ty, path)?;
                }
            }
            _ => {
                self.ty(ty)?;
            }
        }
        Ok(())
    }
}

fn native_type(ty: &TypeName) -> Result<&'static str, BackendError> {
    Ok(match ty {
        TypeName::Void => "void",
        TypeName::Byte => "i8",
        TypeName::Int16 => "i16",
        TypeName::Int32 | TypeName::UInt32 => "i32",
        TypeName::Int64 | TypeName::UInt64 => "i64",
        TypeName::Single => "float",
        TypeName::Double => "double",
        TypeName::Boolean => "i1",
        TypeName::String | TypeName::Variant => "ptr",
        _ => {
            return Err(BackendError::new(
                "native eligibility",
                format!("type {ty:?} has no native primitive ABI yet"),
            ));
        }
    })
}

fn signed(ty: &TypeName) -> bool {
    matches!(ty, TypeName::Int16 | TypeName::Int32 | TypeName::Int64)
}

fn floating(ty: &TypeName) -> bool {
    matches!(ty, TypeName::Single | TypeName::Double)
}

fn bits(ty: &TypeName) -> Option<u16> {
    ty.numeric_bits().or(if *ty == TypeName::Boolean {
        Some(1)
    } else {
        None
    })
}

fn symbol(identity: &str) -> String {
    let mut out = String::from("@valo_");
    for byte in identity.as_bytes() {
        use std::fmt::Write as _;
        write!(out, "{byte:02x}").expect("hex formatting");
    }
    out
}

fn parameter_list(
    signature: &Signature,
    names: bool,
    types: &NativeTypes<'_>,
) -> Result<String, BackendError> {
    signature
        .parameters
        .iter()
        .enumerate()
        .map(|(i, (ty, storage))| {
            let llvm = if *storage == LocalStorage::Value {
                types.ty(ty)?
            } else {
                "ptr".to_string()
            };
            Ok(if names {
                format!("{llvm} %p{i}")
            } else {
                llvm.to_string()
            })
        })
        .collect::<Result<Vec<_>, BackendError>>()
        .map(|v| v.join(", "))
}

/// Eligibility is deliberately module-wide for the first backend. This also
/// validates every resolved call against the declarations collected in pass 1.
fn signatures(
    module: &m::Module,
    types: &NativeTypes<'_>,
) -> Result<Vec<Option<Signature>>, BackendError> {
    let mut native_symbols = std::collections::BTreeSet::new();
    let capacity = module
        .functions
        .iter()
        .map(|f| f.id.0)
        .max()
        .map_or(0, |id| id + 1);
    let mut result = vec![None; capacity];
    for f in &module.functions {
        verify::verify(f).map_err(|e| BackendError::new("MIR verification", e))?;
        ownership::analyze(f).map_err(|e| BackendError::new("MIR dataflow", e))?;
        types.ty(&f.return_type)?;
        if matches!(f.return_type, TypeName::Array(_)) {
            return Err(BackendError::new(
                "native eligibility",
                "array return ABI is not yet supported",
            ));
        }
        for local in &f.locals {
            types.ty(&local.ty)?;
            if matches!(local.ty, TypeName::Array(_))
                && types.has_managed_field(&local.ty, &mut Vec::new())
            {
                return Err(BackendError::new(
                    "native eligibility",
                    "managed fixed arrays require element copy/Drop elaboration",
                ));
            }
            if matches!(local.ty, TypeName::User(_) | TypeName::Tuple(_))
                && (local.properties.copy != KnownProperty::Yes
                    || local.properties.requires_drop == KnownProperty::Unknown)
            {
                return Err(BackendError::new(
                    "native eligibility",
                    "aggregate has unknown or non-Copy ownership requirements",
                ));
            }
        }
        for ty in &f.temps {
            types.ty(ty)?;
            if matches!(ty, TypeName::Array(_)) && types.has_managed_field(ty, &mut Vec::new()) {
                return Err(BackendError::new(
                    "native eligibility",
                    "managed fixed-array temporary requires element copy/Drop elaboration",
                ));
            }
        }
        let mut parameters = f
            .locals
            .iter()
            .filter_map(|l| l.parameter_index.map(|i| (i, l.ty.clone(), l.storage)))
            .collect::<Vec<_>>();
        if parameters
            .iter()
            .any(|(_, ty, _)| matches!(ty, TypeName::Array(_)))
        {
            return Err(BackendError::new(
                "native eligibility",
                "array parameter ABI is not yet supported",
            ));
        }
        parameters.sort_by_key(|(i, _, _)| *i);
        if parameters
            .iter()
            .enumerate()
            .any(|(position, (index, _, _))| *index != position)
        {
            return Err(BackendError::new(
                "native eligibility",
                format!("function {} has a non-contiguous parameter list", f.name),
            ));
        }
        let signature = Signature {
            result: f.return_type.clone(),
            symbol: symbol(&f.symbol_name),
            parameters: parameters
                .into_iter()
                .map(|(_, ty, storage)| (ty, storage))
                .collect(),
        };
        if !native_symbols.insert(signature.symbol.clone()) {
            return Err(BackendError::new(
                "native eligibility",
                format!("duplicate native semantic symbol for {}", f.name),
            ));
        }
        if result[f.id.0].replace(signature).is_some() {
            return Err(BackendError::new(
                "native eligibility",
                "duplicate resolved function ID",
            ));
        }
    }
    Ok(result)
}

pub fn render_module(module: &m::Module, target: &Target) -> Result<String, BackendError> {
    let mut literals = BTreeMap::<String, usize>::new();
    let mut dynamic_types = Vec::<TypeName>::new();
    for function in &module.functions {
        for block in &function.blocks {
            for instruction in &block.instructions {
                if let m::InstructionKind::Const(m::Constant::String(value)) = &instruction.kind {
                    let next = literals.len();
                    literals.entry(value.clone()).or_insert(next);
                }
                let ty = match &instruction.kind {
                    m::InstructionKind::BoxDynamic { ty, .. }
                    | m::InstructionKind::UnboxDynamic { ty, .. } => Some(ty),
                    _ => None,
                };
                if let Some(ty) = ty
                    && !dynamic_types.iter().any(|existing| existing.same_type(ty))
                {
                    dynamic_types.push(ty.clone());
                }
            }
        }
    }
    let types = NativeTypes {
        structures: module
            .functions
            .first()
            .map_or(&[], |f| f.structures.as_slice()),
        classes: module
            .functions
            .first()
            .map_or(&[], |f| f.classes.as_slice()),
        fields: module
            .functions
            .first()
            .map_or(&[], |f| f.fields.as_slice()),
    };
    let declared = signatures(module, &types)?;
    let main = module
        .entry
        .and_then(|id| module.functions.iter().find(|f| f.id == id))
        .ok_or_else(|| {
            BackendError::new(
                "native eligibility",
                "compilation has no resolved entry point",
            )
        })?;
    if !main.locals.iter().all(|l| l.parameter_index.is_none())
        || !matches!(main.return_type, TypeName::Int32 | TypeName::Void)
    {
        return Err(BackendError::new(
            "native eligibility",
            "entry point must be Sub Main() or Function Main() As Integer",
        ));
    }
    let mut out = String::new();
    writeln!(
        out,
        "; Valo experimental native ABI\ntarget datalayout = \"{}\"\ntarget triple = \"{}\"\n",
        target.data_layout, target.triple
    )
    .unwrap();
    writeln!(out, "declare void @llvm.trap()\n").unwrap();
    writeln!(out, "declare ptr @__valo_string_clone(ptr)\ndeclare void @__valo_string_release(ptr)\ndeclare ptr @__valo_string_concat_consume(ptr, ptr)\ndeclare i32 @__valo_string_compare_consume(ptr, ptr)\ndeclare i32 @__valo_string_len_consume(ptr)\ndeclare ptr @__valo_string_from_i64(i64)\ndeclare ptr @__valo_string_from_u64(i64)\ndeclare ptr @__valo_string_from_fixed(double, i32)\ndeclare ptr @__valo_string_from_bool(i32)\n").unwrap();
    writeln!(out, "declare ptr @__valo_object_alloc(i64, ptr)\ndeclare ptr @__valo_object_retain(ptr)\ndeclare void @__valo_object_release(ptr)\n").unwrap();
    writeln!(out, "declare ptr @__valo_dynamic_alloc(i64, ptr, ptr)\ndeclare ptr @__valo_dynamic_data(ptr)\ndeclare ptr @__valo_dynamic_tag(ptr)\ndeclare ptr @__valo_dynamic_retain(ptr)\ndeclare void @__valo_dynamic_release(ptr)\ndeclare ptr @__valo_collection_new()\ndeclare ptr @__valo_collection_retain(ptr)\ndeclare void @__valo_collection_release(ptr)\ndeclare void @__valo_collection_add_consume(ptr, ptr, i64)\ndeclare i32 @__valo_collection_count(ptr)\ndeclare ptr @__valo_collection_item(ptr, i64)\ndeclare void @__valo_collection_remove(ptr, i64)\ndeclare ptr @__valo_collection_snapshot(ptr)\n").unwrap();
    for (value, id) in &literals {
        if value.is_empty() {
            continue;
        }
        let bytes = value.as_bytes();
        let encoded = bytes
            .iter()
            .map(|byte| format!("\\{byte:02X}"))
            .collect::<String>();
        writeln!(out, "@.valo_string_{id} = private constant {{ i64, i64, i64, [{} x i8] }} {{ i64 -1, i64 {}, i64 {}, [{} x i8] c\"{}\" }}", bytes.len(), bytes.len(), value.chars().count(), bytes.len(), encoded).unwrap();
    }
    writeln!(out, "{}", types.definitions()?).unwrap();
    for (id, ty) in dynamic_types.iter().enumerate() {
        writeln!(out, "@.valo_type_tag_{id} = private global i8 0").unwrap();
        writeln!(
            out,
            "define internal void @__valo_dynamic_drop_{id}(ptr %payload) {{\nentry:"
        )
        .unwrap();
        let mut counter = 0;
        drop_managed_value(&mut out, &types, ty, "%payload", &mut counter)?;
        writeln!(out, "  ret void\n}}\n").unwrap();
    }
    for (id, class) in types.classes.iter().enumerate() {
        writeln!(
            out,
            "define internal void @__valo_class_drop_{id}(ptr %object) {{\nentry:"
        )
        .unwrap();
        let mut counter = 0;
        for (index, field) in types.members(class).iter().enumerate().rev() {
            if !types.has_managed_field(&field.ty, &mut Vec::new()) {
                continue;
            }
            let ptr = format!("%field{index}");
            writeln!(
                out,
                "  {ptr} = getelementptr inbounds %valo_c{id}, ptr %object, i32 0, i32 {}",
                index + 2
            )
            .unwrap();
            drop_managed_value(&mut out, &types, &field.ty, &ptr, &mut counter)?;
        }
        writeln!(out, "  ret void\n}}\n").unwrap();
    }
    for f in &module.functions {
        if f.fields != types.fields
            || f.structures != types.structures
            || f.classes != types.classes
        {
            return Err(BackendError::new(
                "native eligibility",
                "functions disagree on resolved Structure fields",
            ));
        }
        lower_function(&mut out, f, &declared, &types, &literals, &dynamic_types)?;
    }
    if main.return_type == TypeName::Void {
        writeln!(
            out,
            "define i32 @main() {{\nentry:\n  call void {}()\n  ret i32 0\n}}",
            symbol(&main.symbol_name)
        )
        .unwrap();
    } else {
        writeln!(out, "define i32 @main() {{\nentry:\n  %entry_result = call i32 {}()\n  ret i32 %entry_result\n}}", symbol(&main.symbol_name)).unwrap();
    }
    Ok(out)
}

fn lower_function(
    out: &mut String,
    f: &m::Function,
    declared: &[Option<Signature>],
    types: &NativeTypes<'_>,
    literals: &BTreeMap<String, usize>,
    dynamic_types: &[TypeName],
) -> Result<(), BackendError> {
    let arrays = ArrayShapes::analyze(f)?;
    let signature = declared[f.id.0].as_ref().expect("collected");
    writeln!(
        out,
        "define {} {}({}) {{",
        types.ty(&f.return_type)?,
        signature.symbol,
        parameter_list(signature, true, types)?
    )
    .unwrap();
    let mut values = vec![None; f.temps.len()];
    let mut guard_counter = 0usize;
    for block in &f.blocks {
        writeln!(out, "bb{}:", block.id.0).unwrap();
        if block.id == f.entry {
            for local in &f.locals {
                if local.storage == LocalStorage::Value {
                    writeln!(out, "  %l{} = alloca {}", local.id.0, types.ty(&local.ty)?).unwrap();
                    if let Some(index) = local.parameter_index {
                        writeln!(
                            out,
                            "  store {} %p{}, ptr %l{}",
                            types.ty(&local.ty)?,
                            index,
                            local.id.0
                        )
                        .unwrap();
                    }
                }
            }
            for (index, length) in arrays.locals.iter().enumerate() {
                if let Some(length) = length {
                    let TypeName::Array(element) = &f.locals[index].ty else {
                        unreachable!()
                    };
                    let array_ty = format!("[{length} x {}]", types.ty(element)?);
                    writeln!(out, "  %arraylocal{index} = alloca {array_ty}").unwrap();
                    writeln!(out, "  %arrayinit{index} = insertvalue {{ i64, ptr }} {{ i64 {length}, ptr null }}, ptr %arraylocal{index}, 1").unwrap();
                    writeln!(
                        out,
                        "  store {{ i64, ptr }} %arrayinit{index}, ptr %l{index}"
                    )
                    .unwrap();
                }
            }
            for (index, length) in arrays.temps.iter().enumerate() {
                if let Some(length) = length {
                    let TypeName::Array(element) = &f.temps[index] else {
                        unreachable!()
                    };
                    writeln!(
                        out,
                        "  %arraytemp{index} = alloca [{length} x {}]",
                        types.ty(element)?
                    )
                    .unwrap();
                }
            }
        }
        for instruction in &block.instructions {
            lower_instruction(
                out,
                f,
                instruction,
                &mut values,
                declared,
                (types, &arrays, literals, dynamic_types),
                &mut guard_counter,
            )?;
        }
        match &block.terminator.as_ref().expect("verified").kind {
            m::TerminatorKind::Goto(id) => writeln!(out, "  br label %bb{}", id.0).unwrap(),
            m::TerminatorKind::Branch {
                condition,
                then_block,
                else_block,
            } => writeln!(
                out,
                "  br i1 {}, label %bb{}, label %bb{}",
                value(&values, *condition)?,
                then_block.0,
                else_block.0
            )
            .unwrap(),
            m::TerminatorKind::Return(id) => writeln!(
                out,
                "  ret {} {}",
                types.ty(&f.return_type)?,
                value(&values, *id)?
            )
            .unwrap(),
            m::TerminatorKind::ReturnVoid => writeln!(out, "  ret void").unwrap(),
            m::TerminatorKind::Unreachable => writeln!(out, "  unreachable").unwrap(),
            m::TerminatorKind::Trap(_) => {
                writeln!(out, "  call void @llvm.trap()\n  unreachable").unwrap()
            }
        }
    }
    writeln!(out, "}}\n").unwrap();
    Ok(())
}

fn value(values: &[Option<String>], id: m::TempId) -> Result<&str, BackendError> {
    values.get(id.0).and_then(Option::as_deref).ok_or_else(|| {
        BackendError::new(
            "LLVM lowering",
            format!("MIR temp %{} is unavailable", id.0),
        )
    })
}

fn place_ptr(
    out: &mut String,
    f: &m::Function,
    place: &m::Place,
    types: &NativeTypes<'_>,
    values: &[Option<String>],
    counter: &mut usize,
) -> Result<String, BackendError> {
    let local = &f.locals[place.root.0];
    let mut ptr = if local.storage == LocalStorage::Value {
        format!("%l{}", place.root.0)
    } else {
        format!("%p{}", local.parameter_index.expect("borrowed parameter"))
    };
    let mut ty = local.ty.clone();
    for projection in &place.projections {
        if let m::Projection::Index(index) = projection {
            let TypeName::Array(element) = &ty else {
                unreachable!("verified MIR")
            };
            let id = *counter;
            *counter += 1;
            writeln!(out, "  %arrdesc{id} = load {{ i64, ptr }}, ptr {ptr}\n  %arrlen{id} = extractvalue {{ i64, ptr }} %arrdesc{id}, 0\n  %arrdata{id} = extractvalue {{ i64, ptr }} %arrdesc{id}, 1").unwrap();
            let index_ty = &f.temps[index.0];
            let index_value = value(values, *index)?;
            let index64 = if types.ty(index_ty)? == "i64" {
                index_value.to_string()
            } else {
                let opcode = if signed(index_ty) { "sext" } else { "zext" };
                writeln!(
                    out,
                    "  %arrindex{id} = {opcode} {} {index_value} to i64",
                    types.ty(index_ty)?
                )
                .unwrap();
                format!("%arrindex{id}")
            };
            writeln!(out, "  %arrvalid{id} = icmp ult i64 {index64}, %arrlen{id}\n  br i1 %arrvalid{id}, label %arrok{id}, label %arrtrap{id}\narrtrap{id}:\n  call void @llvm.trap()\n  unreachable\narrok{id}:\n  %arrelem{id} = getelementptr inbounds {}, ptr %arrdata{id}, i64 {index64}", types.ty(element)?).unwrap();
            ptr = format!("%arrelem{id}");
            ty = *element.clone();
            continue;
        }
        let (index, next) = match projection {
            m::Projection::Field(id) => {
                let field = &f.fields[id.0];
                (types.field_index(*id, &ty)?, field.ty.clone())
            }
            m::Projection::TupleField(index) => {
                let TypeName::Tuple(elements) = &ty else {
                    unreachable!("verified MIR")
                };
                (*index, elements[*index].ty.clone())
            }
            m::Projection::Index(_) => unreachable!(),
        };
        let name = format!("%place{}", *counter);
        *counter += 1;
        if let Some(class) = types.class_id(&ty) {
            let id = *counter;
            *counter += 1;
            writeln!(out, "  %object{id} = load ptr, ptr {ptr}\n  %objectnull{id} = icmp eq ptr %object{id}, null\n  br i1 %objectnull{id}, label %objecttrap{id}, label %objectok{id}\nobjecttrap{id}:\n  call void @llvm.trap()\n  unreachable\nobjectok{id}:").unwrap();
            ptr = format!("%object{id}");
            writeln!(
                out,
                "  {name} = getelementptr inbounds %valo_c{class}, ptr {ptr}, i32 0, i32 {}",
                index + 2
            )
            .unwrap();
            ptr = name;
            ty = next;
            continue;
        }
        writeln!(
            out,
            "  {name} = getelementptr inbounds {}, ptr {ptr}, i32 0, i32 {index}",
            types.ty(&ty)?
        )
        .unwrap();
        ptr = name;
        ty = next;
    }
    Ok(ptr)
}

/// Clone one owned value. Trivial fields stay in the original SSA aggregate;
/// managed fields are replaced with their freshly retained counterparts.
fn clone_managed_value(
    out: &mut String,
    types: &NativeTypes<'_>,
    ty: &TypeName,
    source: &str,
    counter: &mut usize,
) -> Result<String, BackendError> {
    if *ty == TypeName::Variant {
        let id = *counter;
        *counter += 1;
        writeln!(
            out,
            "  %managed{id} = call ptr @__valo_dynamic_retain(ptr {source})"
        )
        .unwrap();
        return Ok(format!("%managed{id}"));
    }
    if matches!(ty, TypeName::User(name) if name.eq_ignore_ascii_case(crate::runtime::well_known::COLLECTION))
    {
        let id = *counter;
        *counter += 1;
        writeln!(
            out,
            "  %managed{id} = call ptr @__valo_collection_retain(ptr {source})"
        )
        .unwrap();
        return Ok(format!("%managed{id}"));
    }
    if types.class_id(ty).is_some() {
        let id = *counter;
        *counter += 1;
        writeln!(
            out,
            "  %managed{id} = call ptr @__valo_object_retain(ptr {source})"
        )
        .unwrap();
        return Ok(format!("%managed{id}"));
    }
    if *ty == TypeName::String {
        let id = *counter;
        *counter += 1;
        writeln!(
            out,
            "  %managed{id} = call ptr @__valo_string_clone(ptr {source})"
        )
        .unwrap();
        return Ok(format!("%managed{id}"));
    }
    let fields: Vec<(usize, TypeName)> = match ty {
        TypeName::User(_) => types
            .members(ty)
            .iter()
            .enumerate()
            .map(|(index, field)| (index, field.ty.clone()))
            .collect(),
        TypeName::Tuple(elements) => elements
            .iter()
            .enumerate()
            .map(|(index, field)| (index, field.ty.clone()))
            .collect(),
        TypeName::Array(_) => {
            return Err(BackendError::new(
                "native eligibility",
                "managed fixed-array copy requires element elaboration",
            ));
        }
        _ => return Ok(source.to_string()),
    };
    let mut current = source.to_string();
    for (index, field) in fields {
        if !types.has_managed_field(&field, &mut Vec::new()) {
            continue;
        }
        let id = *counter;
        *counter += 1;
        writeln!(
            out,
            "  %managedfield{id} = extractvalue {} {source}, {index}",
            types.ty(ty)?
        )
        .unwrap();
        let retained =
            clone_managed_value(out, types, &field, &format!("%managedfield{id}"), counter)?;
        let next = *counter;
        *counter += 1;
        writeln!(
            out,
            "  %managed{next} = insertvalue {} {current}, {} {retained}, {index}",
            types.ty(ty)?,
            types.ty(&field)?
        )
        .unwrap();
        current = format!("%managed{next}");
    }
    Ok(current)
}

/// Destroy managed fields in reverse declaration order. The caller owns the
/// pointed-to value and must have proved it initialized before this operation.
fn drop_managed_value(
    out: &mut String,
    types: &NativeTypes<'_>,
    ty: &TypeName,
    ptr: &str,
    counter: &mut usize,
) -> Result<(), BackendError> {
    if *ty == TypeName::Variant
        || matches!(ty, TypeName::User(name) if name.eq_ignore_ascii_case(crate::runtime::well_known::COLLECTION))
    {
        let helper = if *ty == TypeName::Variant {
            "dynamic"
        } else {
            "collection"
        };
        let id = *counter;
        *counter += 1;
        writeln!(out, "  %managedold{id} = load ptr, ptr {ptr}\n  call void @__valo_{helper}_release(ptr %managedold{id})\n  store ptr null, ptr {ptr}").unwrap();
        return Ok(());
    }
    if types.class_id(ty).is_some() {
        let id = *counter;
        *counter += 1;
        writeln!(out, "  %managedold{id} = load ptr, ptr {ptr}\n  call void @__valo_object_release(ptr %managedold{id})\n  store ptr null, ptr {ptr}").unwrap();
        return Ok(());
    }
    if *ty == TypeName::String {
        let id = *counter;
        *counter += 1;
        writeln!(out, "  %managedold{id} = load ptr, ptr {ptr}\n  call void @__valo_string_release(ptr %managedold{id})\n  store ptr null, ptr {ptr}").unwrap();
        return Ok(());
    }
    let fields: Vec<(usize, TypeName)> = match ty {
        TypeName::User(_) => types
            .members(ty)
            .iter()
            .enumerate()
            .map(|(index, field)| (index, field.ty.clone()))
            .collect(),
        TypeName::Tuple(elements) => elements
            .iter()
            .enumerate()
            .map(|(index, field)| (index, field.ty.clone()))
            .collect(),
        TypeName::Array(_) => {
            return Err(BackendError::new(
                "native eligibility",
                "managed fixed-array Drop requires element elaboration",
            ));
        }
        _ => return Ok(()),
    };
    for (index, field) in fields.into_iter().rev() {
        if !types.has_managed_field(&field, &mut Vec::new()) {
            continue;
        }
        let id = *counter;
        *counter += 1;
        writeln!(
            out,
            "  %managedptr{id} = getelementptr inbounds {}, ptr {ptr}, i32 0, i32 {index}",
            types.ty(ty)?
        )
        .unwrap();
        drop_managed_value(out, types, &field, &format!("%managedptr{id}"), counter)?;
    }
    Ok(())
}

fn lower_instruction(
    out: &mut String,
    f: &m::Function,
    ins: &m::Instruction,
    values: &mut [Option<String>],
    declared: &[Option<Signature>],
    layout: (
        &NativeTypes<'_>,
        &ArrayShapes,
        &BTreeMap<String, usize>,
        &[TypeName],
    ),
    guard_counter: &mut usize,
) -> Result<(), BackendError> {
    let (types, arrays, literals, dynamic_types) = layout;
    let result = ins.result.map(|id| format!("%t{}", id.0));
    let result_ty = ins.result.map(|id| &f.temps[id.0]);
    match &ins.kind {
        m::InstructionKind::Const(constant) => {
            let text = match constant {
                m::Constant::ZeroAggregate => "zeroinitializer".into(),
                m::Constant::Integer(n) => n.to_string(),
                m::Constant::Boolean(b) => b.to_string(),
                m::Constant::Single(n) => float_literal(f64::from(*n)),
                m::Constant::Double(n) => float_literal(*n),
                m::Constant::String(value) => {
                    if value.is_empty() {
                        "null".into()
                    } else {
                        format!("@.valo_string_{}", literals[value])
                    }
                }
                m::Constant::NullReference => "null".into(),
            };
            values[ins.result.expect("verified").0] = Some(text);
            return Ok(());
        }
        m::InstructionKind::NewClass(ty) => {
            let id = types.class_id(ty).ok_or_else(|| {
                BackendError::new(
                    "native eligibility",
                    "Class allocation has no resolved layout",
                )
            })?;
            writeln!(out, "  {} = call ptr @__valo_object_alloc(i64 ptrtoint (ptr getelementptr (%valo_c{id}, ptr null, i32 1) to i64), ptr @__valo_class_drop_{id})", result.as_ref().expect("verified")).unwrap();
        }
        m::InstructionKind::NewCollection => {
            writeln!(
                out,
                "  {} = call ptr @__valo_collection_new()",
                result.as_ref().expect("verified")
            )
            .unwrap();
        }
        m::InstructionKind::SnapshotCollection(source) => {
            let handle = value(values, *source)?.to_string();
            writeln!(out, "  {} = call ptr @__valo_collection_snapshot(ptr {handle})\n  call void @__valo_collection_release(ptr {handle})", result.as_ref().expect("verified")).unwrap();
        }
        m::InstructionKind::BoxDynamic { value: source, ty } => {
            let tag = dynamic_types
                .iter()
                .position(|item| item.same_type(ty))
                .ok_or_else(|| BackendError::new("LLVM lowering", "dynamic type tag is missing"))?;
            let id = *guard_counter;
            *guard_counter += 1;
            let boxed = result.as_ref().expect("verified");
            let llvm_ty = types.ty(ty)?;
            writeln!(out, "  {boxed} = call ptr @__valo_dynamic_alloc(i64 ptrtoint (ptr getelementptr ({llvm_ty}, ptr null, i32 1) to i64), ptr @.valo_type_tag_{tag}, ptr @__valo_dynamic_drop_{tag})\n  %dynamicdata{id} = call ptr @__valo_dynamic_data(ptr {boxed})\n  store {llvm_ty} {}, ptr %dynamicdata{id}", value(values, *source)?).unwrap();
        }
        m::InstructionKind::UnboxDynamic { value: source, ty } => {
            let tag = dynamic_types
                .iter()
                .position(|item| item.same_type(ty))
                .ok_or_else(|| BackendError::new("LLVM lowering", "dynamic type tag is missing"))?;
            let id = *guard_counter;
            *guard_counter += 1;
            let boxed = value(values, *source)?.to_string();
            writeln!(out, "  %dynamictag{id} = call ptr @__valo_dynamic_tag(ptr {boxed})\n  %dynamicmatch{id} = icmp eq ptr %dynamictag{id}, @.valo_type_tag_{tag}\n  br i1 %dynamicmatch{id}, label %dynamicok{id}, label %dynamictrap{id}\ndynamictrap{id}:\n  call void @llvm.trap()\n  unreachable\ndynamicok{id}:\n  %dynamicdata{id} = call ptr @__valo_dynamic_data(ptr {boxed})\n  %dynamicload{id} = load {}, ptr %dynamicdata{id}", types.ty(ty)?).unwrap();
            let cloned =
                clone_managed_value(out, types, ty, &format!("%dynamicload{id}"), guard_counter)?;
            writeln!(out, "  call void @__valo_dynamic_release(ptr {boxed})").unwrap();
            values[ins.result.expect("verified").0] = Some(cloned);
            return Ok(());
        }
        m::InstructionKind::CollectionCount(collection) => {
            let handle = value(values, *collection)?.to_string();
            writeln!(out, "  {} = call i32 @__valo_collection_count(ptr {handle})\n  call void @__valo_collection_release(ptr {handle})", result.as_ref().expect("verified")).unwrap();
        }
        m::InstructionKind::CollectionItem { collection, index } => {
            let handle = value(values, *collection)?.to_string();
            let id = *guard_counter;
            *guard_counter += 1;
            let index_ty = types.ty(&f.temps[index.0])?;
            let index_value = if index_ty == "i64" {
                value(values, *index)?.to_string()
            } else {
                writeln!(
                    out,
                    "  %collectionindex{id} = sext {index_ty} {} to i64",
                    value(values, *index)?
                )
                .unwrap();
                format!("%collectionindex{id}")
            };
            writeln!(out, "  {} = call ptr @__valo_collection_item(ptr {handle}, i64 {index_value})\n  call void @__valo_collection_release(ptr {handle})", result.as_ref().expect("verified")).unwrap();
        }
        m::InstructionKind::CollectionAdd {
            collection,
            item,
            before,
        } => {
            let handle = value(values, *collection)?.to_string();
            let position = before
                .map(|id| value(values, id))
                .transpose()?
                .unwrap_or("0");
            writeln!(out, "  call void @__valo_collection_add_consume(ptr {handle}, ptr {}, i64 {position})\n  call void @__valo_collection_release(ptr {handle})", value(values, *item)?).unwrap();
        }
        m::InstructionKind::CollectionRemove { collection, index } => {
            let handle = value(values, *collection)?.to_string();
            writeln!(out, "  call void @__valo_collection_remove(ptr {handle}, i64 {})\n  call void @__valo_collection_release(ptr {handle})", value(values, *index)?).unwrap();
        }
        m::InstructionKind::TupleInit(elements) => {
            let temp = ins.result.expect("verified").0;
            let tuple_ty = types.ty(&f.temps[temp])?;
            let mut previous = "zeroinitializer".to_string();
            for (index, element) in elements.iter().enumerate() {
                let next = if index + 1 == elements.len() {
                    format!("%t{temp}")
                } else {
                    format!("%tuple{temp}_{index}")
                };
                writeln!(
                    out,
                    "  {next} = insertvalue {tuple_ty} {previous}, {} {}, {index}",
                    types.ty(&f.temps[element.0])?,
                    value(values, *element)?
                )
                .unwrap();
                previous = next;
            }
        }
        m::InstructionKind::Load(place) | m::InstructionKind::Move(place) => {
            if matches!(ins.kind, m::InstructionKind::Move(_))
                && f.locals[place.root.0].properties.copy != KnownProperty::Yes
            {
                return Err(BackendError::new(
                    "native eligibility",
                    "ownership-sensitive Move is not supported",
                ));
            }
            let ptr = place_ptr(out, f, place, types, values, guard_counter)?;
            if place.ty == TypeName::String {
                let id = *guard_counter;
                *guard_counter += 1;
                writeln!(out, "  %stringmove{id} = load ptr, ptr {ptr}\n  {} = call ptr @__valo_string_clone(ptr %stringmove{id})", result.as_ref().expect("verified")).unwrap();
            } else {
                writeln!(
                    out,
                    "  {} = load {}, ptr {}",
                    result.as_ref().expect("verified"),
                    types.ty(&place.ty)?,
                    ptr
                )
                .unwrap();
            }
        }
        m::InstructionKind::CloneString(place) => {
            let ptr = place_ptr(out, f, place, types, values, guard_counter)?;
            let id = *guard_counter;
            *guard_counter += 1;
            writeln!(out, "  %stringload{id} = load ptr, ptr {ptr}\n  {} = call ptr @__valo_string_clone(ptr %stringload{id})", result.as_ref().expect("verified")).unwrap();
        }
        m::InstructionKind::CloneManaged(place) => {
            let ptr = place_ptr(out, f, place, types, values, guard_counter)?;
            let id = *guard_counter;
            *guard_counter += 1;
            writeln!(
                out,
                "  %managedload{id} = load {}, ptr {ptr}",
                types.ty(&place.ty)?
            )
            .unwrap();
            let cloned = clone_managed_value(
                out,
                types,
                &place.ty,
                &format!("%managedload{id}"),
                guard_counter,
            )?;
            values[ins.result.expect("verified").0] = Some(cloned);
            return Ok(());
        }
        m::InstructionKind::StringConcat { left, right } => {
            writeln!(
                out,
                "  {} = call ptr @__valo_string_concat_consume(ptr {}, ptr {})",
                result.as_ref().expect("verified"),
                value(values, *left)?,
                value(values, *right)?
            )
            .unwrap();
        }
        m::InstructionKind::StringCompare {
            op,
            left,
            right,
            text,
        } => {
            if *text {
                return Err(BackendError::new(
                    "native eligibility",
                    "Option Compare Text string comparisons are not yet supported natively",
                ));
            }
            let id = *guard_counter;
            *guard_counter += 1;
            let predicate = match op {
                ComparisonOp::Equal => "eq",
                ComparisonOp::NotEqual => "ne",
                ComparisonOp::Less => "slt",
                ComparisonOp::LessEqual => "sle",
                ComparisonOp::Greater => "sgt",
                ComparisonOp::GreaterEqual => "sge",
            };
            writeln!(out, "  %stringcmp{id} = call i32 @__valo_string_compare_consume(ptr {}, ptr {})\n  {} = icmp {predicate} i32 %stringcmp{id}, 0", value(values, *left)?, value(values, *right)?, result.as_ref().expect("verified")).unwrap();
        }
        m::InstructionKind::ReferenceIdentity {
            left,
            right,
            negated,
        } => {
            let predicate = if *negated { "ne" } else { "eq" };
            writeln!(
                out,
                "  {} = icmp {predicate} ptr {}, {}",
                result.as_ref().expect("verified"),
                value(values, *left)?,
                value(values, *right)?
            )
            .unwrap();
            writeln!(out, "  call void @__valo_object_release(ptr {})\n  call void @__valo_object_release(ptr {})", value(values, *left)?, value(values, *right)?).unwrap();
        }
        m::InstructionKind::StringLen(source) => {
            writeln!(
                out,
                "  {} = call i32 @__valo_string_len_consume(ptr {})",
                result.as_ref().expect("verified"),
                value(values, *source)?
            )
            .unwrap();
        }
        m::InstructionKind::StringFormat {
            value: source,
            decimals,
        } => {
            let source_ty = &f.temps[source.0];
            let source_value = value(values, *source)?;
            let id = *guard_counter;
            *guard_counter += 1;
            let output = result.as_ref().expect("verified");
            match (source_ty, decimals) {
                (TypeName::Single, Some(digits)) => {
                    writeln!(out, "  %stringfloat{id} = fpext float {source_value} to double\n  {output} = call ptr @__valo_string_from_fixed(double %stringfloat{id}, i32 {digits})").unwrap();
                }
                (TypeName::Double, Some(digits)) => writeln!(out, "  {output} = call ptr @__valo_string_from_fixed(double {source_value}, i32 {digits})").unwrap(),
                (TypeName::Boolean, None) => {
                    writeln!(out, "  %stringbool{id} = zext i1 {source_value} to i32\n  {output} = call ptr @__valo_string_from_bool(i32 %stringbool{id})").unwrap();
                }
                (ty, None) if ty.is_integral() => {
                    let signed = signed(ty);
                    let operation = if signed { "sext" } else { "zext" };
                    let wide = if *ty == TypeName::Int64 || *ty == TypeName::UInt64 { source_value.to_string() } else {
                        writeln!(out, "  %stringint{id} = {operation} {} {source_value} to i64", types.ty(ty)?).unwrap();
                        format!("%stringint{id}")
                    };
                    writeln!(out, "  {output} = call ptr @__valo_string_from_{}(i64 {wide})", if signed { "i64" } else { "u64" }).unwrap();
                }
                _ => return Err(BackendError::new("native eligibility", "this interpolation format is not supported by the native String runtime")),
            }
        }
        m::InstructionKind::Store {
            place,
            value: source,
        } => {
            let ptr = place_ptr(out, f, place, types, values, guard_counter)?;
            if let TypeName::Array(element) = &place.ty {
                if !place.projections.is_empty() {
                    return Err(BackendError::new(
                        "native eligibility",
                        "projected array assignment is not supported",
                    ));
                }
                let length = arrays.locals[place.root.0].expect("array shape checked");
                if arrays.temps[source.0] != Some(length) {
                    return Err(BackendError::new(
                        "native eligibility",
                        "array assignment has mismatched fixed bounds",
                    ));
                }
                let id = *guard_counter;
                *guard_counter += 1;
                let aggregate = format!("[{length} x {}]", types.ty(element)?);
                writeln!(out, "  %copysource{id} = extractvalue {{ i64, ptr }} {}, 1\n  %copyvalue{id} = load {aggregate}, ptr %copysource{id}\n  %copydest{id} = load {{ i64, ptr }}, ptr {ptr}\n  %copytarget{id} = extractvalue {{ i64, ptr }} %copydest{id}, 1\n  store {aggregate} %copyvalue{id}, ptr %copytarget{id}", value(values, *source)?).unwrap();
            } else {
                writeln!(
                    out,
                    "  store {} {}, ptr {}",
                    types.ty(&place.ty)?,
                    value(values, *source)?,
                    ptr
                )
                .unwrap();
            }
        }
        m::InstructionKind::Replace {
            place,
            value: source,
        } => {
            let ptr = place_ptr(out, f, place, types, values, guard_counter)?;
            drop_managed_value(out, types, &place.ty, &ptr, guard_counter)?;
            writeln!(
                out,
                "  store {} {}, ptr {ptr}",
                types.ty(&place.ty)?,
                value(values, *source)?
            )
            .unwrap();
        }
        m::InstructionKind::Arithmetic { op, left, right } => {
            let ty = result_ty.expect("verified");
            let llvm_ty = types.ty(ty)?;
            let a = value(values, *left)?.to_string();
            let b = value(values, *right)?.to_string();
            let float = floating(ty);
            let opcode = match op {
                ArithmeticOp::Add => {
                    if float {
                        "fadd"
                    } else {
                        "add"
                    }
                }
                ArithmeticOp::Subtract => {
                    if float {
                        "fsub"
                    } else {
                        "sub"
                    }
                }
                ArithmeticOp::Multiply => {
                    if float {
                        "fmul"
                    } else {
                        "mul"
                    }
                }
                ArithmeticOp::Divide if float => "fdiv",
                ArithmeticOp::IntegerDivide if !float => {
                    if signed(ty) {
                        "sdiv"
                    } else {
                        "udiv"
                    }
                }
                ArithmeticOp::Modulo if !float => {
                    if signed(ty) {
                        "srem"
                    } else {
                        "urem"
                    }
                }
                _ => {
                    return Err(BackendError::new(
                        "native eligibility",
                        format!("arithmetic {op:?} for {ty:?} is not supported"),
                    ));
                }
            };
            if matches!(
                op,
                ArithmeticOp::Divide | ArithmeticOp::IntegerDivide | ArithmeticOp::Modulo
            ) {
                emit_division_guard(out, &llvm_ty, ty, &a, &b, guard_counter);
            }
            writeln!(
                out,
                "  {} = {opcode} {llvm_ty} {a}, {b}",
                result.as_ref().expect("verified")
            )
            .unwrap();
        }
        m::InstructionKind::Compare { op, left, right } => {
            let ty = &f.temps[left.0];
            let predicate = if floating(ty) {
                match op {
                    ComparisonOp::Equal => "oeq",
                    ComparisonOp::NotEqual => "une",
                    ComparisonOp::Less => "olt",
                    ComparisonOp::LessEqual => "ole",
                    ComparisonOp::Greater => "ogt",
                    ComparisonOp::GreaterEqual => "oge",
                }
            } else {
                match op {
                    ComparisonOp::Equal => "eq",
                    ComparisonOp::NotEqual => "ne",
                    ComparisonOp::Less => {
                        if signed(ty) {
                            "slt"
                        } else {
                            "ult"
                        }
                    }
                    ComparisonOp::LessEqual => {
                        if signed(ty) {
                            "sle"
                        } else {
                            "ule"
                        }
                    }
                    ComparisonOp::Greater => {
                        if signed(ty) {
                            "sgt"
                        } else {
                            "ugt"
                        }
                    }
                    ComparisonOp::GreaterEqual => {
                        if signed(ty) {
                            "sge"
                        } else {
                            "uge"
                        }
                    }
                }
            };
            writeln!(
                out,
                "  {} = {} {predicate} {} {}, {}",
                result.as_ref().expect("verified"),
                if floating(ty) { "fcmp" } else { "icmp" },
                types.ty(ty)?,
                value(values, *left)?,
                value(values, *right)?
            )
            .unwrap();
        }
        m::InstructionKind::Cast {
            value: source,
            conversion,
        } => {
            let from = &f.temps[source.0];
            let to = result_ty.expect("verified");
            let (opcode, guard) = safe_cast(from, to, *conversion)?;
            let source_value = value(values, *source)?.to_string();
            if let Some(guard) = guard {
                emit_cast_guard(out, from, &source_value, guard, guard_counter)?;
            }
            if let Some(opcode) = opcode {
                writeln!(
                    out,
                    "  {} = {opcode} {} {} to {}",
                    result.as_ref().expect("verified"),
                    types.ty(from)?,
                    source_value,
                    types.ty(to)?
                )
                .unwrap();
            } else {
                // A same-width, same-representation conversion needs no code.
                values[ins.result.expect("verified").0] = Some(source_value);
                return Ok(());
            }
        }
        m::InstructionKind::Call {
            target,
            arguments,
            parameter_types,
            parameter_modes,
            return_type,
        } => {
            let m::CallTarget::Function(id) = target else {
                return Err(BackendError::new(
                    "native eligibility",
                    "Dispose requires unresolved native Class semantics",
                ));
            };
            let signature = declared.get(id.0).and_then(Option::as_ref).ok_or_else(|| {
                BackendError::new(
                    "native eligibility",
                    format!("callee #{} has no native body", id.0),
                )
            })?;
            if arguments.len() != signature.parameters.len()
                || if signature.result == TypeName::Void {
                    return_type.is_some()
                } else {
                    !return_type
                        .as_ref()
                        .is_some_and(|ty| ty.same_type(&signature.result))
                }
            {
                return Err(BackendError::new(
                    "native eligibility",
                    "resolved call does not match native callee",
                ));
            }
            let mut args = Vec::new();
            for (index, argument) in arguments.iter().enumerate() {
                let (expected_type, storage) = &signature.parameters[index];
                if !expected_type.same_type(&parameter_types[index]) {
                    return Err(BackendError::new(
                        "native eligibility",
                        "call parameter type differs from native callee",
                    ));
                }
                match (argument, storage, parameter_modes[index]) {
                    (m::CallArgument::Value(id), LocalStorage::Value, ArgumentMode::ByVal) => args
                        .push(format!(
                            "{} {}",
                            types.ty(expected_type)?,
                            value(values, *id)?
                        )),
                    (
                        m::CallArgument::Place { place, .. },
                        LocalStorage::BorrowedMutable,
                        ArgumentMode::BorrowMutable,
                    )
                    | (
                        m::CallArgument::Place { place, .. },
                        LocalStorage::BorrowedImmutable,
                        ArgumentMode::BorrowImmutable,
                    ) => args.push(format!(
                        "ptr {}",
                        place_ptr(out, f, place, types, values, guard_counter)?
                    )),
                    _ => {
                        return Err(BackendError::new(
                            "native eligibility",
                            "call ownership mode differs from native callee",
                        ));
                    }
                }
            }
            if signature.result == TypeName::Void {
                writeln!(out, "  call void {}({})", signature.symbol, args.join(", ")).unwrap();
            } else {
                writeln!(
                    out,
                    "  {} = call {} {}({})",
                    result.as_ref().expect("verified"),
                    types.ty(&signature.result)?,
                    signature.symbol,
                    args.join(", ")
                )
                .unwrap();
            }
        }
        m::InstructionKind::ArrayInit { .. } => {
            let temp = ins.result.expect("verified").0;
            let length = arrays.temps[temp].expect("fixed array shape");
            let TypeName::Array(element) = &f.temps[temp] else {
                unreachable!()
            };
            let aggregate = format!("[{length} x {}]", types.ty(element)?);
            writeln!(out, "  store {aggregate} zeroinitializer, ptr %arraytemp{temp}\n  %t{temp} = insertvalue {{ i64, ptr }} {{ i64 {length}, ptr null }}, ptr %arraytemp{temp}, 1").unwrap();
        }
        m::InstructionKind::ArrayLen(place) => {
            let length = arrays.locals[place.root.0].expect("fixed array shape");
            writeln!(
                out,
                "  {} = add i64 0, {length}",
                result.as_ref().expect("verified")
            )
            .unwrap();
        }
        m::InstructionKind::SnapshotArray(place) => {
            let temp = ins.result.expect("verified").0;
            let length = arrays.temps[temp].expect("fixed array shape");
            let TypeName::Array(element) = &f.temps[temp] else {
                unreachable!()
            };
            let aggregate = format!("[{length} x {}]", types.ty(element)?);
            let ptr = place_ptr(out, f, place, types, values, guard_counter)?;
            let id = *guard_counter;
            *guard_counter += 1;
            writeln!(out, "  %snapdesc{id} = load {{ i64, ptr }}, ptr {ptr}\n  %snapdata{id} = extractvalue {{ i64, ptr }} %snapdesc{id}, 1\n  %snapvalue{id} = load {aggregate}, ptr %snapdata{id}\n  store {aggregate} %snapvalue{id}, ptr %arraytemp{temp}\n  %t{temp} = insertvalue {{ i64, ptr }} {{ i64 {length}, ptr null }}, ptr %arraytemp{temp}, 1").unwrap();
        }
        m::InstructionKind::Drop(place) => {
            if !f.has_managed_fields(&place.ty) {
                return Err(BackendError::new(
                    "native eligibility",
                    "Drop has no supported native destructor for this type",
                ));
            }
            let ptr = place_ptr(out, f, place, types, values, guard_counter)?;
            drop_managed_value(out, types, &place.ty, &ptr, guard_counter)?;
        }
        m::InstructionKind::DropCandidate(_) => {
            return Err(BackendError::new(
                "MIR verification",
                "unelaborated Drop candidate",
            ));
        }
        m::InstructionKind::BorrowStart { .. } | m::InstructionKind::EndBorrow(_) => {
            return Err(BackendError::new(
                "native eligibility",
                "explicit borrow lifetime lowering is not implemented",
            ));
        }
    }
    if let Some(id) = ins.result {
        values[id.0] = result;
    }
    Ok(())
}

fn float_literal(n: f64) -> String {
    if n.is_nan() {
        "0x7FF8000000000000".into()
    } else if n.is_infinite() {
        if n.is_sign_negative() {
            "-0x7FF0000000000000".into()
        } else {
            "0x7FF0000000000000".into()
        }
    } else {
        format!("{n:.17e}")
    }
}

#[derive(Clone, Copy)]
enum CastGuard {
    NonNegative,
    AtMost(i64),
}

fn safe_cast(
    from: &TypeName,
    to: &TypeName,
    conversion: Conversion,
) -> Result<(Option<&'static str>, Option<CastGuard>), BackendError> {
    if from.same_type(to) {
        return Ok((None, None));
    }
    if conversion != Conversion::NumericChecked {
        return Err(BackendError::new(
            "native eligibility",
            "integer-division truncation conversion is not native yet",
        ));
    }
    let (Some(a), Some(b)) = (bits(from), bits(to)) else {
        return Err(BackendError::new("native eligibility", "non-numeric cast"));
    };
    if from.is_integral() && to.is_integral() {
        let sign_change = signed(from) != signed(to);
        let guard = if sign_change && signed(from) {
            Some(CastGuard::NonNegative)
        } else if sign_change && a == b {
            Some(CastGuard::AtMost(if b == 64 {
                i64::MAX
            } else {
                (1i64 << (b - 1)) - 1
            }))
        } else {
            None
        };
        if a == b {
            return Ok((None, guard));
        }
        if b > a {
            return Ok((
                Some(if signed(from) && signed(to) {
                    "sext"
                } else {
                    "zext"
                }),
                guard,
            ));
        }
    }
    if *from == TypeName::Single && *to == TypeName::Double {
        return Ok((Some("fpext"), None));
    }
    if from.is_integral() && floating(to) {
        return Ok((Some(if signed(from) { "sitofp" } else { "uitofp" }), None));
    }
    Err(BackendError::new(
        "native eligibility",
        format!("checked conversion {from:?} -> {to:?} is not yet lowered without semantic loss"),
    ))
}

fn emit_cast_guard(
    out: &mut String,
    from: &TypeName,
    value: &str,
    guard: CastGuard,
    counter: &mut usize,
) -> Result<(), BackendError> {
    let id = *counter;
    *counter += 1;
    let ty = native_type(from)?;
    match guard {
        CastGuard::NonNegative => {
            writeln!(out, "  %castbad{id} = icmp slt {ty} {value}, 0").unwrap()
        }
        CastGuard::AtMost(max) => {
            writeln!(out, "  %castbad{id} = icmp ugt {ty} {value}, {max}").unwrap()
        }
    }
    writeln!(out, "  br i1 %castbad{id}, label %casttrap{id}, label %castok{id}\ncasttrap{id}:\n  call void @llvm.trap()\n  unreachable\ncastok{id}:").unwrap();
    Ok(())
}

fn emit_division_guard(
    out: &mut String,
    llvm_ty: &str,
    ty: &TypeName,
    a: &str,
    b: &str,
    counter: &mut usize,
) {
    let id = *counter;
    *counter += 1;
    let zero = if floating(ty) { "0.0" } else { "0" };
    writeln!(
        out,
        "  %divzero{id} = {} {} {b}, {zero}",
        if floating(ty) { "fcmp oeq" } else { "icmp eq" },
        llvm_ty
    )
    .unwrap();
    if signed(ty) {
        let min = match ty {
            TypeName::Int16 => i16::MIN as i64,
            TypeName::Int32 => i32::MIN as i64,
            _ => i64::MIN,
        };
        writeln!(out, "  %divmin{id} = icmp eq {llvm_ty} {a}, {min}\n  %divneg{id} = icmp eq {llvm_ty} {b}, -1\n  %divov{id} = and i1 %divmin{id}, %divneg{id}\n  %divbad{id} = or i1 %divzero{id}, %divov{id}").unwrap();
        writeln!(
            out,
            "  br i1 %divbad{id}, label %divtrap{id}, label %divok{id}"
        )
        .unwrap();
    } else {
        writeln!(
            out,
            "  br i1 %divzero{id}, label %divtrap{id}, label %divok{id}"
        )
        .unwrap();
    }
    writeln!(
        out,
        "divtrap{id}:\n  call void @llvm.trap()\n  unreachable\ndivok{id}:"
    )
    .unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supported_primitive_types_have_explicit_llvm_widths() {
        for (source, llvm) in [
            (TypeName::Byte, "i8"),
            (TypeName::Int16, "i16"),
            (TypeName::Int32, "i32"),
            (TypeName::UInt32, "i32"),
            (TypeName::Int64, "i64"),
            (TypeName::UInt64, "i64"),
            (TypeName::Single, "float"),
            (TypeName::Double, "double"),
            (TypeName::Boolean, "i1"),
        ] {
            assert_eq!(native_type(&source).unwrap(), llvm);
        }
        assert_eq!(native_type(&TypeName::Variant).unwrap(), "ptr");
        assert!(native_type(&TypeName::User("ClassName".into())).is_err());
    }

    #[test]
    fn checked_conversion_plan_preserves_signedness() {
        assert!(matches!(
            safe_cast(
                &TypeName::Int32,
                &TypeName::Int64,
                Conversion::NumericChecked
            ),
            Ok((Some("sext"), None))
        ));
        assert!(matches!(
            safe_cast(
                &TypeName::UInt32,
                &TypeName::UInt64,
                Conversion::NumericChecked
            ),
            Ok((Some("zext"), None))
        ));
        assert!(matches!(
            safe_cast(
                &TypeName::Int32,
                &TypeName::UInt32,
                Conversion::NumericChecked
            ),
            Ok((None, Some(CastGuard::NonNegative)))
        ));
        assert!(matches!(
            safe_cast(
                &TypeName::UInt32,
                &TypeName::Int32,
                Conversion::NumericChecked
            ),
            Ok((None, Some(CastGuard::AtMost(_))))
        ));
        assert!(
            safe_cast(
                &TypeName::Int64,
                &TypeName::Int32,
                Conversion::NumericChecked
            )
            .is_err()
        );
    }
}
