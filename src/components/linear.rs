//! Linear passive components: Resistor, Capacitor, Inductor.

use crate::circuit::{BranchId, ComponentId, NodeId};

/// Modulation configuration for a resistor.
#[derive(Debug, Clone)]
pub struct ResistorModulation {
    /// Name of the LFO to use for modulation
    pub lfo_name: String,
    /// Modulation depth (0.0 to 1.0) - how much the resistance varies
    /// At depth=1.0, resistance varies from R_base to R_base * (1 + range)
    pub depth: f64,
    /// Modulation range - multiplier for how far resistance can vary
    /// Default is 4.0, meaning resistance can go up to 5x base value
    pub range: f64,
}

/// A resistor component with optional modulation.
#[derive(Debug, Clone)]
pub struct Resistor {
    pub id: ComponentId,
    pub name: String,
    pub nodes: [NodeId; 2], // [positive, negative]
    /// Base resistance value (when modulation = 0)
    pub resistance: f64,
    /// Optional modulation configuration
    pub modulation: Option<ResistorModulation>,
    /// Current effective resistance (updated each sample for modulated resistors)
    pub effective_resistance: f64,
}

impl Resistor {
    /// Create a new resistor.
    pub fn new(id: ComponentId, name: String, nodes: [NodeId; 2], resistance: f64) -> Self {
        let r = resistance.max(1e-12); // Minimum resistance to avoid singularity
        Self {
            id,
            name,
            nodes,
            resistance: r,
            modulation: None,
            effective_resistance: r,
        }
    }

    /// Create a new modulated resistor.
    pub fn new_modulated(
        id: ComponentId,
        name: String,
        nodes: [NodeId; 2],
        resistance: f64,
        lfo_name: String,
        depth: f64,
        range: f64,
    ) -> Self {
        let r = resistance.max(1e-12);
        Self {
            id,
            name,
            nodes,
            resistance: r,
            modulation: Some(ResistorModulation {
                lfo_name,
                depth: depth.clamp(0.0, 1.0),
                range: range.max(0.0),
            }),
            effective_resistance: r,
        }
    }

    /// Check if this resistor is modulated.
    pub fn is_modulated(&self) -> bool {
        self.modulation.is_some()
    }

    /// Update the effective resistance based on modulation signal.
    ///
    /// # Arguments
    /// * `mod_value` - The LFO value (0.0 to 1.0)
    pub fn update_modulation(&mut self, mod_value: f64) {
        if let Some(ref modulation) = self.modulation {
            // R_eff = R_base * (1 + depth * range * mod_value)
            // When mod_value = 0: R_eff = R_base
            // When mod_value = 1 and depth = 1: R_eff = R_base * (1 + range)
            let factor = 1.0 + modulation.depth * modulation.range * mod_value;
            self.effective_resistance = (self.resistance * factor).max(1e-12);
        }
    }

    /// Get the conductance (1/R) using the effective resistance.
    pub fn conductance(&self) -> f64 {
        1.0 / self.effective_resistance
    }

    /// Get the base (non-modulated) conductance.
    pub fn base_conductance(&self) -> f64 {
        1.0 / self.resistance
    }
}

/// A capacitor component.
///
/// In discrete-time simulation, a capacitor is modeled using a companion model.
/// Using the trapezoidal rule:
///   i(t) = (2C/dt) * v(t) - i_eq(t-dt)
///
/// where i_eq(t-dt) = (2C/dt) * v(t-dt) + i(t-dt)
///
/// This gives an equivalent conductance G_eq = 2C/dt and an equivalent
/// current source I_eq = -i_eq(t-dt).
#[derive(Debug, Clone)]
pub struct Capacitor {
    pub id: ComponentId,
    pub name: String,
    pub nodes: [NodeId; 2],
    pub capacitance: f64,

    // State for discrete-time model
    /// Previous voltage across capacitor
    pub v_prev: f64,
    /// Previous current through capacitor
    pub i_prev: f64,
}

impl Capacitor {
    /// Create a new capacitor.
    pub fn new(id: ComponentId, name: String, nodes: [NodeId; 2], capacitance: f64) -> Self {
        Self {
            id,
            name,
            nodes,
            capacitance,
            v_prev: 0.0,
            i_prev: 0.0,
        }
    }

