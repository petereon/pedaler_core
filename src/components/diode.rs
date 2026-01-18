//! Diode model.
//!
//! Uses the Shockley diode equation:
//!   I = Is * (exp(V / (n * Vt)) - 1)
//!
//! For Newton-Raphson iteration, we linearize around the current operating point:
//!   I ≈ I0 + G_d * (V - V0)
//!
//! where G_d = dI/dV = Is/(n*Vt) * exp(V0/(n*Vt))

use crate::circuit::{ComponentId, NodeId};
use crate::dsl::ModelDef;
use crate::THERMAL_VOLTAGE;

/// Parameters for a diode model.
#[derive(Debug, Clone)]
pub struct DiodeParams {
    /// Saturation current (Is), typically 1e-14 to 1e-12 A
    pub is: f64,
    /// Ideality factor (n), typically 1.0 to 2.0
    pub n: f64,
    /// Forward voltage drop (for simple model), typically 0.6-0.7V for silicon
    pub vf: f64,
    /// Maximum voltage for exp() calculation to prevent overflow
    pub v_crit: f64,
}

impl Default for DiodeParams {
    fn default() -> Self {
        Self {
            is: 1e-14,
            n: 1.0,
            vf: 0.7,
            v_crit: 0.7, // About 40 * Vt
        }
    }
}

impl DiodeParams {
    /// Create parameters for a germanium diode (lower forward voltage).
    pub fn germanium() -> Self {
        Self {
            is: 1e-9,
            n: 1.5,
            vf: 0.3,
            v_crit: 0.5,
        }
    }

    /// Create parameters for an LED.
    pub fn led(color_vf: f64) -> Self {
        Self {
            is: 1e-18,
            n: 2.0,
            vf: color_vf, // Red ~1.8V, Green ~2.2V, Blue ~3.3V
            v_crit: color_vf,
        }
    }

    /// Create parameters from a model definition.
    pub fn from_model(model: &ModelDef) -> Self {
        let mut params = Self::default();
        if let Some(&is) = model.params.get("is") {
            params.is = is;
        }
        if let Some(&n) = model.params.get("n") {
            params.n = n;
        }
        if let Some(&vf) = model.params.get("vf") {
            params.vf = vf;
            params.v_crit = vf;
        }
        params
    }

    /// Thermal voltage times ideality factor.
    pub fn n_vt(&self) -> f64 {
        self.n * THERMAL_VOLTAGE
    }
}

/// A diode component.
#[derive(Debug, Clone)]
pub struct Diode {
    pub id: ComponentId,
    pub name: String,
    pub nodes: [NodeId; 2], // [anode, cathode]
    pub params: DiodeParams,
    /// Current operating point voltage (for Newton-Raphson)
    pub v_op: f64,
}

impl Diode {
    /// Create a new diode.
    pub fn new(id: ComponentId, name: String, nodes: [NodeId; 2], params: DiodeParams) -> Self {
        Self {
            id,
            name,
            nodes,
            params,
            v_op: 0.0,
        }
    }

    /// Calculate the diode current at a given voltage.
    pub fn current(&self, v: f64) -> f64 {
        let n_vt = self.params.n_vt();

        // Limit voltage to prevent overflow
        let v_limited = v.min(self.params.v_crit * 2.0);

        if v_limited > self.params.v_crit {
            // Linear extrapolation for high forward bias
            let i_crit = self.params.is * ((self.params.v_crit / n_vt).exp() - 1.0);
            let g_crit = self.params.is / n_vt * (self.params.v_crit / n_vt).exp();
            i_crit + g_crit * (v_limited - self.params.v_crit)
        } else if v_limited < -5.0 * n_vt {
            // Deep reverse bias - just use saturation current
            -self.params.is
        } else {
            // Normal Shockley equation
            self.params.is * ((v_limited / n_vt).exp() - 1.0)
        }
    }

    /// Calculate the conductance (dI/dV) at a given voltage.
    pub fn conductance(&self, v: f64) -> f64 {
        let n_vt = self.params.n_vt();
        let v_limited = v.min(self.params.v_crit * 2.0);

        if v_limited > self.params.v_crit {
            // Conductance at critical point
            self.params.is / n_vt * (self.params.v_crit / n_vt).exp()
        } else if v_limited < -5.0 * n_vt {
            // Very small conductance in deep reverse bias
            1e-12
        } else {
            // dI/dV = Is/(n*Vt) * exp(V/(n*Vt))
            self.params.is / n_vt * (v_limited / n_vt).exp()
        }
    }

    /// Get the linearized model parameters at the current operating point.
    /// Returns (conductance G, equivalent current source I_eq)
    /// such that I = G * V + I_eq
    pub fn linearize(&self, v_op: f64) -> (f64, f64) {
        let g = self.conductance(v_op);
        let i = self.current(v_op);
        // I = G * V + I_eq => I_eq = I - G * V
        let i_eq = i - g * v_op;
        (g.max(1e-12), i_eq)
    }

    /// Update the operating point.
    pub fn update_operating_point(&mut self, v: f64) {
        self.v_op = v;
    }

    /// Limit voltage step for Newton-Raphson convergence.
    /// Uses a larger step limit to allow faster convergence while still
    /// preventing numerical overflow in the exponential.
    pub fn limit_voltage_step(&self, v_old: f64, v_new: f64) -> f64 {
        // Allow larger steps (up to critical voltage) for faster convergence
        // but prevent huge jumps that could cause numerical issues
        let max_step = self.params.v_crit.max(0.5);  // At least 0.5V steps allowed

        if (v_new - v_old).abs() > max_step {
            if v_new > v_old {
                v_old + max_step
            } else {
                v_old - max_step
            }
        } else {
            v_new
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diode_forward_bias() {
        let d = Diode::new(
            ComponentId(0),
            "D1".to_string(),
            [NodeId(1), NodeId(0)],
            DiodeParams::default(),
        );

        // At 0V, current should be approximately 0
        assert!(d.current(0.0).abs() < 1e-10);

        // At forward bias, current should increase exponentially
        let i_small = d.current(0.3);
        let i_large = d.current(0.6);
        assert!(i_large > i_small * 100.0);
    }

    #[test]
    fn test_diode_reverse_bias() {
        let d = Diode::new(
            ComponentId(0),
            "D1".to_string(),
            [NodeId(1), NodeId(0)],
            DiodeParams::default(),
        );

        // In reverse bias, current should approach -Is
        let i_rev = d.current(-1.0);
        assert!(i_rev < 0.0);
        assert!(i_rev > -2.0 * d.params.is);
    }
}
