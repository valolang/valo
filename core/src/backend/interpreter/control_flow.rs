use crate::ResumeTarget;
use crate::runtime::Value;

#[derive(Debug, Clone)]
pub(crate) enum ControlFlow {
    Continue,
    Return(Value),
    ExitSub,
    ExitFunction,
    ExitFor,
    ExitWhile,
    ExitDo,
    GoTo(String),
    Resume(ResumeTarget),
    Terminate,
}
