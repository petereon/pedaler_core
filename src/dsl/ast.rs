//! Abstract Syntax Tree types for the circuit DSL.

use std::collections::HashMap;

/// Complete AST representation of a parsed circuit.
#[derive(Debug, Clone)]
pub struct CircuitAst {
    /// All component instances
    pub components: Vec<ComponentDef>,
    /// Model definitions
    pub models: HashMap<String, ModelDef>,
    /// Input node name
    pub input_node: Option<String>,
    /// Output node name
    pub output_node: Option<String>,
    /// All referenced node names (including implicit ones)
    pub nodes: Vec<String>,
}

impl CircuitAst {
    /// Create a new empty circuit AST.
    pub fn new() -> Self {
        Self {
            components: Vec::new(),
            models: HashMap::new(),
            input_node: None,
            output_node: None,
            nodes: Vec::new(),
        }
    }
}

impl Default for CircuitAst {
    fn default() -> Self {
        Self::new()
    }
}

/// A component definition from the DSL.
#[derive(Debug, Clone)]
pub struct ComponentDef {
    /// Component type (R, C, L, D, Q, V, I, OP, POT, SW)
    pub component_type: ComponentType,
    /// Unique component name
    pub name: String,
    /// Connected node names
    pub nodes: Vec<String>,
    /// Component value (resistance, capacitance, etc.)
    pub value: Option<f64>,
    /// Reference to a model definition
    pub model_ref: Option<String>,
    /// Additional parameters
    pub params: HashMap<String, f64>,
    /// Source line number for error reporting
    pub line: usize,
}

/// Component types supported by the DSL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentType {
    /// Resistor
    Resistor,
    /// Capacitor
    Capacitor,
    /// Inductor
    Inductor,
    /// Diode
    Diode,
    /// Bipolar Junction Transistor
    Bjt,
    /// Voltage Source
    VoltageSource,
    /// Current Source
    CurrentSource,
    /// Operational Amplifier
    OpAmp,
    /// Potentiometer
    Potentiometer,
    /// Switch
    Switch,
    /// Digital Delay Line
    Delay,
    /// FDN Reverb
    Reverb,
    /// Low Frequency Oscillator (control signal)
    Lfo,
}

impl ComponentType {
    /// Parse a component type from its DSL prefix.
    pub fn from_prefix(prefix: char) -> Option<Self> {
        match prefix.to_ascii_uppercase() {
            'R' => Some(Self::Resistor),
            'C' => Some(Self::Capacitor),
            'L' => Some(Self::Inductor),
            'D' => Some(Self::Diode),
            'Q' => Some(Self::Bjt),
            'V' => Some(Self::VoltageSource),
            'I' => Some(Self::CurrentSource),
            _ => None,
        }
    }

    /// Parse a component type from a keyword.
    pub fn from_keyword(keyword: &str) -> Option<Self> {
        match keyword.to_ascii_uppercase().as_str() {
            "OP" | "OPAMP" => Some(Self::OpAmp),
            "POT" => Some(Self::Potentiometer),
            "SW" | "SWITCH" => Some(Self::Switch),
            "DELAY" => Some(Self::Delay),
            "REVERB" | "REV" => Some(Self::Reverb),
            "LFO" => Some(Self::Lfo),
            _ => None,
        }
    }

    /// Get the expected number of nodes for this component type.
    pub fn expected_node_count(&self) -> usize {
        match self {
            Self::Resistor | Self::Capacitor | Self::Inductor => 2,
            Self::Diode => 2,
            Self::Bjt => 3,        // collector, base, emitter
            Self::VoltageSource | Self::CurrentSource => 2,
            Self::OpAmp => 3,      // out, in+, in-
            Self::Potentiometer => 3, // n1, wiper, n2
            Self::Switch => 2,
            Self::Delay => 2,      // in, out
            Self::Reverb => 2,     // in, out
            Self::Lfo => 0,        // No electrical nodes - purely a control signal
        }
    }
}

/// A model definition (e.g., for diodes, BJTs).
#[derive(Debug, Clone)]
pub struct ModelDef {
    /// Model name
    pub name: String,
    /// Model type (D for diode, NPN/PNP for BJT, etc.)
    pub model_type: ModelType,
    /// Model parameters
    pub params: HashMap<String, f64>,
    /// Source line number
    pub line: usize,
}

/// Model types for parameterized components.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelType {
    /// Diode model
    Diode,
    /// NPN BJT model
    BjtNpn,
    /// PNP BJT model
    BjtPnp,
    /// Op-amp model
    OpAmp,
}

impl ModelType {
    /// Parse a model type from string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_ascii_uppercase().as_str() {
            "D" | "DIODE" => Some(Self::Diode),
            "NPN" => Some(Self::BjtNpn),
            "PNP" => Some(Self::BjtPnp),
            "OP" | "OPAMP" => Some(Self::OpAmp),
            _ => None,
        }
    }
}

/// Voltage source type.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SourceType {
    /// DC voltage/current
    Dc(f64),
    /// AC source (will be driven by audio input)
    Ac(f64),
    /// Combined DC bias and AC
    DcAc { dc: f64, ac: f64 },
}
