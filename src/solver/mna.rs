//! MNA matrix assembly and solving.

use crate::circuit::{Circuit, NodeId};
use crate::components::Component;
use crate::error::Result;

/// MNA matrix system Ax = z.
#[derive(Debug)]
pub struct MnaMatrix {
    /// System matrix A (row-major)
    pub a: Vec<f64>,
    /// Source vector z
    pub z: Vec<f64>,
    /// Solution vector x
    pub x: Vec<f64>,
    /// Matrix dimension
    pub size: usize,
    /// LU decomposition of A (for efficient solving)
    pub lu: Vec<f64>,
    /// Pivot indices for LU decomposition
    pub pivots: Vec<usize>,
}

impl MnaMatrix {
    /// Create a new MNA matrix for the given circuit.
    pub fn new(size: usize) -> Self {
        Self {
            a: vec![0.0; size * size],
            z: vec![0.0; size],
            x: vec![0.0; size],
            size,
            lu: vec![0.0; size * size],
            pivots: vec![0; size],
        }
    }

    /// Clear the matrix and vectors to zero.
    pub fn clear(&mut self) {
        self.a.fill(0.0);
        self.z.fill(0.0);
    }

    /// Get matrix element at (row, col).
    pub fn get(&self, row: usize, col: usize) -> f64 {
        self.a[row * self.size + col]
    }

    /// Set matrix element at (row, col).
    pub fn set(&mut self, row: usize, col: usize, value: f64) {
        self.a[row * self.size + col] = value;
    }

    /// Add to matrix element at (row, col).
    pub fn add(&mut self, row: usize, col: usize, value: f64) {
        self.a[row * self.size + col] += value;
    }

    /// Add to source vector element.
    pub fn add_source(&mut self, row: usize, value: f64) {
        self.z[row] += value;
    }

    /// Stamp a conductance between two nodes.
    /// For a conductance G between nodes n1 and n2:
    ///   A[n1,n1] += G
    ///   A[n2,n2] += G
    ///   A[n1,n2] -= G
    ///   A[n2,n1] -= G
    pub fn stamp_conductance(&mut self, n1: Option<usize>, n2: Option<usize>, g: f64) {
        if let Some(i) = n1 {
            self.add(i, i, g);
        }
        if let Some(j) = n2 {
            self.add(j, j, g);
        }
        if let (Some(i), Some(j)) = (n1, n2) {
            self.add(i, j, -g);
            self.add(j, i, -g);
        }
    }

    /// Stamp a voltage source between two nodes with branch current at index br.
    /// V[n+] - V[n-] = E
    pub fn stamp_voltage_source(
        &mut self,
        n_pos: Option<usize>,
        n_neg: Option<usize>,
        br: usize,
        voltage: f64,
    ) {
        // KVL equation: V[n+] - V[n-] = E
        if let Some(i) = n_pos {
            self.add(br, i, 1.0);
            self.add(i, br, 1.0);
        }
        if let Some(j) = n_neg {
            self.add(br, j, -1.0);
            self.add(j, br, -1.0);
        }
        self.z[br] = voltage;
    }

    /// Stamp a current source between two nodes.
    /// Current flows from n+ to n-.
    pub fn stamp_current_source(&mut self, n_pos: Option<usize>, n_neg: Option<usize>, current: f64) {
        // Current enters n- and leaves n+
        if let Some(i) = n_pos {
            self.add_source(i, -current);
        }
        if let Some(j) = n_neg {
            self.add_source(j, current);
        }
    }

