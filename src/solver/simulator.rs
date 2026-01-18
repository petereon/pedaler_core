//! Main simulator interface.

use std::collections::HashMap;

use crate::circuit::{BranchId, Circuit, NodeId};
use crate::components::{Component, DelayLine, FdnReverb, Lfo, LfoShape, ReverbParams};
use crate::error::Result;

use super::mna::{stamp_linear_components, MnaMatrix};
use super::{NewtonRaphson, DEFAULT_MAX_ITERATIONS, DEFAULT_TOLERANCE};

/// Configuration for the simulator.
#[derive(Debug, Clone)]
pub struct SimulatorConfig {
    /// Maximum Newton-Raphson iterations for nonlinear components.
    pub max_iterations: usize,
    /// Convergence tolerance for Newton-Raphson (volts).
    pub tolerance: f64,
}

impl Default for SimulatorConfig {
    fn default() -> Self {
        Self {
            max_iterations: DEFAULT_MAX_ITERATIONS,
            tolerance: DEFAULT_TOLERANCE,
        }
    }
}

impl SimulatorConfig {
    /// Create a new configuration with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the maximum Newton-Raphson iterations.
    pub fn with_max_iterations(mut self, max_iterations: usize) -> Self {
        self.max_iterations = max_iterations;
        self
    }

    /// Set the convergence tolerance (in volts).
    ///
    /// Higher tolerance = faster convergence but less accuracy.
    /// - 1e-6 (default): Very precise, may need more iterations
    /// - 1e-4: Good balance for most audio applications
    /// - 1e-3: Fast, suitable for real-time with some accuracy loss
    pub fn with_tolerance(mut self, tolerance: f64) -> Self {
        self.tolerance = tolerance;
        self
    }
}

/// An in-circuit digital delay effect.
struct InCircuitDelay {
    effect: DelayLine,
    input_node: NodeId,
    output_node: NodeId,
    branch: BranchId,
    /// Current output voltage (set before MNA solve)
    output_voltage: f64,
}

/// An in-circuit digital reverb effect.
struct InCircuitReverb {
    effect: FdnReverb,
    input_node: NodeId,
    output_node: NodeId,
    branch: BranchId,
    /// Current output voltage (set before MNA solve)
    output_voltage: f64,
}

/// The main circuit simulator.
pub struct Simulator {
    /// The circuit being simulated
    circuit: Circuit,
    /// MNA matrix system
    matrix: MnaMatrix,
    /// Newton-Raphson solver
    newton: NewtonRaphson,
    /// Sample rate in Hz
    sample_rate: f32,
    /// Time step (1/sample_rate)
    dt: f64,
    /// In-circuit digital delay effects
    delays: Vec<InCircuitDelay>,
    /// In-circuit digital reverb effects
    reverbs: Vec<InCircuitReverb>,
    /// LFOs for modulation (keyed by name)
    lfos: HashMap<String, Lfo>,
    /// Whether the circuit has any modulated components
    has_modulation: bool,
}

impl Simulator {
    /// Create a new simulator for the given circuit with default configuration.
    pub fn new(circuit: Circuit, sample_rate: f32) -> Self {
        Self::with_config(circuit, sample_rate, SimulatorConfig::default())
    }

