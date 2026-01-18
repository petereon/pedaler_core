//! Circuit graph structure.

use std::collections::HashMap;

use super::types::{BranchId, ComponentId, NodeId};
use crate::components::Component;
use crate::dsl::{CircuitAst, ComponentType};
use crate::error::{PedalerError, Result};

/// Definition of a digital delay effect (stored for later instantiation).
#[derive(Debug, Clone)]
pub struct DelayDef {
    /// Component name
    pub name: String,
    /// Input node
    pub input_node: NodeId,
    /// Output node
    pub output_node: NodeId,
    /// Delay time in seconds
    pub delay_time: f64,
    /// Dry/wet mix (0.0-1.0)
    pub mix: f32,
    /// Feedback amount (0.0-1.0)
    pub feedback: f32,
    /// Branch ID for the voltage source (output driver)
    pub branch: BranchId,
}

/// Definition of a digital reverb effect (stored for later instantiation).
#[derive(Debug, Clone)]
pub struct ReverbDef {
    /// Component name
    pub name: String,
    /// Input node
    pub input_node: NodeId,
    /// Output node
    pub output_node: NodeId,
    /// Parameters
    pub params: HashMap<String, f64>,
    /// Branch ID for the voltage source (output driver)
    pub branch: BranchId,
}

/// Definition of an LFO (Low Frequency Oscillator) for modulation.
#[derive(Debug, Clone)]
pub struct LfoDef {
    /// Component name (used to reference from modulated resistors)
    pub name: String,
    /// Oscillation rate in Hz
    pub rate: f64,
    /// Waveform shape (sine, triangle, sawtooth, square)
    pub shape: String,
}

/// A complete circuit ready for simulation.
#[derive(Debug)]
pub struct Circuit {
    /// All components in the circuit
    pub components: Vec<Component>,

    /// Mapping from node names to node IDs
    pub node_map: HashMap<String, NodeId>,

    /// Reverse mapping from node IDs to names (for error messages)
    pub node_names: Vec<String>,

    /// Number of nodes (including ground)
    pub num_nodes: usize,

    /// Number of branch current variables (voltage sources, inductors)
    pub num_branches: usize,

    /// Input node ID (where audio signal is injected)
    pub input_node: NodeId,

    /// Output node ID (where audio signal is read)
    pub output_node: NodeId,

    /// Index of the input voltage source component
    pub input_source_idx: Option<usize>,

    /// Digital delay effect definitions
    pub delay_defs: Vec<DelayDef>,

    /// Digital reverb effect definitions
    pub reverb_defs: Vec<ReverbDef>,

    /// LFO definitions for modulation
    pub lfo_defs: Vec<LfoDef>,
}