    /// Stamp a VCVS (Voltage-Controlled Voltage Source).
    /// V[out] = A * (V[ctrl+] - V[ctrl-])
    pub fn stamp_vcvs(
        &mut self,
        n_out_pos: Option<usize>,
        n_out_neg: Option<usize>,
        n_ctrl_pos: Option<usize>,
        n_ctrl_neg: Option<usize>,
        br: usize,
        gain: f64,
    ) {
        // Output voltage constraint
        if let Some(i) = n_out_pos {
            self.add(br, i, 1.0);
            self.add(i, br, 1.0);
        }
        if let Some(j) = n_out_neg {
            self.add(br, j, -1.0);
            self.add(j, br, -1.0);
        }

        // Control voltage contribution
        if let Some(i) = n_ctrl_pos {
            self.add(br, i, -gain);
        }
        if let Some(j) = n_ctrl_neg {
            self.add(br, j, gain);
        }
    }

    /// Stamp a VCCS (Voltage-Controlled Current Source).
    /// I = gm * (V[ctrl+] - V[ctrl-])
    pub fn stamp_vccs(
        &mut self,
        n_out_pos: Option<usize>,
        n_out_neg: Option<usize>,
        n_ctrl_pos: Option<usize>,
        n_ctrl_neg: Option<usize>,
        gm: f64,
    ) {
        // Current flows from out+ to out-
        if let (Some(i), Some(k)) = (n_out_pos, n_ctrl_pos) {
            self.add(i, k, gm);
        }
        if let (Some(i), Some(l)) = (n_out_pos, n_ctrl_neg) {
            self.add(i, l, -gm);
        }
        if let (Some(j), Some(k)) = (n_out_neg, n_ctrl_pos) {
            self.add(j, k, -gm);
        }
        if let (Some(j), Some(l)) = (n_out_neg, n_ctrl_neg) {
            self.add(j, l, gm);
        }
    }

    /// Perform LU decomposition with partial pivoting.
    pub fn factor(&mut self) -> Result<()> {
        let n = self.size;
        self.lu.copy_from_slice(&self.a);

        for i in 0..n {
            self.pivots[i] = i;
        }

        for k in 0..n {
            // Find pivot
            let mut max_val = self.lu[k * n + k].abs();
            let mut max_row = k;

            for i in (k + 1)..n {
                let val = self.lu[i * n + k].abs();
                if val > max_val {
                    max_val = val;
                    max_row = i;
                }
            }

            if max_val < 1e-15 {
                return Err(crate::error::PedalerError::SingularMatrix);
            }

            // Swap rows if needed
            if max_row != k {
                self.pivots.swap(k, max_row);
                for j in 0..n {
                    let tmp = self.lu[k * n + j];
                    self.lu[k * n + j] = self.lu[max_row * n + j];
                    self.lu[max_row * n + j] = tmp;
                }
            }

            // Eliminate
            let pivot = self.lu[k * n + k];
            for i in (k + 1)..n {
                let factor = self.lu[i * n + k] / pivot;
                self.lu[i * n + k] = factor;
                for j in (k + 1)..n {
                    self.lu[i * n + j] -= factor * self.lu[k * n + j];
                }
            }
        }

        Ok(())
    }

    /// Solve the system using the pre-computed LU decomposition.
    pub fn solve(&mut self) -> Result<()> {
        let n = self.size;

        // Apply pivot permutation to z
        let b = self.z.clone();
        for i in 0..n {
            self.x[i] = b[self.pivots[i]];
        }

        // Forward substitution (L * y = Pb)
        for i in 0..n {
            for j in 0..i {
                self.x[i] -= self.lu[i * n + j] * self.x[j];
            }
        }

        // Back substitution (U * x = y)
        for i in (0..n).rev() {
            for j in (i + 1)..n {
                self.x[i] -= self.lu[i * n + j] * self.x[j];
            }
            let diag = self.lu[i * n + i];
            if diag.abs() < 1e-15 {
                return Err(crate::error::PedalerError::SingularMatrix);
            }
            self.x[i] /= diag;
        }

        Ok(())
    }

    /// Get the voltage at a node.
    pub fn voltage(&self, node: Option<usize>) -> f64 {
        match node {
            Some(i) => self.x[i],
            None => 0.0, // Ground
        }
    }

