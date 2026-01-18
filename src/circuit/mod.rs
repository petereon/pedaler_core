//! Circuit graph representation and validation.
//!
//! This module provides the internal representation of a circuit after parsing.
//! The [`Circuit`] struct holds all components, nodes, and their connections
//! in a form suitable for simulation.

mod graph;
mod types;
mod validate;

pub use graph::{Circuit, DelayDef, LfoDef, ReverbDef};
pub use types::*;
pub use validate::validate_circuit;
