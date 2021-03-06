//!
//! The Zinc virtual machine error.
//!

use failure::Fail;
use num::BigInt;

use franklin_crypto::bellman::SynthesisError;

use zinc_build::ScalarType;

#[derive(Debug, Fail)]
pub enum TypeSizeError {
    #[fail(display = "expected input value of size {}, got {}", expected, found)]
    Input { expected: usize, found: usize },

    #[fail(display = "expected output value of size {}, got {}", expected, found)]
    Output { expected: usize, found: usize },
}

#[derive(Debug, Fail)]
pub enum MalformedBytecode {
    #[fail(display = "invalid arguments to an intrinsic function: {}", _0)]
    InvalidArguments(String),

    #[fail(display = "unexpected `loop_end` instruction")]
    UnexpectedLoopEnd,

    #[fail(display = "unexpected `return` instruction")]
    UnexpectedReturn,

    #[fail(display = "unexpected `else` instruction")]
    UnexpectedElse,

    #[fail(display = "unexpected `end_if` instruction")]
    UnexpectedEndIf,

    #[fail(display = "stack underflow")]
    StackUnderflow,

    #[fail(display = "reading uninitialized memory")]
    UninitializedStorageAccess,

    #[fail(display = "conditional branches produced results of different sizes")]
    BranchStacksDoNotMatch,
}

#[derive(Debug, Fail)]
pub enum VerificationError {
    #[fail(display = "value overflow: value {} is not in the field", _0)]
    ValueOverflow(BigInt),

    #[fail(display = "failed to synthesize circuit: {}", _0)]
    SynthesisError(SynthesisError),
}

#[derive(Debug, Fail)]
pub enum RuntimeError {
    #[fail(display = "synthesis error: {}", _0)]
    SynthesisError(SynthesisError),

    #[fail(display = "internal error in virtual machine: {}", _0)]
    InternalError(String),

    #[fail(display = "malformed bytecode: {}", _0)]
    MalformedBytecode(MalformedBytecode),

    #[fail(display = "require error: {}", _0)]
    RequireError(String),

    #[fail(
        display = "index out of bounds: expected index in range {}..{}, got {}",
        lower_bound, upper_bound, found
    )]
    IndexOutOfBounds {
        lower_bound: usize,
        upper_bound: usize,
        found: usize,
    },

    #[fail(display = "type error: expected {}, got {}", expected, found)]
    TypeError { expected: String, found: String },

    #[fail(display = "constant value expected, got variable (witness)")]
    ExpectedConstant,

    #[fail(display = "size is too large: {}", _0)]
    ExpectedUsize(BigInt),

    #[fail(display = "value overflow or constraint violation")]
    UnsatisfiedConstraint,

    #[fail(display = "division by zero")]
    DivisionByZero,

    #[fail(display = "inverting zero")]
    ZeroInversion,

    #[fail(display = "type size mismatch: {}", _0)]
    TypeSize(TypeSizeError),

    #[fail(
        display = "overflow: value {} is not in range of type {}",
        value, scalar_type
    )]
    ValueOverflow {
        value: BigInt,
        scalar_type: ScalarType,
    },

    #[fail(display = "the unit test data is missing")]
    UnitTestDataMissing,

    #[fail(display = "the instruction is available only for contracts")]
    OnlyForContracts,

    #[fail(display = "invalid storage value")]
    InvalidStorageValue,

    #[fail(display = "contract method `{}` does not exist", _0)]
    MethodNotFound { found: String },
}

impl From<SynthesisError> for RuntimeError {
    fn from(error: SynthesisError) -> Self {
        RuntimeError::SynthesisError(error)
    }
}

impl From<MalformedBytecode> for RuntimeError {
    fn from(error: MalformedBytecode) -> Self {
        RuntimeError::MalformedBytecode(error)
    }
}

impl From<TypeSizeError> for RuntimeError {
    fn from(error: TypeSizeError) -> Self {
        RuntimeError::TypeSize(error)
    }
}