    /// Get the voltage at a NodeId (handling ground).
    pub fn node_voltage(&self, circuit: &Circuit, node: NodeId) -> f64 {
        self.voltage(circuit.node_index(node))
    }
}

/// Stamp all linear components into the MNA matrix.
pub fn stamp_linear_components(circuit: &Circuit, matrix: &mut MnaMatrix, dt: f64) {
    for component in &circuit.components {
        match component {
            Component::Resistor(r) => {
                let n1 = circuit.node_index(r.nodes[0]);
                let n2 = circuit.node_index(r.nodes[1]);
                matrix.stamp_conductance(n1, n2, r.conductance());
            }

            Component::Capacitor(c) => {
                let n1 = circuit.node_index(c.nodes[0]);
                let n2 = circuit.node_index(c.nodes[1]);
                let g = c.conductance(dt);
                matrix.stamp_conductance(n1, n2, g);
                // Companion current source
                let i_eq = c.current_source(dt);
                matrix.stamp_current_source(n1, n2, i_eq);
            }

            Component::Inductor(l) => {
                let n1 = circuit.node_index(l.nodes[0]);
                let n2 = circuit.node_index(l.nodes[1]);
                let br = circuit.branch_index(l.branch);
                let r_eq = l.resistance(dt);
                let v_eq = l.voltage_source(dt);

                // Stamp as voltage source with series resistance
                matrix.stamp_voltage_source(n1, n2, br, v_eq);
                matrix.add(br, br, -r_eq);
            }

            Component::VoltageSource(v) => {
                let n1 = circuit.node_index(v.nodes[0]);
                let n2 = circuit.node_index(v.nodes[1]);
                let br = circuit.branch_index(v.branch);
                matrix.stamp_voltage_source(n1, n2, br, v.voltage());
            }

            Component::CurrentSource(i) => {
                let n1 = circuit.node_index(i.nodes[0]);
                let n2 = circuit.node_index(i.nodes[1]);
                matrix.stamp_current_source(n1, n2, i.current());
            }

            Component::OpAmp(op) => {
                let n_out = circuit.node_index(op.output());
                let n_pos = circuit.node_index(op.input_pos());
                let n_neg = circuit.node_index(op.input_neg());

                // Model op-amp as VCCS + output resistance
                // This is more numerically stable than VCVS for high gains
                //
                // The op-amp is modeled as:
                //   - A voltage-controlled current source: I = gm * (V+ - V-)
                //   - Output resistance Rout to ground
                //
                // At DC: Vout = I * Rout = gm * Rout * (V+ - V-) = A * (V+ - V-)
                // where A = gm * Rout, so gm = A / Rout

                let gm = op.transconductance(); // = gain / r_out
                let g_out = op.output_conductance(); // = 1 / r_out

                // Stamp VCCS: current flows from output to ground
                matrix.stamp_vccs(n_out, None, n_pos, n_neg, gm);

                // Stamp output resistance to ground
                if let Some(out) = n_out {
                    matrix.add(out, out, g_out);
                }

                // Stamp input resistance (between V+ and V-)
                // This prevents floating inputs
                let g_in = op.input_conductance();
                matrix.stamp_conductance(n_pos, n_neg, g_in);
            }

            Component::Potentiometer(p) => {
                let n1 = circuit.node_index(p.n1());
                let nw = circuit.node_index(p.wiper());
                let n2 = circuit.node_index(p.n2());

                // Two resistors: n1-wiper and wiper-n2
                matrix.stamp_conductance(n1, nw, p.g1());
                matrix.stamp_conductance(nw, n2, p.g2());
            }

            Component::Switch(s) => {
                let n1 = circuit.node_index(s.nodes[0]);
                let n2 = circuit.node_index(s.nodes[1]);
                matrix.stamp_conductance(n1, n2, s.conductance());
            }

            // Nonlinear components handled separately
            Component::Diode(_) | Component::Bjt(_) => {}
        }
    }
}