    /// Create a new simulator for the given circuit with custom configuration.
    pub fn with_config(circuit: Circuit, sample_rate: f32, config: SimulatorConfig) -> Self {
        let size = circuit.matrix_size();
        let matrix = MnaMatrix::new(size);
        let newton = NewtonRaphson::with_config(config.max_iterations, config.tolerance);
        let dt = 1.0 / sample_rate as f64;

        // Instantiate digital delay effects with their circuit connections
        let delays: Vec<InCircuitDelay> = circuit
            .delay_defs
            .iter()
            .map(|def| {
                InCircuitDelay {
                    effect: DelayLine::new(
                        def.name.clone(),
                        def.input_node,
                        def.output_node,
                        def.delay_time,
                        sample_rate,
                        def.mix,
                        def.feedback,
                    ),
                    input_node: def.input_node,
                    output_node: def.output_node,
                    branch: def.branch,
                    output_voltage: 0.0,
                }
            })
            .collect();

        // Instantiate digital reverb effects with their circuit connections
        let reverbs: Vec<InCircuitReverb> = circuit
            .reverb_defs
            .iter()
            .map(|def| {
                let params = ReverbParams::from_params(&def.params);
                InCircuitReverb {
                    effect: FdnReverb::new(
                        def.name.clone(),
                        def.input_node,
                        def.output_node,
                        params,
                        sample_rate,
                    ),
                    input_node: def.input_node,
                    output_node: def.output_node,
                    branch: def.branch,
                    output_voltage: 0.0,
                }
            })
            .collect();

        // Instantiate LFOs
        let lfos: HashMap<String, Lfo> = circuit
            .lfo_defs
            .iter()
            .map(|def| {
                let shape = LfoShape::from_str(&def.shape).unwrap_or_default();
                let lfo = Lfo::new(def.name.clone(), def.rate, shape, sample_rate as f64);
                (def.name.clone(), lfo)
            })
            .collect();

        // Check if any resistors are modulated
        let has_modulation = circuit.components.iter().any(|c| {
            matches!(c, Component::Resistor(r) if r.is_modulated())
        });

        Self {
            circuit,
            matrix,
            newton,
            sample_rate,
            dt,
            delays,
            reverbs,
            lfos,
            has_modulation,
        }
    }

