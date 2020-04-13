//! A linear IR for optimizations.
//!
//! This IR is designed such that it should be easy to combine multiple linear
//! optimizations into a single automata.
//!
//! See also `src/linearize.rs` for the AST to linear IR translation pass.

use crate::integer_interner::{IntegerId, IntegerInterner};
use crate::paths::{PathId, PathInterner};
use serde::{Deserialize, Serialize};

/// A set of linear optimizations.
#[derive(Debug)]
pub struct Optimizations {
    /// The linear optimizations.
    pub optimizations: Vec<Optimization>,

    /// The de-duplicated paths referenced by these optimizations.
    pub paths: PathInterner,

    /// The integer literals referenced by these optimizations.
    pub integers: IntegerInterner,
}

/// A linearized optimization.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Optimization {
    /// The chain of increments for this optimization.
    pub increments: Vec<Increment>,
}

/// An increment is a matching operation, the expected result from that
/// operation to continue to the next increment, and the actions to take to
/// build up the LHS scope and RHS instructions given that we got the expected
/// result from this increment's matching operation. Each increment will
/// basically become a state and a transition edge out of that state in the
/// final automata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Increment {
    /// The matching operation to perform.
    pub operation: MatchOp,

    /// The expected result of our matching operation, that enables us to
    /// continue to the next increment. `None` is used for wildcard-style "else"
    /// transitions.
    pub expected: Option<u32>,

    /// Actions to perform, given that the operation resulted in the expected
    /// value.
    pub actions: Vec<Action>,
}

/// A matching operation to be performed on some Cranelift instruction as part
/// of determining whether an optimization is applicable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum MatchOp {
    /// Switch on the opcode of an instruction.
    Opcode {
        /// The path to the instruction whose opcode we're switching on.
        path: PathId,
    },

    /// Does an instruction have a constant value?
    IsConst {
        /// The path to the instruction we're checking whether it is constant or
        /// not.
        path: PathId,
    },

    /// Is the constant value a power of two?
    IsPowerOfTwo {
        /// The id of the constant value that was bound in the left-hand side.
        id: LhsId,
    },

    /// Switch on the bit width of a value.
    BitWidth {
        /// The id of the value that was bound in the left-hand side.
        id: LhsId,
    },

    /// Is the instruction at the given path the same SSA value as the value
    /// bound on the left-hand side?
    Eq {
        /// The id of the value that was bound in the left-hand side.
        id: LhsId,
        /// The path to the instruction we're checking.
        path: PathId,
    },

    /// Switch on the constant integer value of an instruction.
    IntegerValue {
        /// The path to the instruction.
        path: PathId,
    },

    /// Switch on the constant boolean value of an instruction.
    BooleanValue {
        /// The path to the instruction.
        path: PathId,
    },

    /// No operation. Always evaluates to `None`.
    ///
    /// Exceedingly rare in real optimizations; nonetheless required to support
    /// corner cases of the DSL, such as a LHS pattern that is nothing but a
    /// variable pattern.
    Nop,
}

/// A canonicalized identifier for a left-hand side value that was bound in a
/// pattern.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LhsId(pub u32);

/// A canonicalized identifier for a right-hand side value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RhsId(pub u32);

/// An action to perform when transitioning between states in the automata.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Action {
    /// Bind `id = path` in the left-hand side scope.
    BindLhs {
        /// The canonicalized id being bound.
        id: LhsId,
        /// The path to the instruction or value.
        path: PathId,
    },

    /// Implicitly define the n^th built up RHS instruction as something from
    /// the left-hand side.
    GetLhsBinding {
        /// The thing from the left-hand side that is being reused in the
        /// right-hand side.
        id: LhsId,
    },

    /// Implicitly define the n^th RHS instruction as the log2 of a right-hand
    /// side value that is known to be a constant power of two.
    Log2 {
        /// The right-hand side operand.
        operand: RhsId,
    },

    /// Implicitly define the n^th built up RHS instruction by making an `iconst`.
    MakeIntegerConst {
        /// The constant integer value for the `iconst` instruction.
        value: IntegerId,
    },

    /// Implicitly define the n^th RHS instruction by making a `bconst`.
    MakeBooleanConst {
        /// The constant boolean value for the `bconst` instruction.
        value: bool,
    },

    /// Implicitly define the n^th RHS instruction by making an `ashr`.
    MakeAshr {
        /// The right-hand side operands for the `ashr`.
        operands: [RhsId; 2],
    },

    /// Implicitly define the n^th RHS instruction by making a `bor`.
    MakeBor {
        /// The right-hand side operands for the `bor`.
        operands: [RhsId; 2],
    },

    /// Implicitly define the n^th RHS instruction by making an `iadd`.
    MakeIadd {
        /// The right-hand side operands for the `iadd`.
        operands: [RhsId; 2],
    },

    /// Implicitly define the n^th RHS instruction by making an `iadd_imm`.
    MakeIaddImm {
        /// The right-hand side operands for the `iadd_imm`. The first must be a
        /// constant value.
        operands: [RhsId; 2],
    },

    /// Implicitly define the n^th RHS instruction by making an `iconst`.
    MakeIconst {
        /// The right-hand side operand for this `iconst`. Must be a constant
        /// value.
        operand: RhsId,
    },

    /// Implicitly define the n^th RHS instruction by making an `imul`.
    MakeImul {
        /// The right-hand side operands for this `imul`.
        operands: [RhsId; 2],
    },

    /// Implicitly define the n^th RHS instruction by making an `imul_imm`.
    MakeImulImm {
        /// The right-hand side operands for this `imul`. The first must be a
        /// constant value.
        operands: [RhsId; 2],
    },

    /// Implicitly define the n^th RHS instruction by making an `ishl`.
    MakeIshl {
        /// The right-hand side operands for this `ishl`.
        operands: [RhsId; 2],
    },

    /// Implicitly define the n^th RHS instruction by making a `sshr`.
    MakeSshr {
        /// The right-hand side operands for this `sshr`.
        operands: [RhsId; 2],
    },
}
