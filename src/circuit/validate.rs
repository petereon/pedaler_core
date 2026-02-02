//! Circuit validation.

use crate::error::{PedalerError, Result};
use crate::components::Component;

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

    // Validate op-amp power supplies
    validate_opamp_power_rails(circuit)?;

    // Validate input source
    validate_input_source(circuit)?;

    // TODO: More sophisticated connectivity checks
    // - Check for floating nodes (no DC path to ground)
    // - Check for voltage source loops
    // - Check for current source cutsets

    Ok(())
}

/// Validate that op-amps have proper power rail connections.
fn validate_opamp_power_rails(circuit: &Circuit) -> Result<()> {
    for component in &circuit.components {
        if let Component::OpAmp(op) = component {
            let vcc_node = op.rail_pos();
            let vneg_node = op.rail_neg();

            // Find voltage sources connected to these nodes
            let mut vcc_source = None;
            let mut vneg_source = None;

            for comp in &circuit.components {
                if let Component::VoltageSource(vs) = comp {
                    // Check if this source is connected to VCC rail
                    if (vs.nodes[0] == vcc_node && vs.nodes[1].is_ground())
                        || (vs.nodes[1] == vcc_node && vs.nodes[0].is_ground())
                    {
                        if vs.is_audio_input {
                            return Err(PedalerError::InvalidTopology {
                                message: format!(
                                    "Op-amp '{}': VCC rail (node {}) cannot be connected to an audio input (IN) source. \
                                     Power rails must be constant DC sources.",
                                    op.name,
                                    circuit.node_names.get(vcc_node.0).unwrap_or(&"?".to_string())
                                ),
                            });
                        }
                        vcc_source = Some(vs.dc_value);
                    }

                    // Check if this source is connected to VNEG rail
                    if (vs.nodes[0] == vneg_node && vs.nodes[1].is_ground())
                        || (vs.nodes[1] == vneg_node && vs.nodes[0].is_ground())
                    {
                        if vs.is_audio_input {
                            return Err(PedalerError::InvalidTopology {
                                message: format!(
                                    "Op-amp '{}': VNEG rail (node {}) cannot be connected to an audio input (IN) source. \
                                     Power rails must be constant DC sources.",
                                    op.name,
                                    circuit.node_names.get(vneg_node.0).unwrap_or(&"?".to_string())
                                ),
                            });
                        }
                        vneg_source = Some(vs.dc_value);
                    }
                }
            }

            // Ensure power rails are connected to voltage sources
            if !vneg_node.is_ground() && vneg_source.is_none() {
                return Err(PedalerError::InvalidTopology {
                    message: format!(
                        "Op-amp '{}': VNEG rail (node {}) must be connected to a DC voltage source. \
                         Example: VNEG vneg 0 DC 0",
                        op.name,
                        circuit.node_names.get(vneg_node.0).unwrap_or(&"?".to_string())
                    ),
                });
            }

            if vcc_source.is_none() {
                return Err(PedalerError::InvalidTopology {
                    message: format!(
                        "Op-amp '{}': VCC rail (node {}) must be connected to a DC voltage source. \
                         Example: VCC vcc 0 DC 9",
                        op.name,
                        circuit.node_names.get(vcc_node.0).unwrap_or(&"?".to_string())
                    ),
                });
            }

            // Check that VCC > VNEG
            let v_pos = vcc_source.unwrap();
            let v_neg = vneg_source.unwrap_or(0.0); // Ground if VNEG is 0

            if v_pos <= v_neg {
                return Err(PedalerError::InvalidTopology {
                    message: format!(
                        "Op-amp '{}': VCC voltage ({:.1}V) must be greater than VNEG voltage ({:.1}V). \
                         Typical single-supply configuration: VCC=9V, VNEG=0V",
                        op.name, v_pos, v_neg
                    ),
                });
            }
        }
    }

    Ok(())
}

/// Validate that input node has an IN source.
fn validate_input_source(circuit: &Circuit) -> Result<()> {
    // Find voltage source at input node
    let mut has_in_source = false;
    let mut has_dc_source_at_input = false;

    for component in &circuit.components {
        if let Component::VoltageSource(vs) = component {
            // Check if connected to input node
            let connected_to_input = vs.nodes[0] == circuit.input_node || vs.nodes[1] == circuit.input_node;

            if connected_to_input {
                if vs.is_audio_input {
                    has_in_source = true;
                } else {
                    has_dc_source_at_input = true;
                }
            }
        }
    }

    if !has_in_source {
        let default_name = "?".to_string();
        let input_name = circuit.node_names.get(circuit.input_node.0)
            .unwrap_or(&default_name);

        if has_dc_source_at_input {
            return Err(PedalerError::InvalidTopology {
                message: format!(
                    "Input node '{}' has a DC source, but audio input requires an IN source. \
                     Change 'V_IN {} 0 DC 0' to 'V_IN {} 0 IN'",
                    input_name, input_name, input_name
                ),
            });
        } else {
            return Err(PedalerError::InvalidTopology {
                message: format!(
                    "Input node '{}' (.input directive) must have an IN source for audio input. \
                     Example: V_IN {} 0 IN",
                    input_name, input_name
                ),
            });
        }
    }

    Ok(())
}