    /// Get the equivalent conductance for the trapezoidal companion model.
    ///
    /// For a capacitor with trapezoidal integration:
    ///   i(n) = G * v(n) - I_eq
    /// where G = 2C/dt and I_eq = G*v(n-1) + i(n-1)
    pub fn conductance(&self, dt: f64) -> f64 {
        2.0 * self.capacitance / dt
    }

    /// Get the equivalent current source value for the companion model.
    ///
    /// The companion current source represents the "history" term and
    /// should be SUBTRACTED from the current. In MNA terms, we add a
    /// negative current source (current flowing out of node n+).
    pub fn current_source(&self, dt: f64) -> f64 {
        // I_eq = (2C/dt) * v_prev + i_prev
        // This gets SUBTRACTED, so we return -I_eq to add as a source
        -(self.conductance(dt) * self.v_prev + self.i_prev)
    }

    /// Update the state after solving.
    pub fn update_state(&mut self, v_new: f64, dt: f64) {
        // i_new = (2C/dt) * (v_new - v_prev) - i_prev
        let g = self.conductance(dt);
        let i_new = g * (v_new - self.v_prev) - self.i_prev;
        self.v_prev = v_new;
        self.i_prev = i_new;
    }
}

/// An inductor component.
///
/// In discrete-time simulation, an inductor is modeled using a companion model.
/// Using the trapezoidal rule:
///   v(t) = (2L/dt) * i(t) - v_eq(t-dt)
///
/// This requires an additional branch current variable in the MNA matrix.
#[derive(Debug, Clone)]
pub struct Inductor {
    pub id: ComponentId,
    pub name: String,
    pub nodes: [NodeId; 2],
    pub inductance: f64,
    pub branch: BranchId,

    // State for discrete-time model
    /// Previous current through inductor
    pub i_prev: f64,
    /// Previous voltage across inductor
    pub v_prev: f64,
}

impl Inductor {
    /// Create a new inductor.
    pub fn new(
        id: ComponentId,
        name: String,
        nodes: [NodeId; 2],
        inductance: f64,
        branch: BranchId,
    ) -> Self {
        Self {
            id,
            name,
            nodes,
            inductance,
            branch,
            i_prev: 0.0,
            v_prev: 0.0,
        }
    }

    /// Get the equivalent resistance for the trapezoidal companion model.
    pub fn resistance(&self, dt: f64) -> f64 {
        2.0 * self.inductance / dt
    }

    /// Get the equivalent voltage source value for the companion model.
    pub fn voltage_source(&self, dt: f64) -> f64 {
        // V_eq = (2L/dt) * i_prev + v_prev
        self.resistance(dt) * self.i_prev + self.v_prev
    }

    /// Update the state after solving.
    pub fn update_state(&mut self, i_new: f64, dt: f64) {
        // v_new = (2L/dt) * (i_new - i_prev) - v_prev
        let r = self.resistance(dt);
        let v_new = r * (i_new - self.i_prev) - self.v_prev;
        self.i_prev = i_new;
        self.v_prev = v_new;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resistor_conductance() {
        let r = Resistor::new(
            ComponentId(0),
            "R1".to_string(),
            [NodeId(1), NodeId(0)],
            1000.0,
        );
        assert!((r.conductance() - 0.001).abs() < 1e-10);
    }

    #[test]
    fn test_capacitor_companion_model() {
        let mut c = Capacitor::new(
            ComponentId(0),
            "C1".to_string(),
            [NodeId(1), NodeId(0)],
            1e-6, // 1µF
        );
        let dt = 1.0 / 48000.0;
        let g = c.conductance(dt);

        // G_eq = 2C/dt = 2 * 1e-6 / (1/48000) = 96 mS
        assert!((g - 0.096).abs() < 1e-6);

        // Initial current source should be 0
        assert!((c.current_source(dt)).abs() < 1e-10);

        // Update state with 1V across cap
        c.update_state(1.0, dt);
        assert!((c.v_prev - 1.0).abs() < 1e-10);
    }
}