    /// Get the sample rate.
    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }

    /// Set the input voltage (audio sample).
    pub fn set_input(&mut self, voltage: f32) {
        // Find the audio input voltage source and set its value
        if let Some(idx) = self.circuit.input_source_idx {
            if let Component::VoltageSource(ref mut vs) = self.circuit.components[idx] {
                vs.set_value(voltage as f64);
            }
        }
    }

    /// Update LFOs and modulated components.
    fn update_modulation(&mut self) {
        if !self.has_modulation {
            return;
        }

        // Tick all LFOs and collect their current values
        let lfo_values: HashMap<String, f64> = self.lfos
            .iter_mut()
            .map(|(name, lfo)| (name.clone(), lfo.tick()))
            .collect();

        // Update modulated resistors
        for component in &mut self.circuit.components {
            if let Component::Resistor(r) = component {
                if let Some(ref modulation) = r.modulation {
                    if let Some(&value) = lfo_values.get(&modulation.lfo_name) {
                        r.update_modulation(value);
                    }
                }
            }
        }
    }

    /// Step the simulation by one sample.
    pub fn step(&mut self) -> Result<f32> {
        // Update LFOs and modulated components before stamping
        self.update_modulation();

        // Clear the matrix
        self.matrix.clear();

        // Stamp linear components
        stamp_linear_components(&self.circuit, &mut self.matrix, self.dt);

        // Stamp digital effects as voltage sources
        // They use their output_voltage (computed from previous sample's input)
        self.stamp_digital_effects();

        // Solve (with Newton-Raphson if there are nonlinear components)
        self.newton.solve(&self.circuit, &mut self.matrix, self.dt)?;

        // Update reactive component states
        self.update_reactive_states();

        // Read input voltages for digital effects and process them
        // The processed values will be used as output in the next sample
        self.process_digital_effects();

        // Read output voltage from circuit
        let v_out = self.matrix.node_voltage(&self.circuit, self.circuit.output_node) as f32;

        Ok(v_out)
    }

    /// Stamp digital effects as voltage sources into the MNA matrix.
    fn stamp_digital_effects(&mut self) {
        let num_nodes = self.circuit.num_nodes;

        // Stamp delays as voltage sources: V(out) - V(in) = output_voltage
        for delay in &self.delays {
            let row = num_nodes - 1 + delay.branch.0;

            // Digital effect modeled as ideal voltage source between input and output
            // KCL: branch current enters output node, leaves input node
            // KVL: V(out) - V(in) = output_voltage

            // Add branch current to output node KCL
            if !delay.output_node.is_ground() {
                let out_idx = delay.output_node.0 - 1;
                self.matrix.add(out_idx, row, 1.0);
                self.matrix.add(row, out_idx, 1.0);
            }

            // Subtract branch current from input node KCL
            if !delay.input_node.is_ground() {
                let in_idx = delay.input_node.0 - 1;
                self.matrix.add(in_idx, row, -1.0);
                self.matrix.add(row, in_idx, -1.0);
            }

            // RHS: the processed voltage value
            self.matrix.add_source(row, delay.output_voltage);
        }

        // Stamp reverbs as voltage sources
        for reverb in &self.reverbs {
            let row = num_nodes - 1 + reverb.branch.0;

            if !reverb.output_node.is_ground() {
                let out_idx = reverb.output_node.0 - 1;
                self.matrix.add(out_idx, row, 1.0);
                self.matrix.add(row, out_idx, 1.0);
            }

            if !reverb.input_node.is_ground() {
                let in_idx = reverb.input_node.0 - 1;
                self.matrix.add(in_idx, row, -1.0);
                self.matrix.add(row, in_idx, -1.0);
            }

            self.matrix.add_source(row, reverb.output_voltage);
        }
    }

    /// Process digital effects: read input voltages and compute output for next sample.
    fn process_digital_effects(&mut self) {
        // Process delays
        for delay in &mut self.delays {
            // Read input voltage from the circuit
            let v_in = if delay.input_node.is_ground() {
                0.0
            } else {
                self.matrix.x[delay.input_node.0 - 1]
            };

            // Process through the delay and store for next sample
            delay.output_voltage = delay.effect.process(v_in as f32) as f64;
        }

        // Process reverbs
        for reverb in &mut self.reverbs {
            let v_in = if reverb.input_node.is_ground() {
                0.0
            } else {
                self.matrix.x[reverb.input_node.0 - 1]
            };

            reverb.output_voltage = reverb.effect.process(v_in as f32) as f64;
        }
    }

    /// Process a block of samples.
    pub fn process_block(&mut self, input: &[f32], output: &mut [f32]) -> Result<()> {
        for (i, &sample) in input.iter().enumerate() {
            self.set_input(sample);
            output[i] = self.step()?;
        }
        Ok(())
    }

    /// Update the state of reactive components (capacitors, inductors).
    fn update_reactive_states(&mut self) {
        let num_nodes = self.circuit.num_nodes;
        let dt = self.dt;

        for component in &mut self.circuit.components {
            match component {
                Component::Capacitor(c) => {
                    // Get node indices directly without borrowing circuit
                    let n1_idx = if c.nodes[0].is_ground() { None } else { Some(c.nodes[0].0 - 1) };
                    let n2_idx = if c.nodes[1].is_ground() { None } else { Some(c.nodes[1].0 - 1) };

                    let v1 = n1_idx.map(|i| self.matrix.x[i]).unwrap_or(0.0);
                    let v2 = n2_idx.map(|i| self.matrix.x[i]).unwrap_or(0.0);
                    let v = v1 - v2;
                    c.update_state(v, dt);
                }

                Component::Inductor(l) => {
                    // Branch index calculation without borrowing circuit
                    let br = (num_nodes - 1) + l.branch.0;
                    let i = self.matrix.x[br];
                    l.update_state(i, dt);
                }

                _ => {}
            }
        }
    }

    /// Get the current voltage at a node by name.
    pub fn node_voltage(&self, name: &str) -> Option<f64> {
        let node = self.circuit.find_node(name)?;
        Some(self.matrix.node_voltage(&self.circuit, node))
    }

    /// Get a reference to the circuit.
    pub fn circuit(&self) -> &Circuit {
        &self.circuit
    }
}
