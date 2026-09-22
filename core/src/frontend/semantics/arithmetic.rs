//! Target-independent arithmetic selection shared by checking, HIR and execution.
use crate::BinaryOp;
use crate::frontend::type_model::TypeName;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArithmeticOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    IntegerDivide,
    Modulo,
    Power,
}

impl ArithmeticOp {
    pub fn from_ast(op: BinaryOp) -> Option<Self> {
        Some(match op {
            BinaryOp::Add => Self::Add,
            BinaryOp::Subtract => Self::Subtract,
            BinaryOp::Multiply => Self::Multiply,
            BinaryOp::Divide => Self::Divide,
            BinaryOp::IntegerDivide => Self::IntegerDivide,
            BinaryOp::Modulo => Self::Modulo,
            BinaryOp::Exponent => Self::Power,
            _ => return None,
        })
    }
}

/// Both operands are converted to this type before performing the operation.
/// Integer add/subtract/multiply wrap at the selected width, independent of values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArithmeticSignature {
    pub operand_type: TypeName,
    pub result_type: TypeName,
}

pub fn signature(
    op: ArithmeticOp,
    left: &TypeName,
    right: &TypeName,
) -> Option<ArithmeticSignature> {
    use TypeName::*;
    let numeric =
        |t: &TypeName| t.is_integral() || matches!(t, Single | Double | Decimal | Currency | Date);
    if !numeric(left) || !numeric(right) {
        return None;
    }
    let extended = |t: &TypeName| matches!(t, Double | Decimal | Currency | Date);
    let ty = if matches!(op, ArithmeticOp::Divide | ArithmeticOp::Power) {
        Double
    } else if matches!(op, ArithmeticOp::IntegerDivide)
        && (!left.is_integral() || !right.is_integral())
    {
        Int64
    } else if extended(left) || extended(right) {
        Double
    } else if matches!(left, Single) || matches!(right, Single) {
        Single
    } else if matches!(left, UInt64) || matches!(right, UInt64) {
        // No signed integer type can represent every UInt64 value.
        if matches!(left, Int16 | Int32 | Int64) || matches!(right, Int16 | Int32 | Int64) {
            return None;
        }
        UInt64
    } else if matches!(left, Int64) || matches!(right, Int64) {
        Int64
    } else if matches!(left, UInt32) || matches!(right, UInt32) {
        if matches!(left, Int16 | Int32) || matches!(right, Int16 | Int32) {
            Int64
        } else {
            UInt32
        }
    } else {
        Int32
    };
    Some(ArithmeticSignature {
        operand_type: ty.clone(),
        result_type: ty,
    })
}
