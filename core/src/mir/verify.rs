//! Verifier for the first non-SSA MIR. It checks structure, identities and
//! types before a later dataflow pass reasons about dominance and lifetimes.
use super::ir::*;
use crate::frontend::semantics::arithmetic;
use crate::frontend::type_model::TypeName;

pub fn verify(function: &Function) -> Result<(), String> {
    if function.blocks.get(function.entry.0).is_none() {
        return Err("MIR entry block is missing".into());
    }
    for (index, local) in function.locals.iter().enumerate() {
        if local.id != LocalId(index) {
            return Err("MIR local ID is invalid".into());
        }
    }
    for (index, field) in function.fields.iter().enumerate() {
        if field.id.0 != index {
            return Err("MIR field ID is invalid".into());
        }
    }
    let mut definitions = vec![false; function.temps.len()];
    for (index, block) in function.blocks.iter().enumerate() {
        if block.id != BlockId(index) {
            return Err("MIR block ID is invalid".into());
        }
        if block.terminator.is_none() {
            return Err(format!("MIR bb{index} has no terminator"));
        }
        for instruction in &block.instructions {
            if let Some(result) = instruction.result {
                let defined = definitions
                    .get_mut(result.0)
                    .ok_or("MIR result temp is invalid")?;
                if *defined {
                    return Err("MIR temp has multiple definitions".into());
                }
                *defined = true;
            }
            verify_instruction(function, instruction)?;
        }
        match &block.terminator.as_ref().expect("checked above").kind {
            TerminatorKind::Goto(target) => block_id(function, *target)?,
            TerminatorKind::Branch {
                condition,
                then_block,
                else_block,
            } => {
                if !temp_type(function, *condition)?.same_type(&TypeName::Boolean) {
                    return Err("MIR branch condition is not Boolean".into());
                }
                block_id(function, *then_block)?;
                block_id(function, *else_block)?;
            }
            TerminatorKind::Return(value) => {
                if !temp_type(function, *value)?.same_type(&function.return_type) {
                    return Err("MIR return type is incorrect".into());
                }
            }
            TerminatorKind::Trap(_) | TerminatorKind::Unreachable => {}
        }
    }
    if definitions.iter().any(|defined| !defined) {
        return Err("MIR temp has no definition".into());
    }
    Ok(())
}

