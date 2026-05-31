use crate::runtime::{Span, TypeName};

use super::Expr;

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Dim {
        name: String,
        ty: Option<TypeName>,
        array: Option<ArrayDecl>,
        as_new: bool,
        new_args: Vec<Expr>,
        initializer: Option<Expr>,
        collection_initializer: Option<Vec<Expr>>,
        span: Span,
    },
    DimMany {
        decls: Vec<VariableDecl>,
        span: Span,
    },
    Static {
        name: String,
        ty: Option<TypeName>,
        array: Option<ArrayDecl>,
        as_new: bool,
        new_args: Vec<Expr>,
        initializer: Option<Expr>,
        collection_initializer: Option<Vec<Expr>>,
        span: Span,
    },
    StaticMany {
        decls: Vec<VariableDecl>,
        span: Span,
    },
    Const {
        name: String,
        ty: Option<TypeName>,
        value: Expr,
        span: Span,
    },
    ConstMany {
        consts: Vec<crate::ConstDecl>,
        span: Span,
    },
    Assign {
        target: AssignTarget,
        expr: Expr,
        span: Span,
    },
    SetAssign {
        target: AssignTarget,
        expr: Expr,
        span: Span,
    },
    ConsoleCall {
        method: String,
        args: Vec<Expr>,
        span: Span,
    },
    End {
        span: Span,
    },
    SubCall {
        name: String,
        args: Vec<Expr>,
        span: Span,
    },
    MemberSubCall {
        object: Expr,
        method: String,
        args: Vec<Expr>,
        span: Span,
    },
    RaiseEvent {
        name: String,
        args: Vec<Expr>,
        span: Span,
    },
    AddHandler {
        event: Expr,
        handler: Expr,
        span: Span,
    },
    RemoveHandler {
        event: Expr,
        handler: Expr,
        span: Span,
    },
    Await {
        expr: Expr,
        span: Span,
    },
    Return {
        expr: Expr,
        span: Span,
    },
    If {
        condition: Expr,
        then_body: Vec<Stmt>,
        elseif_branches: Vec<ElseIfBranch>,
        else_body: Vec<Stmt>,
        span: Span,
    },
    SelectCase {
        subject: Expr,
        branches: Vec<CaseBranch>,
        else_body: Vec<Stmt>,
        span: Span,
    },
    While {
        condition: Expr,
        body: Vec<Stmt>,
        span: Span,
    },
    DoLoop {
        condition: DoLoopCondition,
        body: Vec<Stmt>,
        span: Span,
    },
    For {
        variable: String,
        start: Expr,
        end: Expr,
        step: Option<Expr>,
        next_variable: Option<(String, Span)>,
        body: Vec<Stmt>,
        span: Span,
    },
    ForEach {
        variable: String,
        iterable: Expr,
        next_variable: Option<(String, Span)>,
        body: Vec<Stmt>,
        span: Span,
    },
    ReDim {
        target: ReDimTarget,
        dims: Vec<(Option<Expr>, Expr)>,
        preserve: bool,
        span: Span,
    },
    Erase {
        target: ReDimTarget,
        span: Span,
    },
    LSet {
        target: AssignTarget,
        expr: Expr,
        span: Span,
    },
    RSet {
        target: AssignTarget,
        expr: Expr,
        span: Span,
    },
    Label {
        name: String,
        span: Span,
    },
    GoTo {
        label: String,
        span: Span,
    },
    OnError {
        mode: OnErrorMode,
        span: Span,
    },
    Resume {
        target: ResumeTarget,
        span: Span,
    },
    With {
        target: Expr,
        body: Vec<Stmt>,
        span: Span,
    },
    Using {
        resource: UsingResource,
        body: Vec<Stmt>,
        span: Span,
    },
    Exit {
        target: ExitTarget,
        span: Span,
    },
    TryCatch {
        try_body: Vec<Stmt>,
        catch_block: Option<CatchBlock>,
        finally_body: Option<Vec<Stmt>>,
        span: Span,
    },
    DebugPrint {
        args: Vec<Expr>,
        span: Span,
    },
    OpenFile {
        path: Expr,
        mode: OpenMode,
        access: Option<FileAccess>,
        lock: Option<FileLock>,
        shared: bool,
        number: Expr,
        record_len: Option<Expr>,
        span: Span,
    },
    CloseFile {
        numbers: Vec<Expr>,
        span: Span,
    },
    LineInput {
        number: Expr,
        target: AssignTarget,
        span: Span,
    },
    InputFile {
        number: Expr,
        targets: Vec<AssignTarget>,
        span: Span,
    },
    PrintFile {
        number: Expr,
        items: Vec<PrintItem>,
        trailing: Option<PrintSeparator>,
        span: Span,
    },
    WriteFile {
        number: Expr,
        args: Vec<Expr>,
        span: Span,
    },
    GetFile {
        number: Expr,
        position: Option<Expr>,
        target: AssignTarget,
        span: Span,
    },
    PutFile {
        number: Expr,
        position: Option<Expr>,
        expr: Expr,
        span: Span,
    },
    SeekFile {
        number: Expr,
        position: Expr,
        span: Span,
    },
    NameFile {
        old_path: Expr,
        new_path: Expr,
        span: Span,
    },
    Yield {
        expr: Expr,
        span: Span,
    },
    Throw {
        expr: Expr,
        span: Span,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenMode {
    Input,
    Output,
    Append,
    Binary,
    Random,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileAccess {
    Read,
    Write,
    ReadWrite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileLock {
    Read,
    Write,
    ReadWrite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrintSeparator {
    None,
    Comma,
    Semicolon,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PrintItem {
    pub separator: PrintSeparator,
    pub expr: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CatchBlock {
    pub variable: Option<String>,
    pub body: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OnErrorMode {
    ResumeNext,
    GoToZero,
    GoToMinusOne,
    GoToLabel(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResumeTarget {
    Retry,
    Next,
    Label(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ArrayDecl {
    Fixed(Vec<crate::runtime::ArrayBound>),
    Dynamic,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AssignTarget {
    Variable {
        name: String,
        span: Span,
    },
    ArrayElement {
        name: String,
        indices: Vec<Expr>,
        span: Span,
    },
    Member {
        object: Expr,
        field: String,
        span: Span,
    },
    MemberArrayElement {
        object: Expr,
        field: String,
        indices: Vec<Expr>,
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct VariableDecl {
    pub name: String,
    pub ty: Option<TypeName>,
    pub array: Option<ArrayDecl>,
    pub as_new: bool,
    pub new_args: Vec<Expr>,
    pub initializer: Option<Expr>,
    pub collection_initializer: Option<Vec<Expr>>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UsingResource {
    Declaration(VariableDecl),
    Target(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReDimTarget {
    Variable {
        name: String,
        span: Span,
    },
    Member {
        object: Expr,
        field: String,
        span: Span,
    },
}

impl ReDimTarget {
    pub fn name(&self) -> &str {
        match self {
            ReDimTarget::Variable { name, .. } => name,
            ReDimTarget::Member { field, .. } => field,
        }
    }
}

impl AssignTarget {
    pub fn span(&self) -> Span {
        match self {
            AssignTarget::Variable { span, .. }
            | AssignTarget::ArrayElement { span, .. }
            | AssignTarget::Member { span, .. }
            | AssignTarget::MemberArrayElement { span, .. } => *span,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ElseIfBranch {
    pub condition: Expr,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CaseBranch {
    pub items: Vec<CaseItem>,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CaseItem {
    Value(Expr),
    Range { start: Expr, end: Expr },
    Compare { op: CaseCompareOp, expr: Expr },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseCompareOp {
    Equal,
    NotEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DoLoopCondition {
    Infinite,
    PreWhile(Expr),
    PreUntil(Expr),
    PostWhile(Expr),
    PostUntil(Expr),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitTarget {
    Sub,
    Function,
    Property,
    For,
    While,
    Do,
}

impl Stmt {
    pub fn substitute_generics(&self, bindings: &[(String, TypeName)]) -> Self {
        match self {
            Stmt::Dim {
                name,
                ty,
                array,
                as_new,
                new_args,
                initializer,
                collection_initializer,
                span,
            } => Stmt::Dim {
                name: name.clone(),
                ty: ty.as_ref().map(|ty| ty.substitute_generics(bindings)),
                array: array.clone(),
                as_new: *as_new,
                new_args: new_args
                    .iter()
                    .map(|arg| arg.substitute_generics(bindings))
                    .collect(),
                initializer: initializer
                    .as_ref()
                    .map(|init| init.substitute_generics(bindings)),
                collection_initializer: collection_initializer.as_ref().map(|init| {
                    init.iter()
                        .map(|arg| arg.substitute_generics(bindings))
                        .collect()
                }),
                span: *span,
            },
            Stmt::DimMany { decls, span } => Stmt::DimMany {
                decls: decls
                    .iter()
                    .map(|decl| decl.substitute_generics(bindings))
                    .collect(),
                span: *span,
            },
            Stmt::Static {
                name,
                ty,
                array,
                as_new,
                new_args,
                initializer,
                collection_initializer,
                span,
            } => Stmt::Static {
                name: name.clone(),
                ty: ty.as_ref().map(|ty| ty.substitute_generics(bindings)),
                array: array.clone(),
                as_new: *as_new,
                new_args: new_args
                    .iter()
                    .map(|arg| arg.substitute_generics(bindings))
                    .collect(),
                initializer: initializer
                    .as_ref()
                    .map(|init| init.substitute_generics(bindings)),
                collection_initializer: collection_initializer.as_ref().map(|init| {
                    init.iter()
                        .map(|arg| arg.substitute_generics(bindings))
                        .collect()
                }),
                span: *span,
            },
            Stmt::StaticMany { decls, span } => Stmt::StaticMany {
                decls: decls
                    .iter()
                    .map(|decl| decl.substitute_generics(bindings))
                    .collect(),
                span: *span,
            },
            Stmt::Const {
                name,
                ty,
                value,
                span,
            } => Stmt::Const {
                name: name.clone(),
                ty: ty.as_ref().map(|ty| ty.substitute_generics(bindings)),
                value: value.substitute_generics(bindings),
                span: *span,
            },
            Stmt::ConstMany { consts, span } => Stmt::ConstMany {
                consts: consts
                    .iter()
                    .map(|c| crate::ConstDecl {
                        visibility: c.visibility,
                        name: c.name.clone(),
                        ty: c.ty.as_ref().map(|ty| ty.substitute_generics(bindings)),
                        value: c.value.substitute_generics(bindings),
                        span: c.span,
                    })
                    .collect(),
                span: *span,
            },
            Stmt::Assign { target, expr, span } => Stmt::Assign {
                target: target.substitute_generics(bindings),
                expr: expr.substitute_generics(bindings),
                span: *span,
            },
            Stmt::SetAssign { target, expr, span } => Stmt::SetAssign {
                target: target.substitute_generics(bindings),
                expr: expr.substitute_generics(bindings),
                span: *span,
            },
            Stmt::ConsoleCall { method, args, span } => Stmt::ConsoleCall {
                method: method.clone(),
                args: args
                    .iter()
                    .map(|arg| arg.substitute_generics(bindings))
                    .collect(),
                span: *span,
            },
            Stmt::SubCall { name, args, span } => Stmt::SubCall {
                name: name.clone(),
                args: args
                    .iter()
                    .map(|arg| arg.substitute_generics(bindings))
                    .collect(),
                span: *span,
            },
            Stmt::MemberSubCall {
                object,
                method,
                args,
                span,
            } => Stmt::MemberSubCall {
                object: object.substitute_generics(bindings),
                method: method.clone(),
                args: args
                    .iter()
                    .map(|arg| arg.substitute_generics(bindings))
                    .collect(),
                span: *span,
            },
            Stmt::RaiseEvent { name, args, span } => Stmt::RaiseEvent {
                name: name.clone(),
                args: args
                    .iter()
                    .map(|arg| arg.substitute_generics(bindings))
                    .collect(),
                span: *span,
            },
            Stmt::AddHandler {
                event,
                handler,
                span,
            } => Stmt::AddHandler {
                event: event.substitute_generics(bindings),
                handler: handler.substitute_generics(bindings),
                span: *span,
            },
            Stmt::RemoveHandler {
                event,
                handler,
                span,
            } => Stmt::RemoveHandler {
                event: event.substitute_generics(bindings),
                handler: handler.substitute_generics(bindings),
                span: *span,
            },
            Stmt::Await { expr, span } => Stmt::Await {
                expr: expr.substitute_generics(bindings),
                span: *span,
            },
            Stmt::Return { expr, span } => Stmt::Return {
                expr: expr.substitute_generics(bindings),
                span: *span,
            },
            Stmt::If {
                condition,
                then_body,
                elseif_branches,
                else_body,
                span,
            } => Stmt::If {
                condition: condition.substitute_generics(bindings),
                then_body: then_body
                    .iter()
                    .map(|s| s.substitute_generics(bindings))
                    .collect(),
                elseif_branches: elseif_branches
                    .iter()
                    .map(|b| b.substitute_generics(bindings))
                    .collect(),
                else_body: else_body
                    .iter()
                    .map(|s| s.substitute_generics(bindings))
                    .collect(),
                span: *span,
            },
            Stmt::SelectCase {
                subject,
                branches,
                else_body,
                span,
            } => Stmt::SelectCase {
                subject: subject.substitute_generics(bindings),
                branches: branches
                    .iter()
                    .map(|b| b.substitute_generics(bindings))
                    .collect(),
                else_body: else_body
                    .iter()
                    .map(|s| s.substitute_generics(bindings))
                    .collect(),
                span: *span,
            },
            Stmt::While {
                condition,
                body,
                span,
            } => Stmt::While {
                condition: condition.substitute_generics(bindings),
                body: body
                    .iter()
                    .map(|s| s.substitute_generics(bindings))
                    .collect(),
                span: *span,
            },
            Stmt::DoLoop {
                condition,
                body,
                span,
            } => Stmt::DoLoop {
                condition: condition.substitute_generics(bindings),
                body: body
                    .iter()
                    .map(|s| s.substitute_generics(bindings))
                    .collect(),
                span: *span,
            },
            Stmt::For {
                variable,
                start,
                end,
                step,
                next_variable,
                body,
                span,
            } => Stmt::For {
                variable: variable.clone(),
                start: start.substitute_generics(bindings),
                end: end.substitute_generics(bindings),
                step: step.as_ref().map(|s| s.substitute_generics(bindings)),
                next_variable: next_variable.clone(),
                body: body
                    .iter()
                    .map(|s| s.substitute_generics(bindings))
                    .collect(),
                span: *span,
            },
            Stmt::ForEach {
                variable,
                iterable,
                next_variable,
                body,
                span,
            } => Stmt::ForEach {
                variable: variable.clone(),
                iterable: iterable.substitute_generics(bindings),
                next_variable: next_variable.clone(),
                body: body
                    .iter()
                    .map(|s| s.substitute_generics(bindings))
                    .collect(),
                span: *span,
            },
            Stmt::ReDim {
                target,
                dims,
                preserve,
                span,
            } => Stmt::ReDim {
                target: target.substitute_generics(bindings),
                dims: dims
                    .iter()
                    .map(|(l, u)| {
                        (
                            l.as_ref().map(|e| e.substitute_generics(bindings)),
                            u.substitute_generics(bindings),
                        )
                    })
                    .collect(),
                preserve: *preserve,
                span: *span,
            },
            Stmt::Erase { target, span } => Stmt::Erase {
                target: target.substitute_generics(bindings),
                span: *span,
            },
            Stmt::LSet { target, expr, span } => Stmt::LSet {
                target: target.substitute_generics(bindings),
                expr: expr.substitute_generics(bindings),
                span: *span,
            },
            Stmt::RSet { target, expr, span } => Stmt::RSet {
                target: target.substitute_generics(bindings),
                expr: expr.substitute_generics(bindings),
                span: *span,
            },
            Stmt::With { target, body, span } => Stmt::With {
                target: target.substitute_generics(bindings),
                body: body
                    .iter()
                    .map(|s| s.substitute_generics(bindings))
                    .collect(),
                span: *span,
            },
            Stmt::Using {
                resource,
                body,
                span,
            } => Stmt::Using {
                resource: resource.substitute_generics(bindings),
                body: body
                    .iter()
                    .map(|s| s.substitute_generics(bindings))
                    .collect(),
                span: *span,
            },
            Stmt::TryCatch {
                try_body,
                catch_block,
                finally_body,
                span,
            } => Stmt::TryCatch {
                try_body: try_body
                    .iter()
                    .map(|s| s.substitute_generics(bindings))
                    .collect(),
                catch_block: catch_block
                    .as_ref()
                    .map(|b| b.substitute_generics(bindings)),
                finally_body: finally_body
                    .as_ref()
                    .map(|b| b.iter().map(|s| s.substitute_generics(bindings)).collect()),
                span: *span,
            },
            Stmt::DebugPrint { args, span } => Stmt::DebugPrint {
                args: args
                    .iter()
                    .map(|arg| arg.substitute_generics(bindings))
                    .collect(),
                span: *span,
            },
            Stmt::OpenFile {
                path,
                mode,
                access,
                lock,
                shared,
                number,
                record_len,
                span,
            } => Stmt::OpenFile {
                path: path.substitute_generics(bindings),
                mode: *mode,
                access: *access,
                lock: *lock,
                shared: *shared,
                number: number.substitute_generics(bindings),
                record_len: record_len.as_ref().map(|e| e.substitute_generics(bindings)),
                span: *span,
            },
            Stmt::CloseFile { numbers, span } => Stmt::CloseFile {
                numbers: numbers
                    .iter()
                    .map(|n| n.substitute_generics(bindings))
                    .collect(),
                span: *span,
            },
            Stmt::LineInput {
                number,
                target,
                span,
            } => Stmt::LineInput {
                number: number.substitute_generics(bindings),
                target: target.substitute_generics(bindings),
                span: *span,
            },
            Stmt::InputFile {
                number,
                targets,
                span,
            } => Stmt::InputFile {
                number: number.substitute_generics(bindings),
                targets: targets
                    .iter()
                    .map(|t| t.substitute_generics(bindings))
                    .collect(),
                span: *span,
            },
            Stmt::PrintFile {
                number,
                items,
                trailing,
                span,
            } => Stmt::PrintFile {
                number: number.substitute_generics(bindings),
                items: items
                    .iter()
                    .map(|i| i.substitute_generics(bindings))
                    .collect(),
                trailing: *trailing,
                span: *span,
            },
            Stmt::WriteFile { number, args, span } => Stmt::WriteFile {
                number: number.substitute_generics(bindings),
                args: args
                    .iter()
                    .map(|arg| arg.substitute_generics(bindings))
                    .collect(),
                span: *span,
            },
            Stmt::GetFile {
                number,
                position,
                target,
                span,
            } => Stmt::GetFile {
                number: number.substitute_generics(bindings),
                position: position.as_ref().map(|e| e.substitute_generics(bindings)),
                target: target.substitute_generics(bindings),
                span: *span,
            },
            Stmt::PutFile {
                number,
                position,
                expr,
                span,
            } => Stmt::PutFile {
                number: number.substitute_generics(bindings),
                position: position.as_ref().map(|e| e.substitute_generics(bindings)),
                expr: expr.substitute_generics(bindings),
                span: *span,
            },
            Stmt::SeekFile {
                number,
                position,
                span,
            } => Stmt::SeekFile {
                number: number.substitute_generics(bindings),
                position: position.substitute_generics(bindings),
                span: *span,
            },
            Stmt::NameFile {
                old_path,
                new_path,
                span,
            } => Stmt::NameFile {
                old_path: old_path.substitute_generics(bindings),
                new_path: new_path.substitute_generics(bindings),
                span: *span,
            },
            Stmt::Yield { expr, span } => Stmt::Yield {
                expr: expr.substitute_generics(bindings),
                span: *span,
            },
            Stmt::Throw { expr, span } => Stmt::Throw {
                expr: expr.substitute_generics(bindings),
                span: *span,
            },
            _ => self.clone(),
        }
    }
}

impl VariableDecl {
    pub fn substitute_generics(&self, bindings: &[(String, TypeName)]) -> Self {
        VariableDecl {
            name: self.name.clone(),
            ty: self.ty.as_ref().map(|ty| ty.substitute_generics(bindings)),
            array: self.array.clone(),
            as_new: self.as_new,
            new_args: self
                .new_args
                .iter()
                .map(|arg| arg.substitute_generics(bindings))
                .collect(),
            initializer: self
                .initializer
                .as_ref()
                .map(|init| init.substitute_generics(bindings)),
            collection_initializer: self.collection_initializer.as_ref().map(|init| {
                init.iter()
                    .map(|arg| arg.substitute_generics(bindings))
                    .collect()
            }),
            span: self.span,
        }
    }
}

impl AssignTarget {
    pub fn substitute_generics(&self, bindings: &[(String, TypeName)]) -> Self {
        match self {
            AssignTarget::ArrayElement {
                name,
                indices,
                span,
            } => AssignTarget::ArrayElement {
                name: name.clone(),
                indices: indices
                    .iter()
                    .map(|i| i.substitute_generics(bindings))
                    .collect(),
                span: *span,
            },
            AssignTarget::Member {
                object,
                field,
                span,
            } => AssignTarget::Member {
                object: object.substitute_generics(bindings),
                field: field.clone(),
                span: *span,
            },
            AssignTarget::MemberArrayElement {
                object,
                field,
                indices,
                span,
            } => AssignTarget::MemberArrayElement {
                object: object.substitute_generics(bindings),
                field: field.clone(),
                indices: indices
                    .iter()
                    .map(|i| i.substitute_generics(bindings))
                    .collect(),
                span: *span,
            },
            _ => self.clone(),
        }
    }
}

impl ElseIfBranch {
    pub fn substitute_generics(&self, bindings: &[(String, TypeName)]) -> Self {
        ElseIfBranch {
            condition: self.condition.substitute_generics(bindings),
            body: self
                .body
                .iter()
                .map(|s| s.substitute_generics(bindings))
                .collect(),
        }
    }
}

impl CaseBranch {
    pub fn substitute_generics(&self, bindings: &[(String, TypeName)]) -> Self {
        CaseBranch {
            items: self
                .items
                .iter()
                .map(|i| i.substitute_generics(bindings))
                .collect(),
            body: self
                .body
                .iter()
                .map(|s| s.substitute_generics(bindings))
                .collect(),
        }
    }
}

impl CaseItem {
    pub fn substitute_generics(&self, bindings: &[(String, TypeName)]) -> Self {
        match self {
            CaseItem::Value(e) => CaseItem::Value(e.substitute_generics(bindings)),
            CaseItem::Range { start, end } => CaseItem::Range {
                start: start.substitute_generics(bindings),
                end: end.substitute_generics(bindings),
            },
            CaseItem::Compare { op, expr } => CaseItem::Compare {
                op: *op,
                expr: expr.substitute_generics(bindings),
            },
        }
    }
}

impl DoLoopCondition {
    pub fn substitute_generics(&self, bindings: &[(String, TypeName)]) -> Self {
        match self {
            DoLoopCondition::PreWhile(e) => {
                DoLoopCondition::PreWhile(e.substitute_generics(bindings))
            }
            DoLoopCondition::PreUntil(e) => {
                DoLoopCondition::PreUntil(e.substitute_generics(bindings))
            }
            DoLoopCondition::PostWhile(e) => {
                DoLoopCondition::PostWhile(e.substitute_generics(bindings))
            }
            DoLoopCondition::PostUntil(e) => {
                DoLoopCondition::PostUntil(e.substitute_generics(bindings))
            }
            DoLoopCondition::Infinite => DoLoopCondition::Infinite,
        }
    }
}

impl CatchBlock {
    pub fn substitute_generics(&self, bindings: &[(String, TypeName)]) -> Self {
        CatchBlock {
            variable: self.variable.clone(),
            body: self
                .body
                .iter()
                .map(|s| s.substitute_generics(bindings))
                .collect(),
            span: self.span,
        }
    }
}

impl UsingResource {
    pub fn substitute_generics(&self, bindings: &[(String, TypeName)]) -> Self {
        match self {
            UsingResource::Declaration(d) => {
                UsingResource::Declaration(d.substitute_generics(bindings))
            }
            UsingResource::Target(e) => UsingResource::Target(e.substitute_generics(bindings)),
        }
    }
}

impl ReDimTarget {
    pub fn substitute_generics(&self, bindings: &[(String, TypeName)]) -> Self {
        match self {
            ReDimTarget::Member {
                object,
                field,
                span,
            } => ReDimTarget::Member {
                object: object.substitute_generics(bindings),
                field: field.clone(),
                span: *span,
            },
            _ => self.clone(),
        }
    }
}

impl PrintItem {
    pub fn substitute_generics(&self, bindings: &[(String, TypeName)]) -> Self {
        PrintItem {
            separator: self.separator,
            expr: self.expr.substitute_generics(bindings),
        }
    }
}
