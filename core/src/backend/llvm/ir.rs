use std::fmt::Write;

use crate::frontend::semantics::arithmetic::ArithmeticOp;
use crate::frontend::semantics::type_properties::KnownProperty;
use crate::frontend::semantics::typed_hir::{ArgumentMode, ComparisonOp, Conversion, LocalStorage};
use crate::frontend::type_model::TypeName;
use crate::mir::{analysis::ownership, ir as m, verify};

use super::BackendError;

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

fn parameter_list(signature: &Signature, names: bool) -> Result<String, BackendError> {
    signature
        .parameters
        .iter()
        .enumerate()
        .map(|(i, (ty, storage))| {
            let llvm = if *storage == LocalStorage::Value {
                native_type(ty)?
            } else {
                "ptr"
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
fn signatures(module: &m::Module) -> Result<Vec<Option<Signature>>, BackendError> {
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
        native_type(&f.return_type)?;
        for local in &f.locals {
            native_type(&local.ty)?;
        }
        for ty in &f.temps {
            native_type(ty)?;
        }
        let mut parameters = f
            .locals
            .iter()
            .filter_map(|l| l.parameter_index.map(|i| (i, l.ty.clone(), l.storage)))
            .collect::<Vec<_>>();
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
    let declared = signatures(module)?;
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
    for f in &module.functions {
        lower_function(&mut out, f, &declared)?;
    }
    writeln!(out, "define i32 @main() {{\nentry:\n  %entry_result = call i32 {}()\n  ret i32 %entry_result\n}}", symbol(main[0].id)).unwrap();
    Ok(out)
}

fn lower_function(
    out: &mut String,
    f: &m::Function,
    declared: &[Option<Signature>],
) -> Result<(), BackendError> {
    let signature = declared[f.id.0].as_ref().expect("collected");
    writeln!(
        out,
        "define {} {}({}) {{",
        native_type(&f.return_type)?,
        symbol(f.id),
        parameter_list(signature, true)?
    )
    .unwrap();
    let mut values = vec![None; f.temps.len()];
    let mut guard_counter = 0usize;
    for block in &f.blocks {
        writeln!(out, "bb{}:", block.id.0).unwrap();
        if block.id == f.entry {
            for local in &f.locals {
                if local.storage == LocalStorage::Value {
                    writeln!(
                        out,
                        "  %l{} = alloca {}",
                        local.id.0,
                        native_type(&local.ty)?
                    )
                    .unwrap();
                    if let Some(index) = local.parameter_index {
                        writeln!(
                            out,
                            "  store {} %p{}, ptr %l{}",
                            native_type(&local.ty)?,
                            index,
                            local.id.0
                        )
                        .unwrap();
                    }
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
                native_type(&f.return_type)?,
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

fn place_ptr(f: &m::Function, place: &m::Place) -> Result<String, BackendError> {
    if !place.projections.is_empty() {
        return Err(BackendError::new(
            "native eligibility",
            "projected Places need native aggregate layout",
        ));
    }
    let local = &f.locals[place.root.0];
    Ok(if local.storage == LocalStorage::Value {
        format!("%l{}", place.root.0)
    } else {
        format!("%p{}", local.parameter_index.expect("borrowed parameter"))
    })
}

fn lower_instruction(
    out: &mut String,
    f: &m::Function,
    ins: &m::Instruction,
    values: &mut [Option<String>],
    declared: &[Option<Signature>],
    guard_counter: &mut usize,
) -> Result<(), BackendError> {
    let result = ins.result.map(|id| format!("%t{}", id.0));
    let result_ty = ins.result.map(|id| &f.temps[id.0]);
    match &ins.kind {
        m::InstructionKind::Const(constant) => {
            let text = match constant {
                m::Constant::Integer(n) => n.to_string(),
                m::Constant::Boolean(b) => b.to_string(),
                m::Constant::Single(n) => float_literal(f64::from(*n)),
                m::Constant::Double(n) => float_literal(*n),
            };
            values[ins.result.expect("verified").0] = Some(text);
            return Ok(());
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
            writeln!(
                out,
                "  {} = load {}, ptr {}",
                result.as_ref().expect("verified"),
                native_type(&place.ty)?,
                place_ptr(f, place)?
            )
            .unwrap();
        }
        m::InstructionKind::Store {
            place,
            value: source,
        } => writeln!(
            out,
            "  store {} {}, ptr {}",
            native_type(&place.ty)?,
            value(values, *source)?,
            place_ptr(f, place)?
        )
        .unwrap(),
        m::InstructionKind::Arithmetic { op, left, right } => {
            let ty = result_ty.expect("verified");
            let llvm_ty = native_type(ty)?;
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
                emit_division_guard(out, llvm_ty, ty, &a, &b, guard_counter);
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
                native_type(ty)?,
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
                    native_type(from)?,
                    source_value,
                    native_type(to)?
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
                            native_type(expected_type)?,
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
                    ) => args.push(format!("ptr {}", place_ptr(f, place)?)),
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
                native_type(&signature.result)?,
                symbol(*id),
                args.join(", ")
            )
            .unwrap();
        }
        m::InstructionKind::ArrayInit { .. }
        | m::InstructionKind::ArrayLen(_)
        | m::InstructionKind::SnapshotArray(_) => {
            return Err(BackendError::new(
                "native eligibility",
                "array representation is not native yet",
            ));
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
