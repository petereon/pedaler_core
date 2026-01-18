//! Core types for circuit representation.

use std::fmt;

/// A unique identifier for a node in the circuit.
/// Node 0 is always ground.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(pub usize);

impl NodeId {
    /// The ground node (always index 0).
    pub const GROUND: NodeId = NodeId(0);

    /// Check if this is the ground node.
    pub fn is_ground(&self) -> bool {
        self.0 == 0
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_ground() {
            write!(f, "GND")
        } else {
            write!(f, "N{}", self.0)
        }
    }
}

/// A unique identifier for a component in the circuit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ComponentId(pub usize);

impl fmt::Display for ComponentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "C{}", self.0)
    }
}

/// Index for extra variables in the MNA matrix (e.g., voltage source currents).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BranchId(pub usize);

impl fmt::Display for BranchId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "I{}", self.0)
    }
}

/// Variable index in the MNA solution vector.
/// Can be either a node voltage or a branch current.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VarIndex {
    /// Node voltage variable
    Voltage(NodeId),
    /// Branch current variable (for voltage sources, inductors)
    Current(BranchId),
}

impl VarIndex {
    /// Get the raw index into the solution vector.
    /// Node voltages come first (excluding ground), then branch currents.
    pub fn to_index(&self, num_nodes: usize) -> usize {
        match self {
            // Node 0 (ground) is not in the matrix, so subtract 1
            VarIndex::Voltage(NodeId(n)) => {
                debug_assert!(*n > 0, "Ground node should not be in solution vector");
                n - 1
            }
            // Branch currents come after node voltages
            VarIndex::Current(BranchId(b)) => (num_nodes - 1) + b,
        }
    }
}
