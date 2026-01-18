//! BJT (Bipolar Junction Transistor) model.
//!
//! Uses a simplified Ebers-Moll model for NPN and PNP transistors.
//! The BJT is modeled as two diodes with a current-controlled current source.

use crate::circuit::{ComponentId, NodeId};
use crate::dsl::{ModelDef, ModelType};
use crate::error::{PedalerError, Result};
use crate::THERMAL_VOLTAGE;

/// BJT type (NPN or PNP).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BjtType {
    Npn,
    Pnp,
}

/// Parameters for a BJT model.
#[derive(Debug, Clone)]
pub struct BjtParams {
    /// Forward current gain (β_F)
    pub beta_f: f64,
    /// Reverse current gain (β_R)
    pub beta_r: f64,
    /// Base-emitter saturation current
    pub is_be: f64,
    /// Base-collector saturation current
    pub is_bc: f64,
    /// Ideality factor
    pub n: f64,
    /// Early voltage (for output resistance), 0 = infinite
    pub va: f64,
}

impl Default for BjtParams {
    fn default() -> Self {
        Self {
            beta_f: 100.0,
            beta_r: 1.0,
            is_be: 1e-14,
            is_bc: 1e-14,
            n: 1.0,
            va: 100.0,
        }
    }
}

impl BjtParams {
    /// Create parameters from a model definition.
    pub fn from_model(model: &ModelDef) -> Result<(BjtType, Self)> {
        let bjt_type = match model.model_type {
            ModelType::BjtNpn => BjtType::Npn,
            ModelType::BjtPnp => BjtType::Pnp,
            _ => {
                return Err(PedalerError::InvalidParameter {
                    component: model.name.clone(),
                    param: "type".to_string(),
                    message: "expected NPN or PNP model type".to_string(),
                });
            }
        };

        let mut params = Self::default();

        if let Some(&bf) = model.params.get("bf") {
            params.beta_f = bf;
        }
        if let Some(&br) = model.params.get("br") {
            params.beta_r = br;
        }
        if let Some(&is) = model.params.get("is") {
            params.is_be = is;
            params.is_bc = is;
        }
        if let Some(&n) = model.params.get("n") {
            params.n = n;
        }
        if let Some(&va) = model.params.get("va") {
            params.va = va;
        }

        Ok((bjt_type, params))
    }

    /// Thermal voltage times ideality factor.
    pub fn n_vt(&self) -> f64 {
        self.n * THERMAL_VOLTAGE
    }
}

/// A BJT component.
#[derive(Debug, Clone)]
pub struct Bjt {
    pub id: ComponentId,
    pub name: String,
    pub nodes: [NodeId; 3], // [collector, base, emitter]
    pub bjt_type: BjtType,
    pub params: BjtParams,
    /// Current base-emitter voltage operating point
    pub v_be_op: f64,
    /// Current base-collector voltage operating point
    pub v_bc_op: f64,
}

impl Bjt {
    /// Create a new BJT.
    pub fn new(
        id: ComponentId,
        name: String,
        nodes: [NodeId; 3],
        bjt_type: BjtType,
        params: BjtParams,
    ) -> Self {
        Self {
            id,
            name,
            nodes,
            bjt_type,
            params,
            v_be_op: 0.0,
            v_bc_op: 0.0,
        }
    }

    /// Get the collector node.
    pub fn collector(&self) -> NodeId {
        self.nodes[0]
    }

    /// Get the base node.
    pub fn base(&self) -> NodeId {
        self.nodes[1]
    }

    /// Get the emitter node.
    pub fn emitter(&self) -> NodeId {
        self.nodes[2]
    }

    /// Calculate the base-emitter diode current.
    pub fn i_be(&self, v_be: f64) -> f64 {
        let n_vt = self.params.n_vt();
        let v = match self.bjt_type {
            BjtType::Npn => v_be,
            BjtType::Pnp => -v_be,
        };

        if v > 0.8 {
            // Linear extrapolation to prevent overflow
            let v_crit = 0.8;
            let i_crit = self.params.is_be * ((v_crit / n_vt).exp() - 1.0);
            let g_crit = self.params.is_be / n_vt * (v_crit / n_vt).exp();
            i_crit + g_crit * (v - v_crit)
        } else {
            self.params.is_be * ((v / n_vt).exp() - 1.0)
        }
    }

