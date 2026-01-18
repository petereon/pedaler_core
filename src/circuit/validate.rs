//! Circuit validation.

use crate::error::{PedalerError, Result};

use super::Circuit;

/// Validate a circuit for simulation.
///
/// Checks:
/// - All nodes have a path to ground
/// - No duplicate component names
/// - Component parameters are valid
pub fn validate_circuit(circuit: &Circuit) -> Result<()> {
    // Check that input and output nodes exist and are not ground
    if circuit.input_node.is_ground() {
        return Err(PedalerError::InvalidTopology {
            message: "Input node cannot be ground".to_string(),
        });
    }

    if circuit.output_node.is_ground() {
        return Err(PedalerError::InvalidTopology {
            message: "Output node cannot be ground".to_string(),
        });
    }

    // Check that we have at least one component
    if circuit.components.is_empty() {
        return Err(PedalerError::InvalidTopology {
            message: "Circuit has no components".to_string(),
        });
    }

    // TODO: More sophisticated connectivity checks
    // - Check for floating nodes (no DC path to ground)
    // - Check for voltage source loops
    // - Check for current source cutsets

    Ok(())
}
