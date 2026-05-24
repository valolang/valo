//! Callable/delegate metadata foundation.
//!
//! This models callable values independently from AST nodes. Lambdas, delegates,
//! event APIs, async callbacks, and future stdlib collection APIs can lower into
//! this shape without making the interpreter's current call dispatch the public
//! runtime contract.

use crate::runtime::TypeName;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DelegateType {
    pub name: String,
    pub params: Vec<CallableParam>,
    pub return_type: Option<TypeName>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallableParam {
    pub name: String,
    pub ty: TypeName,
    pub passing: CallablePassingMode,
    pub optional: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallablePassingMode {
    ByVal,
    ByRef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallableRef {
    pub target: CallableTarget,
    pub delegate_type: Option<DelegateType>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallableTarget {
    Function(String),
    Method {
        receiver_type: TypeName,
        name: String,
    },
    NativeCallback(usize),
}
