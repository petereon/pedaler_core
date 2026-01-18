//! Voltage and current sources.

use crate::circuit::{BranchId, ComponentId, NodeId};

/// A voltage source component.
///
/// Voltage sources require an extra row/column in the MNA matrix for the
/// branch current. The source enforces: V+ - V- = V_source
#[derive(Debug, Clone)]
pub struct VoltageSource {
    pub id: ComponentId,
    pub name: String,
    pub nodes: [NodeId; 2], // [positive, negative]
    pub dc_value: f64,
    pub branch: BranchId,
    /// If true, the DC value is modulated by audio input
    pub is_audio_input: bool,
    /// Current value (may be modulated)
    pub current_value: f64,
}

impl VoltageSource {
    /// Create a new voltage source.
    pub fn new(
        id: ComponentId,
        name: String,
        nodes: [NodeId; 2],
        dc_value: f64,
        branch: BranchId,
        is_audio_input: bool,
    ) -> Self {
        Self {
            id,
            name,
            nodes,
            dc_value,
            branch,
            is_audio_input,
            current_value: dc_value,
        }
    }

    /// Set the source value (used for audio input modulation).
    pub fn set_value(&mut self, value: f64) {
        self.current_value = value;
    }

    /// Get the current source voltage.
    pub fn voltage(&self) -> f64 {
        self.current_value
    }
}

/// A current source component.
///
/// Current sources add directly to the RHS vector of the MNA equations.
#[derive(Debug, Clone)]
pub struct CurrentSource {
    pub id: ComponentId,
    pub name: String,
    pub nodes: [NodeId; 2], // [positive, negative] - current flows from + to -
    pub dc_value: f64,
    /// Current value (may be modulated)
    pub current_value: f64,
}

impl CurrentSource {
    /// Create a new current source.
    pub fn new(id: ComponentId, name: String, nodes: [NodeId; 2], dc_value: f64) -> Self {
        Self {
            id,
            name,
            nodes,
            dc_value,
            current_value: dc_value,
        }
    }

    /// Set the source value.
    pub fn set_value(&mut self, value: f64) {
        self.current_value = value;
    }

    /// Get the current source value.
    pub fn current(&self) -> f64 {
        self.current_value
    }
}
