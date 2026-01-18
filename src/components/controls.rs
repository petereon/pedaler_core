//! Control components: Potentiometer and Switch.

use crate::circuit::{ComponentId, NodeId};

/// A potentiometer component.
///
/// Modeled as two resistors in series with a wiper tap:
///   n1 ----[R1]---- wiper ----[R2]---- n2
///
/// where R1 = position * total_resistance
/// and   R2 = (1 - position) * total_resistance
#[derive(Debug, Clone)]
pub struct Potentiometer {
    pub id: ComponentId,
    pub name: String,
    pub nodes: [NodeId; 3], // [n1, wiper, n2]
    pub total_resistance: f64,
    /// Position from 0.0 to 1.0
    pub position: f64,
}

impl Potentiometer {
    /// Create a new potentiometer.
    pub fn new(
        id: ComponentId,
        name: String,
        nodes: [NodeId; 3],
        total_resistance: f64,
        position: f64,
    ) -> Self {
        Self {
            id,
            name,
            nodes,
            total_resistance: total_resistance.max(1.0), // Minimum 1 ohm
            position: position.clamp(0.001, 0.999), // Avoid zero resistance
        }
    }

    /// Get the resistance from n1 to wiper.
    pub fn r1(&self) -> f64 {
        (self.position * self.total_resistance).max(0.1)
    }

    /// Get the resistance from wiper to n2.
    pub fn r2(&self) -> f64 {
        ((1.0 - self.position) * self.total_resistance).max(0.1)
    }

    /// Get the conductance from n1 to wiper.
    pub fn g1(&self) -> f64 {
        1.0 / self.r1()
    }

    /// Get the conductance from wiper to n2.
    pub fn g2(&self) -> f64 {
        1.0 / self.r2()
    }

    /// Set the wiper position.
    pub fn set_position(&mut self, position: f64) {
        self.position = position.clamp(0.001, 0.999);
    }

    /// Get node n1.
    pub fn n1(&self) -> NodeId {
        self.nodes[0]
    }

    /// Get the wiper node.
    pub fn wiper(&self) -> NodeId {
        self.nodes[1]
    }

    /// Get node n2.
    pub fn n2(&self) -> NodeId {
        self.nodes[2]
    }
}

/// A switch component.
///
/// Modeled as a resistance:
/// - Closed: very small resistance (0.01 ohms)
/// - Open: very large resistance (1e9 ohms)
#[derive(Debug, Clone)]
pub struct Switch {
    pub id: ComponentId,
    pub name: String,
    pub nodes: [NodeId; 2],
    pub closed: bool,
}

impl Switch {
    /// Resistance when closed.
    pub const R_CLOSED: f64 = 0.01;
    /// Resistance when open.
    pub const R_OPEN: f64 = 1e9;

    /// Create a new switch.
    pub fn new(id: ComponentId, name: String, nodes: [NodeId; 2], closed: bool) -> Self {
        Self {
            id,
            name,
            nodes,
            closed,
        }
    }

    /// Get the current resistance.
    pub fn resistance(&self) -> f64 {
        if self.closed {
            Self::R_CLOSED
        } else {
            Self::R_OPEN
        }
    }

    /// Get the current conductance.
    pub fn conductance(&self) -> f64 {
        1.0 / self.resistance()
    }

    /// Set the switch state.
    pub fn set_state(&mut self, closed: bool) {
        self.closed = closed;
    }

    /// Toggle the switch state.
    pub fn toggle(&mut self) {
        self.closed = !self.closed;
    }
}
