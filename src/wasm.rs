//! WASM bindings for Pedaler Core.
//!
//! This module provides JavaScript-friendly bindings for use in web browsers
//! with Web Audio API's AudioWorklet.
//!
//! ## Usage (JavaScript)
//!
//! ```javascript
//! import init, { WasmPedalSim } from 'pedaler_core';
//!
//! await init();
//!
//! const circuitDsl = `
//!   .input in
//!   .output out
//!   V_IN in 0 DC 0
//!   R1 in out 10k
//!   R2 out 0 10k
//! `;
//!
//! const sim = new WasmPedalSim(circuitDsl, 48000);
//!
//! // In AudioWorkletProcessor.process():
//! const input = inputBuffer.getChannelData(0);
//! const output = outputBuffer.getChannelData(0);
//! sim.process_block(input, output);
//! ```

use wasm_bindgen::prelude::*;

use crate::circuit::Circuit;
use crate::dsl;
use crate::solver::{Simulator, SimulatorConfig};

/// Initialize panic hook for better error messages in browser console.
#[wasm_bindgen(start)]
pub fn init_panic_hook() {
    console_error_panic_hook::set_once();
}

/// WASM-compatible guitar pedal circuit simulator.
///
/// This struct wraps the native `Simulator` and provides a JavaScript-friendly API
/// for processing audio blocks in Web Audio AudioWorklet.
#[wasm_bindgen]
pub struct WasmPedalSim {
    simulator: Simulator,
}

#[wasm_bindgen]
impl WasmPedalSim {
    /// Create a new simulator from a circuit DSL string.
    ///
    /// # Arguments
    /// * `circuit_dsl` - The circuit description in Pedaler DSL format
    /// * `sample_rate` - Audio sample rate in Hz (typically 44100 or 48000)
    ///
    /// # Returns
    /// A new `WasmPedalSim` instance or an error if the circuit is invalid.
    ///
    /// # Example
    /// ```javascript
    /// const sim = new WasmPedalSim(circuitDsl, 48000);
    /// ```
    #[wasm_bindgen(constructor)]
    pub fn new(circuit_dsl: &str, sample_rate: f32) -> Result<WasmPedalSim, JsValue> {
        Self::with_config(circuit_dsl, sample_rate, 50, 1e-4)
    }

    /// Create a new simulator with custom Newton-Raphson configuration.
    ///
    /// # Arguments
    /// * `circuit_dsl` - The circuit description in Pedaler DSL format
    /// * `sample_rate` - Audio sample rate in Hz
    /// * `max_iterations` - Maximum Newton-Raphson iterations (default: 50)
    /// * `tolerance` - Convergence tolerance in volts (default: 1e-4)
    #[wasm_bindgen]
    pub fn with_config(
        circuit_dsl: &str,
        sample_rate: f32,
        max_iterations: usize,
        tolerance: f64,
    ) -> Result<WasmPedalSim, JsValue> {
        // Parse the DSL
        let ast = dsl::parse(circuit_dsl).map_err(|e| JsValue::from_str(&e.to_string()))?;

        // Build the circuit
        let circuit =
            Circuit::from_ast(ast).map_err(|e| JsValue::from_str(&e.to_string()))?;

        // Validate
        crate::circuit::validate_circuit(&circuit)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        // Create simulator with configuration
        let config = SimulatorConfig::new()
            .with_max_iterations(max_iterations)
            .with_tolerance(tolerance);
        let simulator = Simulator::with_config(circuit, sample_rate, config);

        Ok(WasmPedalSim { simulator })
    }

    /// Process a block of audio samples.
    ///
    /// This is the main processing function, designed to be called from
    /// an AudioWorkletProcessor's `process()` method.
    ///
    /// # Arguments
    /// * `input` - Input audio samples (mono, f32)
    /// * `output` - Output buffer to write processed samples into
    ///
    /// # Example (AudioWorklet)
    /// ```javascript
    /// class PedalerProcessor extends AudioWorkletProcessor {
    ///   process(inputs, outputs) {
    ///     const input = inputs[0][0];
    ///     const output = outputs[0][0];
    ///     if (input && output) {
    ///       this.sim.process_block(input, output);
    ///     }
    ///     return true;
    ///   }
    /// }
    /// ```
    #[wasm_bindgen]
    pub fn process_block(&mut self, input: &[f32], output: &mut [f32]) {
        // Process each sample
        let len = input.len().min(output.len());
        for i in 0..len {
            self.simulator.set_input(input[i]);
            output[i] = self.simulator.step().unwrap_or(0.0);
        }
    }

    /// Process a block of audio samples, returning the result.
    ///
    /// Alternative API that returns a new array instead of writing to an output buffer.
    /// Useful for simpler JavaScript usage patterns.
    ///
    /// # Arguments
    /// * `input` - Input audio samples (mono, f32)
    ///
    /// # Returns
    /// A new Float32Array with processed samples.
    #[wasm_bindgen]
    pub fn process_block_alloc(&mut self, input: &[f32]) -> Vec<f32> {
        let mut output = vec![0.0; input.len()];
        self.process_block(input, &mut output);
        output
    }

    /// Get the sample rate this simulator was configured with.
    #[wasm_bindgen(getter)]
    pub fn sample_rate(&self) -> f32 {
        self.simulator.sample_rate()
    }

    /// Get the current voltage at a named node.
    ///
    /// Useful for debugging or visualization.
    ///
    /// # Arguments
    /// * `node_name` - The name of the node in the circuit
    ///
    /// # Returns
    /// The voltage at the node, or `undefined` if the node doesn't exist.
    #[wasm_bindgen]
    pub fn node_voltage(&self, node_name: &str) -> Option<f64> {
        self.simulator.node_voltage(node_name)
    }
}

/// Get the library version.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Get the default sample rate.
#[wasm_bindgen]
pub fn default_sample_rate() -> f32 {
    crate::DEFAULT_SAMPLE_RATE
}
