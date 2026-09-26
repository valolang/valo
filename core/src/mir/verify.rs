//! Verifier for the first non-SSA MIR. It checks structure, identities and
//! types before a later dataflow pass reasons about dominance and lifetimes.
use super::ir::*;
use crate::frontend::semantics::arithmetic;
use crate::frontend::semantics::type_properties::KnownProperty;
use crate::frontend::semantics::typed_hir::LocalStorage;
use crate::frontend::type_model::TypeName;

pub fn verify(function: &Function) -> Result<(), String> {
    for (index, structure) in function.structures.iter().enumerate() {
        if !matches!(structure, TypeName::User(_))
            || function.structures[..index]
                .iter()
                .any(|prior| prior.same_type(structure))
        {
            return Err("MIR Structure identity is invalid or duplicated".into());
        }
    }
    if function.fields.iter().any(|field| {
        !function
            .structures
            .iter()
            .chain(&function.classes)
            .any(|owner| owner.same_type(&field.owner))
    }) {
        return Err("MIR field owner is not a declared value or reference type".into());
    }
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
            TerminatorKind::ReturnVoid => {
                if function.return_type != TypeName::Void {
                    return Err("MIR Void return in non-Void function".into());
                }
            }
            TerminatorKind::Trap(_) | TerminatorKind::Unreachable => {}
        }
    }
    if definitions.iter().any(|defined| !defined) {
        return Err("MIR temp has no definition".into());
    }
    verify_temp_availability(function)?;
    verify_managed_temp_uses(function)?;
    Ok(())
}

/// Managed temporaries own their retained fields. Every reachable definition
/// must be transferred or consumed once; ordinary scalar temps may be reused.
fn verify_managed_temp_uses(function: &Function) -> Result<(), String> {
    let cfg = super::analysis::cfg::Cfg::build(function)?;
    let mut counts = vec![0usize; function.temps.len()];
    let mut defined = vec![false; function.temps.len()];
    for block in &function.blocks {
        if !cfg.reachable[block.id.0] {
            continue;
        }
        for instruction in &block.instructions {
            if let Some(id) = instruction.result {
                defined[id.0] = true;
            }
            for id in instruction_uses(&instruction.kind) {
                counts[id.0] += 1;
            }
        }
        if let TerminatorKind::Return(id) = block.terminator.as_ref().expect("verified").kind {
            counts[id.0] += 1;
        }
    }
    for (index, ty) in function.temps.iter().enumerate() {
        if function.has_managed_fields(ty) && defined[index] && counts[index] != 1 {
            return Err(format!(
                "MIR owned managed temp %{index} must be consumed exactly once"
            ));
        }
    }
    Ok(())
}

fn verify_temp_availability(function: &Function) -> Result<(), String> {
    let cfg = super::analysis::cfg::Cfg::build(function)?;
    let states = super::analysis::dataflow::solve_forward(
        function,
        &cfg,
        vec![false; function.temps.len()],
        |a, b| a.iter().zip(b).map(|(x, y)| *x && *y).collect(),
        |block, input| {
            let mut available = input.clone();
            for instruction in &block.instructions {
                if let Some(id) = instruction.result {
                    available[id.0] = true;
                }
            }
            available
        },
    );
    for block in &function.blocks {
        let Some(mut available) = states.entry[block.id.0].clone() else {
            continue;
        };
        for instruction in &block.instructions {
            for id in instruction_uses(&instruction.kind) {
                if !available[id.0] {
                    return Err(format!(
                        "MIR bb{} uses temp %{} before definition on all paths",
                        block.id.0, id.0
                    ));
                }
            }
            if let Some(id) = instruction.result {
                available[id.0] = true;
            }
        }
        let terminator = &block.terminator.as_ref().expect("checked above").kind;
        let used = match terminator {
            TerminatorKind::Branch { condition, .. } | TerminatorKind::Return(condition) => {
                Some(*condition)
            }
            _ => None,
        };
        if let Some(id) = used
            && !available[id.0]
        {
            return Err(format!(
                "MIR bb{} uses temp %{} before definition on all paths",
                block.id.0, id.0
            ));
        }
    }
    Ok(())
}

