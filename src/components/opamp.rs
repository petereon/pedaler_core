//! Operational Amplifier model.
//!
//! Supports both ideal op-amp (infinite gain) and finite-gain models.
//! The op-amp enforces V+ = V- for ideal model, or Vout = A * (V+ - V-)
//! for finite gain model.

use crate::circuit::{BranchId, ComponentId, NodeId};
use crate::dsl::ModelDef;

/// Parameters for an op-amp model.
#[derive(Debug, Clone)]
pub struct OpAmpParams {
    /// Open-loop DC gain (A_OL), use f64::INFINITY for ideal
    pub gain: f64,
    /// Output resistance (R_out)
    pub r_out: f64,
    /// Input resistance (R_in)
    pub r_in: f64,
    /// Positive rail voltage
    pub v_rail_pos: f64,
    /// Negative rail voltage
    pub v_rail_neg: f64,
    /// Slew rate (V/µs), 0 = infinite
    pub slew_rate: f64,
}

impl Default for OpAmpParams {
    fn default() -> Self {
        Self::ideal()
    }
}

impl OpAmpParams {
    /// Create parameters for an ideal op-amp.
    pub fn ideal() -> Self {
        Self {
            gain: 1e9, // Very high but finite for numerical stability
            r_out: 0.1,
            r_in: 1e12,
            v_rail_pos: 15.0,
            v_rail_neg: -15.0,
            slew_rate: 0.0,
        }
    }

    /// Create parameters for a typical 741-style op-amp.
    pub fn ua741() -> Self {
        Self {
            gain: 2e5, // 200,000 open-loop gain
            r_out: 75.0,
            r_in: 2e6,
            v_rail_pos: 15.0,
            v_rail_neg: -15.0,
            slew_rate: 0.5,
        }
    }

    /// Create parameters for a TL072-style JFET op-amp.
    pub fn tl072() -> Self {
        Self {
            gain: 2e5,
            r_out: 100.0,
            r_in: 1e12,
            v_rail_pos: 15.0,
            v_rail_neg: -15.0,
            slew_rate: 13.0,
        }
    }

    /// Create parameters from a model definition.
    pub fn from_model(model: &ModelDef) -> Self {
        let mut params = Self::ideal();

        if let Some(&gain) = model.params.get("gain") {
            params.gain = gain;
        }
        if let Some(&a) = model.params.get("a") {
            params.gain = a;
        }
        if let Some(&ro) = model.params.get("rout") {
            params.r_out = ro;
        }
        if let Some(&ri) = model.params.get("rin") {
            params.r_in = ri;
        }
        if let Some(&vp) = model.params.get("vpos") {
            params.v_rail_pos = vp;
        }
        if let Some(&vn) = model.params.get("vneg") {
            params.v_rail_neg = vn;
        }

        params
    }

    /// Check if this is effectively an ideal op-amp.
    pub fn is_ideal(&self) -> bool {
        self.gain > 1e6
    }
}

/// An operational amplifier component.
#[derive(Debug, Clone)]
pub struct OpAmp {
    pub id: ComponentId,
    pub name: String,
    pub nodes: [NodeId; 3], // [output, non-inverting (+), inverting (-)]
    pub params: OpAmpParams,
    pub branch: BranchId,
    /// Current output voltage (for slew rate limiting)
    pub v_out: f64,
}

impl OpAmp {
    /// Create a new op-amp.
    pub fn new(
        id: ComponentId,
        name: String,
        nodes: [NodeId; 3],
        params: OpAmpParams,
        branch: BranchId,
    ) -> Self {
        Self {
            id,
            name,
            nodes,
            params,
            branch,
            v_out: 0.0,
        }
    }

    /// Get the output node.
    pub fn output(&self) -> NodeId {
        self.nodes[0]
    }

    /// Get the non-inverting input node.
    pub fn input_pos(&self) -> NodeId {
        self.nodes[1]
    }

    /// Get the inverting input node.
    pub fn input_neg(&self) -> NodeId {
        self.nodes[2]
    }

    /// Calculate the ideal output voltage (before rail limiting).
    pub fn v_out_ideal(&self, v_pos: f64, v_neg: f64) -> f64 {
        let v_diff = v_pos - v_neg;
        self.params.gain * v_diff
    }

    /// Calculate the actual output voltage with rail limiting.
    pub fn v_out_limited(&self, v_pos: f64, v_neg: f64) -> f64 {
        let v_ideal = self.v_out_ideal(v_pos, v_neg);
        v_ideal
            .max(self.params.v_rail_neg + 0.5)
            .min(self.params.v_rail_pos - 0.5)
    }

    /// Calculate the actual output voltage with slew rate limiting.
    pub fn v_out_slew_limited(&mut self, v_pos: f64, v_neg: f64, dt: f64) -> f64 {
        let v_target = self.v_out_limited(v_pos, v_neg);

        if self.params.slew_rate > 0.0 {
            let max_change = self.params.slew_rate * 1e6 * dt; // Convert V/µs to V/s
            let change = (v_target - self.v_out).clamp(-max_change, max_change);
            self.v_out += change;
        } else {
            self.v_out = v_target;
        }

        self.v_out
    }

    /// Get the effective transconductance for MNA stamping.
    /// For high gain, we model as: I_out = gm * (V+ - V-)
    pub fn transconductance(&self) -> f64 {
        self.params.gain / self.params.r_out
    }

    /// Get the input conductance.
    pub fn input_conductance(&self) -> f64 {
        1.0 / self.params.r_in
    }

    /// Get the output conductance.
    pub fn output_conductance(&self) -> f64 {
        1.0 / self.params.r_out
    }
}
