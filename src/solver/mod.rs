//! MNA (Modified Nodal Analysis) solver.
//!
//! This module provides the numerical engine for circuit simulation.
//!
//! ## Modified Nodal Analysis
//!
//! MNA assembles a system of equations Ax = z where:
//! - x contains node voltages and branch currents
//! - A is the conductance/coefficient matrix
//! - z is the source vector
//!
//! The matrix structure is:
//! ```text
//! [ G   B ] [ v ]   [ i ]
//! [ C   D ] [ j ] = [ e ]
//! ```
//!
//! where:
//! - G is the conductance matrix (node equations)
//! - B, C connect voltage sources to nodes
//! - D is usually 0 (for ideal voltage sources)
//! - v is the vector of node voltages
//! - j is the vector of voltage source currents
//! - i is the sum of current sources into each node
//! - e is the vector of voltage source values

mod mna;
mod newton;
mod simulator;

pub use mna::MnaMatrix;
pub use newton::NewtonRaphson;
pub use simulator::Simulator;

/// Convergence tolerance for Newton-Raphson iteration.
pub const CONVERGENCE_TOLERANCE: f64 = 1e-6;

/// Maximum Newton-Raphson iterations per time step.
pub const MAX_ITERATIONS: usize = 50;

/// Minimum conductance to prevent singular matrix.
pub const MIN_CONDUCTANCE: f64 = 1e-12;