fn place_uses(place: &Place) -> Vec<TempId> {
    place
        .projections
        .iter()
        .filter_map(|p| {
            if let Projection::Index(id) = p {
                Some(*id)
            } else {
                None
            }
        })
        .collect()
}

fn instruction_uses(kind: &InstructionKind) -> Vec<TempId> {
    match kind {
        InstructionKind::Const(_)
        | InstructionKind::ArrayInit { .. }
        | InstructionKind::NewClass(_)
        | InstructionKind::NewCollection
        | InstructionKind::EndBorrow(_)
        | InstructionKind::DropCandidate(_) => vec![],
        InstructionKind::StringLen(value) | InstructionKind::StringFormat { value, .. } => {
            vec![*value]
        }
        InstructionKind::BoxDynamic { value, .. }
        | InstructionKind::UnboxDynamic { value, .. }
        | InstructionKind::SnapshotCollection(value)
        | InstructionKind::CollectionCount(value) => vec![*value],
        InstructionKind::CollectionItem { collection, index }
        | InstructionKind::CollectionRemove { collection, index } => vec![*collection, *index],
        InstructionKind::CollectionAdd {
            collection,
            item,
            before,
        } => {
            let mut uses = vec![*collection, *item];
            uses.extend(before);
            uses
        }
        InstructionKind::StringConcat { left, right }
        | InstructionKind::StringCompare { left, right, .. }
        | InstructionKind::ReferenceIdentity { left, right, .. } => vec![*left, *right],
        InstructionKind::TupleInit(elements) => elements.clone(),
        InstructionKind::ArrayLen(place)
        | InstructionKind::SnapshotArray(place)
        | InstructionKind::Load(place)
        | InstructionKind::CloneString(place)
        | InstructionKind::CloneManaged(place)
        | InstructionKind::Move(place)
        | InstructionKind::Drop(place)
        | InstructionKind::BorrowStart { place, .. } => place_uses(place),
        InstructionKind::Store { place, value } | InstructionKind::Replace { place, value } => {
            let mut used = place_uses(place);
            used.push(*value);
            used
        }
        InstructionKind::Arithmetic { left, right, .. }
        | InstructionKind::Compare { left, right, .. } => vec![*left, *right],
        InstructionKind::Cast { value, .. } => vec![*value],
        InstructionKind::Call { arguments, .. } => arguments
            .iter()
            .flat_map(|a| match a {
                CallArgument::Value(id) => vec![*id],
                CallArgument::Place { place, .. } => place_uses(place),
            })
            .collect(),
    }
}