fn verify_instruction(function: &Function, instruction: &Instruction) -> Result<(), String> {
    let result = instruction
        .result
        .map(|id| temp_type(function, id))
        .transpose()?;
    match &instruction.kind {
        InstructionKind::Const(value) => {
            let Some(result) = result else {
                return Err("MIR constant needs a result".into());
            };
            let valid = match value {
                Constant::Integer(_) => result.is_integral(),
                Constant::Single(_) => result.same_type(&TypeName::Single),
                Constant::Double(_) => result.same_type(&TypeName::Double),
                Constant::Boolean(_) => result.same_type(&TypeName::Boolean),
            };
            if !valid {
                return Err("MIR constant type is incorrect".into());
            }
        }
        InstructionKind::ArrayInit { upper } => {
            if *upper < 0 || !matches!(result, Some(TypeName::Array(_))) {
                return Err("MIR array initializer is invalid".into());
            }
        }
        InstructionKind::ArrayLen(array) => {
            if !matches!(place_type(function, array)?, TypeName::Array(_))
                || !result.is_some_and(|ty| ty.same_type(&TypeName::Int64))
            {
                return Err("MIR array length types are incorrect".into());
            }
        }
        InstructionKind::Load(place) => {
            let ty = place_type(function, place)?;
            if !result.is_some_and(|result| result.same_type(&ty)) {
                return Err("MIR load type is incorrect".into());
            }
        }
        InstructionKind::Store { place, value } => {
            let ty = place_type(function, place)?;
            if result.is_some() || !temp_type(function, *value)?.same_type(&ty) {
                return Err("MIR store type is incorrect".into());
            }
        }
        InstructionKind::Arithmetic { op, left, right } => {
            let left = temp_type(function, *left)?;
            let right = temp_type(function, *right)?;
            let Some(signature) = arithmetic::signature(*op, left, right) else {
                return Err("MIR arithmetic operand types are invalid".into());
            };
            if !left.same_type(right)
                || !result.is_some_and(|ty| ty.same_type(&signature.result_type))
            {
                return Err("MIR arithmetic result type is incorrect".into());
            }
        }
        InstructionKind::Compare { left, right, .. } => {
            let left = temp_type(function, *left)?;
            let right = temp_type(function, *right)?;
            if !left.same_type(right) || !result.is_some_and(|ty| ty.same_type(&TypeName::Boolean))
            {
                return Err("MIR comparison types are incorrect".into());
            }
        }
        InstructionKind::Cast { value, .. } => {
            let source = temp_type(function, *value)?;
            let Some(target) = result else {
                return Err("MIR cast needs a result".into());
            };
            let numeric = |ty: &TypeName| {
                ty.is_integral() || matches!(ty, TypeName::Single | TypeName::Double)
            };
            if !numeric(source) || !numeric(target) {
                return Err("MIR cast types are unsupported".into());
            }
        }
        InstructionKind::Call {
            target,
            arguments,
            parameter_types,
            parameter_modes,
            return_type,
        } => {
            if arguments.len() != parameter_types.len() || arguments.len() != parameter_modes.len()
            {
                return Err("MIR call arity is incorrect".into());
            }
            for ((argument, expected), mode) in
                arguments.iter().zip(parameter_types).zip(parameter_modes)
            {
                let correct_mode = match argument {
                    CallArgument::Value(_) => {
                        *mode == crate::frontend::semantics::typed_hir::ArgumentMode::ByVal
                    }
                    CallArgument::Place { mode: actual, .. } => actual == mode,
                };
                if !correct_mode {
                    return Err("MIR call argument mode is incorrect".into());
                }
                let actual = match argument {
                    CallArgument::Value(value) => temp_type(function, *value)?.clone(),
                    CallArgument::Place { place, .. } => place_type(function, place)?,
                };
                if !actual.same_type(expected) {
                    return Err("MIR call argument type is incorrect".into());
                }
            }
            if result.cloned() != *return_type {
                return Err("MIR call return type is incorrect".into());
            }
            if let CallTarget::Dispose(method) = target {
                let owner = function
                    .disposers
                    .get(method.0)
                    .ok_or("MIR Dispose method is invalid")?;
                if arguments.len() != 1
                    || !parameter_types[0].same_type(owner)
                    || !matches!(arguments[0], CallArgument::Place { .. })
                    || return_type.is_some()
                {
                    return Err("MIR Dispose call signature is invalid".into());
                }
            }
        }
    }
    Ok(())
}

fn block_id(function: &Function, id: BlockId) -> Result<(), String> {
    if function.blocks.get(id.0).is_none() {
        Err("MIR branch target is invalid".into())
    } else {
        Ok(())
    }
}

fn temp_type(function: &Function, id: TempId) -> Result<&TypeName, String> {
    function
        .temps
        .get(id.0)
        .ok_or_else(|| "MIR temp ID is invalid".into())
}

fn place_type(function: &Function, place: &Place) -> Result<TypeName, String> {
    let mut ty = function
        .locals
        .get(place.root.0)
        .ok_or("MIR Place root is invalid")?
        .ty
        .clone();
    for projection in &place.projections {
        ty = match projection {
            Projection::Field(id) => {
                let field = function.fields.get(id.0).ok_or("MIR field ID is invalid")?;
                if field.id != *id || !field.owner.same_type(&ty) {
                    return Err("MIR field owner is incorrect".into());
                }
                field.ty.clone()
            }
            Projection::TupleField(index) => {
                let TypeName::Tuple(elements) = ty else {
                    return Err("MIR tuple projection has wrong base".into());
                };
                elements
                    .get(*index)
                    .ok_or("MIR tuple field is out of bounds")?
                    .ty
                    .clone()
            }
            Projection::Index(index) => {
                if !temp_type(function, *index)?.is_integral() {
                    return Err("MIR index is not integral".into());
                }
                let TypeName::Array(element) = ty else {
                    return Err("MIR index projection has wrong base".into());
                };
                *element
            }
        };
    }
    if !ty.same_type(&place.ty) {
        return Err("MIR Place final type is incorrect".into());
    }
    Ok(ty)
}