    /// Calculate the base-collector diode current.
    pub fn i_bc(&self, v_bc: f64) -> f64 {
        let n_vt = self.params.n_vt();
        let v = match self.bjt_type {
            BjtType::Npn => v_bc,
            BjtType::Pnp => -v_bc,
        };

        if v > 0.8 {
            let v_crit = 0.8;
            let i_crit = self.params.is_bc * ((v_crit / n_vt).exp() - 1.0);
            let g_crit = self.params.is_bc / n_vt * (v_crit / n_vt).exp();
            i_crit + g_crit * (v - v_crit)
        } else {
            self.params.is_bc * ((v / n_vt).exp() - 1.0)
        }
    }

    /// Calculate the collector current (Ic).
    pub fn i_c(&self, v_be: f64, v_bc: f64) -> f64 {
        let i_f = self.i_be(v_be);
        let i_r = self.i_bc(v_bc);
        let sign = match self.bjt_type {
            BjtType::Npn => 1.0,
            BjtType::Pnp => -1.0,
        };
        sign * (self.params.beta_f * i_f / (self.params.beta_f + 1.0)
            - i_r * (self.params.beta_r + 1.0) / self.params.beta_r)
    }

    /// Calculate the base current (Ib).
    pub fn i_b(&self, v_be: f64, v_bc: f64) -> f64 {
        let i_f = self.i_be(v_be);
        let i_r = self.i_bc(v_bc);
        let sign = match self.bjt_type {
            BjtType::Npn => 1.0,
            BjtType::Pnp => -1.0,
        };
        sign * (i_f / (self.params.beta_f + 1.0) + i_r / (self.params.beta_r + 1.0))
    }

    /// Calculate the emitter current (Ie).
    pub fn i_e(&self, v_be: f64, v_bc: f64) -> f64 {
        // Ie = Ic + Ib
        self.i_c(v_be, v_bc) + self.i_b(v_be, v_bc)
    }

    /// Get partial derivatives for linearization.
    /// Returns (gm, go, gpi, gmu) - transconductance, output conductance,
    /// input conductance, feedback conductance.
    pub fn linearize(&self, v_be: f64, v_bc: f64) -> (f64, f64, f64, f64) {
        let n_vt = self.params.n_vt();

        // dI_be/dV_be
        let v_be_eff = match self.bjt_type {
            BjtType::Npn => v_be,
            BjtType::Pnp => -v_be,
        };
        let g_be = if v_be_eff > 0.0 {
            (self.params.is_be / n_vt * (v_be_eff / n_vt).exp()).min(1.0)
        } else {
            1e-12
        };

        // dI_bc/dV_bc
        let v_bc_eff = match self.bjt_type {
            BjtType::Npn => v_bc,
            BjtType::Pnp => -v_bc,
        };
        let g_bc = if v_bc_eff > 0.0 {
            (self.params.is_bc / n_vt * (v_bc_eff / n_vt).exp()).min(1.0)
        } else {
            1e-12
        };

        // Transconductance gm = dIc/dVbe
        let gm = self.params.beta_f * g_be / (self.params.beta_f + 1.0);

        // Output conductance (Early effect)
        let go = if self.params.va > 0.0 {
            self.i_c(v_be, v_bc).abs() / self.params.va
        } else {
            1e-12
        };

        // Input conductance gpi = dIb/dVbe
        let gpi = g_be / (self.params.beta_f + 1.0);

        // Feedback conductance gmu = dIb/dVbc
        let gmu = g_bc / (self.params.beta_r + 1.0);

        (gm.max(1e-12), go.max(1e-12), gpi.max(1e-12), gmu.max(1e-12))
    }

    /// Update operating points.
    pub fn update_operating_point(&mut self, v_be: f64, v_bc: f64) {
        self.v_be_op = v_be;
        self.v_bc_op = v_bc;
    }
}
