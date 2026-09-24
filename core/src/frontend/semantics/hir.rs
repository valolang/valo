//! Project-wide semantic index and HIR foundation.
//!
//! This is not bytecode and does not replace the AST. It records stable IDs and
//! import-aware ownership so later passes can resolve bodies, expose LSP data,
//! lower to bytecode, and build packages without re-deriving identity from
//! strings at every layer.

use std::collections::HashMap;

use crate::Visibility;
use crate::frontend::modules::Project;
use crate::runtime::{Diagnostic, DiagnosticCode, Span};

use super::ids::{FunctionId, ModuleId, SymbolId, TypeId};
use super::symbols::key;

#[derive(Debug, Clone)]
pub struct ProjectIndex {
    pub modules: Vec<ModuleSymbol>,
    pub types: Vec<TypeSymbol>,
    pub functions: Vec<FunctionSymbol>,
    pub by_qualified_name: HashMap<String, SymbolId>,
}

#[derive(Debug, Clone)]
pub struct ModuleSymbol {
    pub id: ModuleId,
    pub name: String,
    pub namespace: Option<String>,
    pub path: std::path::PathBuf,
}

#[derive(Debug, Clone)]
pub struct TypeSymbol {
    pub id: TypeId,
    pub module: ModuleId,
    pub name: String,
    pub qualified_name: String,
    pub visibility: Visibility,
    pub kind: TypeSymbolKind,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeSymbolKind {
    Class,
    Interface,
    Structure,
    Type,
    Enum,
}

#[derive(Debug, Clone)]
pub struct FunctionSymbol {
    pub id: FunctionId,
    pub module: ModuleId,
    pub name: String,
    pub qualified_name: String,
    pub visibility: Visibility,
    pub kind: FunctionSymbolKind,
    pub signature: Option<Vec<(crate::PassingMode, crate::frontend::type_model::TypeName)>>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctionSymbolKind {
    Sub,
    Function,
    Property,
    Declare,
}

pub fn build_project_index(project: &Project) -> Result<ProjectIndex, Diagnostic> {
    let mut index = ProjectIndex {
        modules: Vec::new(),
        types: Vec::new(),
        functions: Vec::new(),
        by_qualified_name: HashMap::new(),
    };

    for (module_index, module) in project.modules.iter().enumerate() {
        let module_id = ModuleId(module_index);
        let module_name = module.name.clone();
        let namespace = module.program.namespace.clone();
        let module_symbol = ModuleSymbol {
            id: module_id,
            name: module_name.clone(),
            namespace: namespace.clone(),
            path: module.path.clone(),
        };
        insert_symbol(
            &mut index.by_qualified_name,
            &module_name,
            SymbolId::Module(module_id),
            None,
        )?;
        index.modules.push(module_symbol);

        let scope = SymbolScope {
            module: module_id,
            module_name: &module_name,
            namespace: namespace.as_deref(),
            owners: &module.program.owners,
        };

        fn index_nested(
            members: &[crate::ClassMember],
            prefix: &str,
            index: &mut ProjectIndex,
            scope: SymbolScope<'_>,
        ) -> Result<(), Diagnostic> {
            for member in members {
                match member {
                    crate::ClassMember::Type(t) => {
                        let kind = if t.kind == crate::TypeKind::Structure {
                            TypeSymbolKind::Structure
                        } else {
                            TypeSymbolKind::Type
                        };
                        push_type(
                            index,
                            scope,
                            &format!("{}.{}", prefix, t.name),
                            t.visibility,
                            kind,
                            t.span,
                        )?;
                    }
                    crate::ClassMember::Enum(e) => {
                        push_type(
                            index,
                            scope,
                            &format!("{}.{}", prefix, e.name),
                            e.visibility,
                            TypeSymbolKind::Enum,
                            e.span,
                        )?;
                    }
                    crate::ClassMember::Class(c) => {
                        let qualified = format!("{}.{}", prefix, c.name);
                        push_type(
                            index,
                            scope,
                            &qualified,
                            c.visibility,
                            TypeSymbolKind::Class,
                            c.span,
                        )?;
                        index_nested(&c.members, &qualified, index, scope)?;
                    }
                    _ => {}
                }
            }
            Ok(())
        }

        for type_decl in &module.program.types {
            let kind = if type_decl.kind == crate::TypeKind::Structure {
                TypeSymbolKind::Structure
            } else {
                TypeSymbolKind::Type
            };
            push_type(
                &mut index,
                scope,
                &type_decl.name,
                type_decl.visibility,
                kind,
                type_decl.span,
            )?;
        }
        for enum_decl in &module.program.enums {
            push_type(
                &mut index,
                scope,
                &enum_decl.name,
                enum_decl.visibility,
                TypeSymbolKind::Enum,
                enum_decl.span,
            )?;
        }
        for interface_decl in &module.program.interfaces {
            push_type(
                &mut index,
                scope,
                &interface_decl.name,
                interface_decl.visibility,
                TypeSymbolKind::Interface,
                interface_decl.span,
            )?;
        }
        for class_decl in &module.program.classes {
            push_type(
                &mut index,
                scope,
                &class_decl.name,
                class_decl.visibility,
                TypeSymbolKind::Class,
                class_decl.span,
            )?;
            index_nested(&class_decl.members, &class_decl.name, &mut index, scope)?;
        }
        for procedure in &module.program.procedures {
            push_function(
                &mut index,
                scope,
                &procedure.name,
                procedure.visibility,
                FunctionSymbolKind::Sub,
                procedure.span,
                Some(
                    procedure
                        .params
                        .iter()
                        .map(|param| (param.mode, param.ty.clone()))
                        .collect(),
                ),
            )?;
        }
        for function in &module.program.functions {
            push_function(
                &mut index,
                scope,
                &function.name,
                function.visibility,
                FunctionSymbolKind::Function,
                function.span,
                Some(
                    function
                        .params
                        .iter()
                        .map(|param| (param.mode, param.ty.clone()))
                        .collect(),
                ),
            )?;
        }
        for property in &module.program.properties {
            push_function(
                &mut index,
                scope,
                &property.name,
                property.visibility,
                FunctionSymbolKind::Property,
                property.span,
                None,
            )?;
        }
        for declare in &module.program.declares {
            push_function(
                &mut index,
                scope,
                &declare.name,
                declare.visibility,
                FunctionSymbolKind::Declare,
                declare.span,
                None,
            )?;
        }
    }

    Ok(index)
}

#[derive(Clone, Copy)]
struct SymbolScope<'a> {
    module: ModuleId,
    module_name: &'a str,
    namespace: Option<&'a str>,
    owners: &'a std::collections::HashMap<Span, String>,
}

fn push_type(
    index: &mut ProjectIndex,
    scope: SymbolScope<'_>,
    name: &str,
    visibility: Visibility,
    kind: TypeSymbolKind,
    span: Span,
) -> Result<(), Diagnostic> {
    let id = TypeId(index.types.len());
    let qualified_name = qualified_name(
        scope.module_name,
        scope.namespace,
        scope.owners.get(&span).map(String::as_str),
        name,
    );
    insert_symbol(
        &mut index.by_qualified_name,
        &qualified_name,
        SymbolId::Type(id),
        Some(span),
    )?;
    index.types.push(TypeSymbol {
        id,
        module: scope.module,
        name: name.to_string(),
        qualified_name,
        visibility,
        kind,
        span,
    });
    Ok(())
}

fn push_function(
    index: &mut ProjectIndex,
    scope: SymbolScope<'_>,
    name: &str,
    visibility: Visibility,
    kind: FunctionSymbolKind,
    span: Span,
    signature: Option<Vec<(crate::PassingMode, crate::frontend::type_model::TypeName)>>,
) -> Result<(), Diagnostic> {
    let id = FunctionId(index.functions.len());
    let qualified_name = qualified_name(
        scope.module_name,
        scope.namespace,
        scope.owners.get(&span).map(String::as_str),
        name,
    );
    if let Some(existing) = index.functions.iter().find(|existing| {
        key(&existing.qualified_name) == key(&qualified_name)
            && existing.kind == kind
            && existing.signature.is_some()
            && existing.signature == signature
    }) {
        return Err(Diagnostic::new(
            DiagnosticCode::DUPLICATE_DECLARATION,
            format!("Callable '{qualified_name}' with this signature is already declared"),
            Some(span),
        )
        .with_note(format!("first declaration is at {:?}", existing.span)));
    }
    let symbol_key = key(&qualified_name);
    if let Some(existing) = index.by_qualified_name.get(&symbol_key).copied() {
        // Procedures are allowed to share a name: a `Get` and a `Set` are one
        // property, and two `Sub`s with different parameters are overloads.
        // Whether they can actually be told apart is a question about their
        // parameters, which this index does not carry. `collect_signatures`
        // holds those and rejects the pairs no call could choose between. What
        // is rejected here is a procedure colliding with something that is not
        // one, such as a class of the same name.
        if !matches!(existing, SymbolId::Function(_)) {
            return Err(Diagnostic::new(
                DiagnosticCode::DUPLICATE_DECLARATION,
                format!("Symbol '{qualified_name}' is already declared in this project"),
                Some(span),
            ));
        }
    } else {
        index
            .by_qualified_name
            .insert(symbol_key, SymbolId::Function(id));
    }
    index.functions.push(FunctionSymbol {
        id,
        module: scope.module,
        name: name.to_string(),
        qualified_name,
        visibility,
        kind,
        signature,
        span,
    });
    Ok(())
}

fn qualified_name(
    module_name: &str,
    namespace: Option<&str>,
    owner: Option<&str>,
    name: &str,
) -> String {
    if let Some(owner) = owner.filter(|owner| !owner.is_empty()) {
        if name
            .to_ascii_lowercase()
            .starts_with(&format!("{}.", owner.to_ascii_lowercase()))
        {
            return name.to_string();
        }
        return format!("{owner}.{name}");
    }
    match namespace {
        Some(namespace) => {
            if name.starts_with(namespace) {
                name.to_string()
            } else {
                format!("{namespace}.{name}")
            }
        }
        None => format!("{module_name}.{name}"),
    }
}

fn insert_symbol(
    by_qualified_name: &mut HashMap<String, SymbolId>,
    qualified_name: &str,
    symbol: SymbolId,
    span: Option<Span>,
) -> Result<(), Diagnostic> {
    let key = key(qualified_name);
    if by_qualified_name.insert(key, symbol).is_some() {
        return Err(Diagnostic::new(
            DiagnosticCode::DUPLICATE_DECLARATION,
            format!("Symbol '{qualified_name}' is already declared in this project"),
            span,
        ));
    }
    Ok(())
}