fn verify_instruction(function: &Function, instruction: &Instruction) -> Result<(), String> {
    let result = instruction
        .result
        .map(|id| temp_type(function, id))
        .transpose()?;
    match &instruction.kind {
        InstructionKind::DropCandidate(_) => {
            return Err("MIR Drop candidate was not elaborated".into());
        }
        InstructionKind::Const(value) => {
            let Some(result) = result else {
                return Err("MIR constant needs a result".into());
            };
            let valid = match value {
                Constant::ZeroAggregate => matches!(result, TypeName::User(_) | TypeName::Tuple(_)),
                Constant::Integer(_) => result.is_integral(),
                Constant::Single(_) => result.same_type(&TypeName::Single),
                Constant::Double(_) => result.same_type(&TypeName::Double),
                Constant::Boolean(_) => result.same_type(&TypeName::Boolean),
                Constant::String(_) => result.same_type(&TypeName::String),
                Constant::NullReference => {
                    *result == TypeName::Variant
                        || matches!(result, TypeName::User(name) if name.eq_ignore_ascii_case(crate::runtime::well_known::COLLECTION))
                        || function.classes.iter().any(|class| class.same_type(result))
                }
            };
            if !valid {
                return Err("MIR constant type is incorrect".into());
            }
        }
        InstructionKind::TupleInit(values) => {
            let Some(TypeName::Tuple(elements)) = result else {
                return Err("MIR tuple constructor needs a tuple result".into());
            };
            if values.len() != elements.len()
                || values.iter().zip(elements).any(|(value, element)| {
                    !temp_type(function, *value).is_ok_and(|ty| ty.same_type(&element.ty))
                })
            {
                return Err("MIR tuple constructor elements have incorrect types".into());
            }
        }
        InstructionKind::ArrayInit { upper } => {
            if *upper < 0 || !matches!(result, Some(TypeName::Array(_))) {
                return Err("MIR array initializer is invalid".into());
            }
        }
        InstructionKind::NewClass(ty) => {
            if !function.classes.iter().any(|class| class.same_type(ty))
                || !result.is_some_and(|result| result.same_type(ty))
            {
                return Err("MIR Class allocation has an unresolved type".into());
            }
        }
        InstructionKind::NewCollection => {
            if !result.is_some_and(is_collection) {
                return Err("MIR Collection allocation has wrong type".into());
            }
        }
        InstructionKind::SnapshotCollection(value) => {
            if !is_collection(temp_type(function, *value)?) || !result.is_some_and(is_collection) {
                return Err("MIR Collection snapshot types are invalid".into());
            }
        }
        InstructionKind::BoxDynamic { value, ty } => {
            if !temp_type(function, *value)?.same_type(ty)
                || !result.is_some_and(|ty| ty == &TypeName::Variant)
            {
                return Err("MIR dynamic box has wrong type".into());
            }
        }
        InstructionKind::UnboxDynamic { value, ty } => {
            if temp_type(function, *value)? != &TypeName::Variant
                || !result.is_some_and(|result| result.same_type(ty))
            {
                return Err("MIR dynamic unbox has wrong type".into());
            }
        }
        InstructionKind::CollectionCount(value) => {
            if !is_collection(temp_type(function, *value)?)
                || !result.is_some_and(|ty| *ty == TypeName::Int32)
            {
                return Err("MIR Collection.Count types are invalid".into());
            }
        }
        InstructionKind::CollectionItem { collection, index } => {
            if !is_collection(temp_type(function, *collection)?)
                || !temp_type(function, *index)?.is_integral()
                || !result.is_some_and(|ty| *ty == TypeName::Variant)
            {
                return Err("MIR Collection.Item types are invalid".into());
            }
        }
        InstructionKind::CollectionAdd {
            collection,
            item,
            before,
        } => {
            if !is_collection(temp_type(function, *collection)?)
                || *temp_type(function, *item)? != TypeName::Variant
                || before
                    .is_some_and(|id| !temp_type(function, id).is_ok_and(TypeName::is_integral))
                || result.is_some()
            {
                return Err("MIR Collection.Add types are invalid".into());
            }
        }
        InstructionKind::CollectionRemove { collection, index } => {
            if !is_collection(temp_type(function, *collection)?)
                || !temp_type(function, *index)?.is_integral()
                || result.is_some()
            {
                return Err("MIR Collection.Remove types are invalid".into());
            }
        }
        InstructionKind::ArrayLen(array) => {
            if !matches!(place_type(function, array)?, TypeName::Array(_))
                || !result.is_some_and(|ty| ty.same_type(&TypeName::Int64))
            {
                return Err("MIR array length types are incorrect".into());
            }
        }
        InstructionKind::SnapshotArray(array) => {
            let ty = place_type(function, array)?;
            if !matches!(ty, TypeName::Array(_))
                || !result.is_some_and(|result| result.same_type(&ty))
            {
                return Err("MIR array snapshot type is incorrect".into());
            }
        }
        InstructionKind::Load(place) => {
            let ty = place_type(function, place)?;
            if function.has_managed_fields(&ty) {
                return Err("MIR managed load must use semantic Clone".into());
            }
            if !result.is_some_and(|result| result.same_type(&ty)) {
                return Err("MIR load type is incorrect".into());
            }
        }
        InstructionKind::CloneString(place) => {
            if !place_type(function, place)?.same_type(&TypeName::String)
                || !result.is_some_and(|ty| ty.same_type(&TypeName::String))
            {
                return Err("MIR String clone requires String input and result".into());
            }
        }
        InstructionKind::CloneManaged(place) => {
            let ty = place_type(function, place)?;
            if matches!(ty, TypeName::String | TypeName::Array(_))
                || !function.has_managed_fields(&ty)
                || !result.is_some_and(|result| result.same_type(&ty))
            {
                return Err("MIR managed aggregate clone has incorrect types".into());
            }
        }
        InstructionKind::StringConcat { left, right }
        | InstructionKind::StringCompare { left, right, .. } => {
            let expected = if matches!(instruction.kind, InstructionKind::StringConcat { .. }) {
                TypeName::String
            } else {
                TypeName::Boolean
            };
            if !temp_type(function, *left)?.same_type(&TypeName::String)
                || !temp_type(function, *right)?.same_type(&TypeName::String)
                || !result.is_some_and(|ty| ty.same_type(&expected))
            {
                return Err("MIR String binary operation has incorrect types".into());
            }
        }
        InstructionKind::ReferenceIdentity { left, right, .. } => {
            let ty = temp_type(function, *left)?;
            if !ty.same_type(temp_type(function, *right)?)
                || !function.classes.iter().any(|class| class.same_type(ty))
                || !result.is_some_and(|ty| ty.same_type(&TypeName::Boolean))
            {
                return Err("MIR reference identity types are invalid".into());
            }
        }
        InstructionKind::StringLen(value) => {
            if !temp_type(function, *value)?.same_type(&TypeName::String)
                || !result.is_some_and(|ty| ty.same_type(&TypeName::Int32))
            {
                return Err("MIR String length has incorrect types".into());
            }
        }
        InstructionKind::StringFormat { value, decimals } => {
            let source = temp_type(function, *value)?;
            if !(source.is_integral()
                || matches!(
                    source,
                    TypeName::Single | TypeName::Double | TypeName::Boolean
                ))
                || !result.is_some_and(|ty| ty.same_type(&TypeName::String))
                || decimals.is_some_and(|digits| digits > 6)
            {
                return Err("MIR String format has incorrect types".into());
            }
        }
        InstructionKind::Move(place) => {
            let ty = place_type(function, place)?;
            let local = &function.locals[place.root.0];
            if !place.projections.is_empty()
                || local.storage != LocalStorage::Value
                || local.properties.copy == KnownProperty::Unknown
                || !result.is_some_and(|result| result.same_type(&ty))
            {
                return Err("MIR Move requires a classified whole owned local".into());
            }
        }
        InstructionKind::Drop(place) => {
            place_type(function, place)?;
            let local = &function.locals[place.root.0];
            if result.is_some()
                || !place.projections.is_empty()
                || local.storage != LocalStorage::Value
                || local.properties.requires_drop != KnownProperty::Yes
            {
                return Err("MIR Drop requires a droppable whole owned local".into());
            }
        }
        InstructionKind::BorrowStart { kind, place, .. } => {
            place_type(function, place)?;
            if result.is_some()
                || (*kind == BorrowKind::Mutable
                    && function.locals[place.root.0].storage == LocalStorage::BorrowedImmutable)
            {
                return Err("MIR mutable borrow targets a ReadOnly Place".into());
            }
        }
        InstructionKind::EndBorrow(_) => {
            if result.is_some() {
                return Err("MIR EndBorrow cannot produce a value".into());
            }
        }
        InstructionKind::Store { place, value } | InstructionKind::Replace { place, value } => {
            let ty = place_type(function, place)?;
            if result.is_some() || !temp_type(function, *value)?.same_type(&ty) {
                return Err("MIR store type is incorrect".into());
            }
            if matches!(instruction.kind, InstructionKind::Replace { .. })
                && !function.has_managed_fields(&ty)
            {
                return Err("MIR Replace requires a managed Place".into());
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

fn is_collection(ty: &TypeName) -> bool {
    matches!(ty, TypeName::User(name) if name.eq_ignore_ascii_case(crate::runtime::well_known::COLLECTION))
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
