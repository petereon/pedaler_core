//! # Pedaler Core
//!
//! A real-time circuit simulator for guitar pedals.
//!
//! This library provides:
//! - A custom DSL for describing circuit topologies
//! - Modified Nodal Analysis (MNA) based circuit simulation
//! - Support for linear components (R, C, L) and nonlinear components (diodes, BJTs, op-amps)
//! - Audio processing pipeline for real-time effect simulation
//!
//! ## Architecture
//!
//! The library is organized into several modules:
//!
//! - [`dsl`] - Parser for the circuit description language
//! - [`circuit`] - Circuit graph representation and validation
//! - [`components`] - Component models (resistors, capacitors, diodes, etc.)
//! - [`solver`] - MNA matrix assembly and numerical solving
//! - [`audio`] - Audio I/O and processing (CLI only)
//!
//! ## Usage
//!
//! ### Native CLI
//!
//! ```bash
//! ffmpeg -i input.wav -f f32le -ac 1 -ar 48000 - | pedaler circuit.ped | ffmpeg -f f32le -ac 1 -ar 48000 -i - output.wav
//! ```
//!
//! ### WASM
//!
//! ```javascript
//! import { WasmPedalSim } from 'pedaler_core';
//!
//! const sim = new WasmPedalSim(circuitDsl, 48000);
//! sim.process_block(inputBuffer, outputBuffer);
//! ```
//!
//! ## Circuit Simulation Method
//!
//! The simulator uses Modified Nodal Analysis (MNA) to solve circuit equations.
//! For each time step dt = 1/sample_rate:
//!
//! 1. Assemble the system matrix A and source vector z
//! 2. Solve Ax = z for node voltages and branch currents
//! 3. For nonlinear elements, iterate using Newton-Raphson until convergence
//!
//! Reactive elements (C, L) are discretized using the trapezoidal rule for
//! accuracy and stability.

pub mod circuit;
pub mod components;
pub mod dsl;
pub mod error;
pub mod solver;

#[cfg(feature = "cli")]
pub mod audio;

// Re-export main types for convenience
pub use circuit::Circuit;
pub use error::{PedalerError, Result};
pub use solver::Simulator;

// WASM bindings
#[cfg(feature = "wasm")]
mod wasm;

#[cfg(feature = "wasm")]
pub use wasm::WasmPedalSim;

/// Default sample rate in Hz
pub const DEFAULT_SAMPLE_RATE: f32 = 48000.0;

/// Thermal voltage at room temperature (approximately 26mV)
pub const THERMAL_VOLTAGE: f64 = 0.0258;
