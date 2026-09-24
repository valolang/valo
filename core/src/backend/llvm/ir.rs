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
}

struct NativeTypes<'a> {
    structures: &'a [TypeName],
    fields: &'a [m::Field],
}

impl NativeTypes<'_> {
    fn ty(&self, ty: &TypeName) -> Result<String, BackendError> {
        match ty {
            TypeName::User(_) => self
                .structure_id(ty)
                .map(|id| format!("%valo_t{id}"))
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
        Ok(out)
    }

    fn check_acyclic(&self, ty: &TypeName, path: &mut Vec<String>) -> Result<(), BackendError> {
        match ty {
            TypeName::User(name) => {
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
        TypeName::Byte => "i8",
        TypeName::Int16 => "i16",
        TypeName::Int32 | TypeName::UInt32 => "i32",
        TypeName::Int64 | TypeName::UInt64 => "i64",
        TypeName::Single => "float",
        TypeName::Double => "double",
        TypeName::Boolean => "i1",
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

fn symbol(id: crate::frontend::semantics::typed_hir::BodyFunctionId) -> String {
    format!("@valo_f{}", id.0)
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
            if matches!(local.ty, TypeName::User(_) | TypeName::Tuple(_))
                && (local.properties.copy != KnownProperty::Yes
                    || local.properties.requires_drop != KnownProperty::No)
            {
                return Err(BackendError::new(
                    "native eligibility",
                    "aggregate has unknown or non-Copy ownership requirements",
                ));
            }
        }
        for ty in &f.temps {
            types.ty(ty)?;
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
            parameters: parameters
                .into_iter()
                .map(|(_, ty, storage)| (ty, storage))
                .collect(),
        };
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
    let types = NativeTypes {
        structures: module
            .functions
            .first()
            .map_or(&[], |f| f.structures.as_slice()),
        fields: module
            .functions
            .first()
            .map_or(&[], |f| f.fields.as_slice()),
    };
    let declared = signatures(module, &types)?;
    let main = module
        .functions
        .iter()
        .filter(|f| f.name.eq_ignore_ascii_case("main"))
        .collect::<Vec<_>>();
    if main.len() != 1
        || !main[0].locals.iter().all(|l| l.parameter_index.is_none())
        || main[0].return_type != TypeName::Int32
    {
        return Err(BackendError::new(
            "native eligibility",
            "exactly one Function Main() As Integer is required",
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
    writeln!(out, "{}", types.definitions()?).unwrap();
    for f in &module.functions {
        if f.fields != types.fields || f.structures != types.structures {
            return Err(BackendError::new(
                "native eligibility",
                "functions disagree on resolved Structure fields",
            ));
        }
        lower_function(&mut out, f, &declared, &types)?;
    }
    writeln!(out, "define i32 @main() {{\nentry:\n  %entry_result = call i32 {}()\n  ret i32 %entry_result\n}}", symbol(main[0].id)).unwrap();
    Ok(out)
}

fn lower_function(
    out: &mut String,
    f: &m::Function,
    declared: &[Option<Signature>],
    types: &NativeTypes<'_>,
) -> Result<(), BackendError> {
    let arrays = ArrayShapes::analyze(f)?;
    let signature = declared[f.id.0].as_ref().expect("collected");
    writeln!(
        out,
        "define {} {}({}) {{",
        types.ty(&f.return_type)?,
        symbol(f.id),
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
                (types, &arrays),
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

fn lower_instruction(
    out: &mut String,
    f: &m::Function,
    ins: &m::Instruction,
    values: &mut [Option<String>],
    declared: &[Option<Signature>],
    layout: (&NativeTypes<'_>, &ArrayShapes),
    guard_counter: &mut usize,
) -> Result<(), BackendError> {
    let (types, arrays) = layout;
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
            };
            values[ins.result.expect("verified").0] = Some(text);
            return Ok(());
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
            writeln!(
                out,
                "  {} = load {}, ptr {}",
                result.as_ref().expect("verified"),
                types.ty(&place.ty)?,
                ptr
            )
            .unwrap();
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
                || !return_type
                    .as_ref()
                    .is_some_and(|ty| ty.same_type(&signature.result))
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
            writeln!(
                out,
                "  {} = call {} {}({})",
                result.as_ref().expect("verified"),
                types.ty(&signature.result)?,
                symbol(*id),
                args.join(", ")
            )
            .unwrap();
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
        m::InstructionKind::Drop(_) => {
            return Err(BackendError::new(
                "native eligibility",
                "native Drop contract is not implemented",
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
        assert!(native_type(&TypeName::Variant).is_err());
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
