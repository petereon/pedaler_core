//! Newton-Raphson iteration for nonlinear components.

use crate::circuit::Circuit;
use crate::components::Component;
use crate::error::{PedalerError, Result};
use super::mna::MnaMatrix;
use super::{CONVERGENCE_TOLERANCE, MAX_ITERATIONS};

/// Newton-Raphson solver for nonlinear circuits.
pub struct NewtonRaphson {
    /// Maximum iterations
    pub max_iterations: usize,
    /// Convergence tolerance
    pub tolerance: f64,
    /// Previous solution for convergence check
    x_prev: Vec<f64>,
}

impl Default for NewtonRaphson {
    fn default() -> Self {
        Self::new()
    }
}

impl NewtonRaphson {
    /// Create a new Newton-Raphson solver.
    pub fn new() -> Self {
        Self {
            max_iterations: MAX_ITERATIONS,
            tolerance: CONVERGENCE_TOLERANCE,
            x_prev: Vec::new(),
        }
    }

    /// Solve the nonlinear circuit using Newton-Raphson iteration.
    ///
    /// Returns the number of iterations used.
    pub fn solve(&mut self, circuit: &Circuit, matrix: &mut MnaMatrix, dt: f64) -> Result<usize> {
        // Check if there are any nonlinear components
        let has_nonlinear = circuit.components.iter().any(|c| c.is_nonlinear());

        if !has_nonlinear {
            // Purely linear circuit - solve directly
            matrix.factor()?;
            matrix.solve()?;
            return Ok(1);
        }

        // Initialize x_prev
        if self.x_prev.len() != matrix.size {
            self.x_prev = vec![0.0; matrix.size];
        }

        // Use previous solution as initial guess
        self.x_prev.copy_from_slice(&matrix.x);

        for iter in 0..self.max_iterations {
            // Clear and rebuild matrix
            matrix.clear();

            // Stamp linear components
            super::mna::stamp_linear_components(circuit, matrix, dt);

            // Stamp linearized nonlinear components
            self.stamp_nonlinear_components(circuit, matrix)?;

            // Solve the linear system
            matrix.factor()?;
            matrix.solve()?;

            // Check convergence
            let mut max_diff = 0.0f64;
            for i in 0..matrix.size {
                let diff = (matrix.x[i] - self.x_prev[i]).abs();
                max_diff = max_diff.max(diff);
            }

            if max_diff < self.tolerance {
                // Update operating points for next time step
                self.update_operating_points(circuit, matrix);
                return Ok(iter + 1);
            }

            // Save current solution for next iteration
            self.x_prev.copy_from_slice(&matrix.x);
        }

        Err(PedalerError::convergence_failure(
            self.max_iterations,
            self.residual(matrix),
        ))
    }

    /// Stamp linearized nonlinear components into the matrix.
    fn stamp_nonlinear_components(&self, circuit: &Circuit, matrix: &mut MnaMatrix) -> Result<()> {
        for component in &circuit.components {
            match component {
                Component::Diode(d) => {
                    let n_anode = circuit.node_index(d.nodes[0]);
                    let n_cathode = circuit.node_index(d.nodes[1]);

                    // Get voltage across diode from previous iteration
                    let v_a = matrix.voltage(n_anode);
                    let v_c = matrix.voltage(n_cathode);
                    let v_d = v_a - v_c;

                    // Limit voltage step
                    let v_op = d.limit_voltage_step(d.v_op, v_d);

                    // Get linearized model
                    let (g, i_eq) = d.linearize(v_op);

                    // Stamp as conductance + current source
                    matrix.stamp_conductance(n_anode, n_cathode, g);
                    matrix.stamp_current_source(n_anode, n_cathode, i_eq);
                }

                Component::Bjt(q) => {
                    let n_c = circuit.node_index(q.collector());
                    let n_b = circuit.node_index(q.base());
                    let n_e = circuit.node_index(q.emitter());

                    // Get voltages
                    let v_c = matrix.voltage(n_c);
                    let v_b = matrix.voltage(n_b);
                    let v_e = matrix.voltage(n_e);
                    let v_be = v_b - v_e;
                    let v_bc = v_b - v_c;

                    // Get linearized small-signal parameters
                    let (gm, go, gpi, gmu) = q.linearize(v_be, v_bc);

                    // Stamp input conductance (B-E)
                    matrix.stamp_conductance(n_b, n_e, gpi);

                    // Stamp feedback conductance (B-C)
                    matrix.stamp_conductance(n_b, n_c, gmu);

                    // Stamp output conductance (C-E)
                    matrix.stamp_conductance(n_c, n_e, go);

                    // Stamp transconductance (VCCS)
                    matrix.stamp_vccs(n_c, n_e, n_b, n_e, gm);

                    // DC operating point currents
                    let i_c = q.i_c(v_be, v_bc);
                    let i_b = q.i_b(v_be, v_bc);

                    // Companion current sources to match DC point
                    let i_c_eq = i_c - gm * v_be - go * (v_c - v_e);
                    let i_b_eq = i_b - gpi * v_be - gmu * v_bc;

                    matrix.stamp_current_source(n_c, n_e, i_c_eq);
                    matrix.stamp_current_source(n_b, None, -i_b_eq);
                }

                _ => {} // Linear components already handled
            }
        }

        Ok(())
    }

    /// Update operating points after successful convergence.
    fn update_operating_points(&self, circuit: &Circuit, matrix: &MnaMatrix) {
        // Note: We can't mutate circuit components here since we only have &Circuit
        // In practice, the operating points would be stored in a separate state struct
        // or the circuit would need to use interior mutability (RefCell)
        let _ = circuit;
        let _ = matrix;
    }

    /// Calculate the residual for error reporting.
    fn residual(&self, matrix: &MnaMatrix) -> f64 {
        let mut max_diff = 0.0f64;
        for i in 0..matrix.size {
            let diff = (matrix.x[i] - self.x_prev[i]).abs();
            max_diff = max_diff.max(diff);
        }
        max_diff
    }
}