impl Circuit {
    /// Build a circuit from a parsed AST.
    pub fn from_ast(ast: CircuitAst) -> Result<Self> {
        let mut node_map = HashMap::new();
        let mut node_names = Vec::new();

        // Ground is always node 0
        node_map.insert("0".to_string(), NodeId::GROUND);
        node_map.insert("GND".to_string(), NodeId::GROUND);
        node_names.push("0".to_string());

        // Assign IDs to all other nodes
        let mut next_id = 1usize;
        for node_name in &ast.nodes {
            if !node_map.contains_key(node_name) {
                node_map.insert(node_name.clone(), NodeId(next_id));
                node_names.push(node_name.clone());
                next_id += 1;
            }
        }

        // Also add nodes from components that might not be in the explicit list
        // Skip LFO components - they don't have electrical nodes
        for comp in &ast.components {
            // LFOs are purely control signals, not part of the circuit topology
            if comp.component_type == ComponentType::Lfo {
                continue;
            }
            // Also skip digital effects
            if comp.component_type == ComponentType::Delay || comp.component_type == ComponentType::Reverb {
                continue;
            }
            for node_name in &comp.nodes {
                let normalized = if node_name == "GND" { "0" } else { node_name };
                if !node_map.contains_key(normalized) {
                    node_map.insert(normalized.to_string(), NodeId(next_id));
                    node_names.push(normalized.to_string());
                    next_id += 1;
                }
            }
        }

        let num_nodes = next_id;

        // Get input/output nodes
        let input_node_name = ast.input_node.as_ref().ok_or(PedalerError::MissingInput)?;
        let output_node_name = ast.output_node.as_ref().ok_or(PedalerError::MissingOutput)?;

        let input_node = *node_map
            .get(input_node_name)
            .ok_or_else(|| PedalerError::NodeNotFound {
                node: input_node_name.clone(),
            })?;

        let output_node = *node_map
            .get(output_node_name)
            .ok_or_else(|| PedalerError::NodeNotFound {
                node: output_node_name.clone(),
            })?;

        // Convert components
        let mut components = Vec::with_capacity(ast.components.len());
        let mut delay_defs = Vec::new();
        let mut reverb_defs = Vec::new();
        let mut lfo_defs = Vec::new();
        let mut num_branches = 0usize;
        let mut input_source_idx = None;

        for (idx, comp_def) in ast.components.into_iter().enumerate() {
            // Resolve node names to IDs
            let nodes: Vec<NodeId> = comp_def
                .nodes
                .iter()
                .map(|name| {
                    let normalized = if name == "GND" { "0" } else { name };
                    node_map.get(normalized).copied().ok_or_else(|| {
                        PedalerError::NodeNotFound {
                            node: name.clone(),
                        }
                    })
                })
                .collect::<Result<Vec<_>>>()?;

            // Handle digital effects and LFOs separately - they need special treatment
            match comp_def.component_type {
                ComponentType::Delay => {
                    let delay_time = comp_def.value.unwrap_or(0.1); // Default 100ms
                    let mix = comp_def.params.get("mix").copied().unwrap_or(0.5) as f32;
                    let feedback = comp_def.params.get("feedback").copied().unwrap_or(0.3) as f32;
                    // Assign a branch for the output voltage source
                    let branch = BranchId(num_branches);
                    num_branches += 1;
                    delay_defs.push(DelayDef {
                        name: comp_def.name.clone(),
                        input_node: nodes[0],
                        output_node: nodes[1],
                        delay_time,
                        mix,
                        feedback,
                        branch,
                    });
                    continue;
                }
                ComponentType::Reverb => {
                    // Assign a branch for the output voltage source
                    let branch = BranchId(num_branches);
                    num_branches += 1;
                    reverb_defs.push(ReverbDef {
                        name: comp_def.name.clone(),
                        input_node: nodes[0],
                        output_node: nodes[1],
                        params: comp_def.params.clone(),
                        branch,
                    });
                    continue;
                }
                ComponentType::Lfo => {
                    let rate = comp_def.value.unwrap_or(0.5); // Default 0.5 Hz
                    let shape = comp_def.model_ref.clone().unwrap_or_else(|| "sine".to_string());
                    lfo_defs.push(LfoDef {
                        name: comp_def.name.clone(),
                        rate,
                        shape,
                    });
                    continue;
                }
                _ => {}
            }

            // Look up model if referenced
            let model = comp_def
                .model_ref
                .as_ref()
                .and_then(|name| ast.models.get(name).cloned());

            // Build component
            let component = Component::from_def(
                ComponentId(idx),
                &comp_def,
                &nodes,
                model.as_ref(),
                &mut num_branches,
            )?;

            // Check if this is the input voltage source
            if let Component::VoltageSource(ref vs) = component {
                if vs.nodes[0] == input_node || vs.nodes[1] == input_node {
                    input_source_idx = Some(components.len());
                }
            }

            components.push(component);
        }

        Ok(Circuit {
            components,
            node_map,
            node_names,
            num_nodes,
            num_branches,
            input_node,
            output_node,
            input_source_idx,
            delay_defs,
            reverb_defs,
            lfo_defs,
        })
    }

    /// Get the total size of the MNA solution vector.
    pub fn matrix_size(&self) -> usize {
        // Nodes (excluding ground) + branch currents
        (self.num_nodes - 1) + self.num_branches
    }

    /// Get the matrix index for a node voltage.
    /// Returns None for ground (node 0).
    pub fn node_index(&self, node: NodeId) -> Option<usize> {
        if node.is_ground() {
            None
        } else {
            Some(node.0 - 1)
        }
    }

    /// Get the matrix index for a branch current.
    pub fn branch_index(&self, branch: BranchId) -> usize {
        (self.num_nodes - 1) + branch.0
    }

    /// Find a node ID by name.
    pub fn find_node(&self, name: &str) -> Option<NodeId> {
        self.node_map.get(name).copied()
    }

    /// Get the name of a node.
    pub fn node_name(&self, node: NodeId) -> &str {
        &self.node_names[node.0]
    }
}
