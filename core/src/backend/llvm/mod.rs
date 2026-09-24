//! Experimental LLVM 22 toolchain adapter. MIR contains no LLVM-specific data.
mod emit;
mod ir;
mod layout;

pub use emit::{Artifact, EmitKind, LlvmTools, NativeOptions, build};
pub use ir::{Target, render_module};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendError {
    pub stage: &'static str,
    pub message: String,
}

impl BackendError {
    pub(crate) fn new(stage: &'static str, message: impl Into<String>) -> Self {
        Self {
            stage,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for BackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.stage, self.message)
    }
}

impl std::error::Error for BackendError {}
