//! Component models for circuit simulation.
//!
//! This module provides models for all supported circuit components:
//! - Linear: Resistor, Capacitor, Inductor
//! - Sources: Voltage Source, Current Source
//! - Nonlinear: Diode, BJT, Op-Amp
//! - Controls: Potentiometer, Switch
//! - Digital: Delay Line, FDN Reverb
//! - Modulation: LFO
//!
//! Each component implements stamping into the MNA matrix.

mod linear;
mod sources;
mod diode;
mod bjt;
mod opamp;
mod controls;
mod delay;
mod reverb;
mod lfo;

pub use linear::{Resistor, Capacitor, Inductor};
pub use sources::{VoltageSource, CurrentSource};
pub use diode::Diode;
pub use bjt::{Bjt, BjtType};
pub use opamp::OpAmp;
pub use controls::{Potentiometer, Switch};
pub use delay::DelayLine;
pub use reverb::{FdnReverb, ReverbParams};
pub use lfo::{Lfo, LfoShape};

use crate::circuit::{BranchId, ComponentId, NodeId};
use crate::dsl::{ComponentDef, ComponentType, ModelDef};
use crate::error::{PedalerError, Result};

/// A circuit component.
#[derive(Debug, Clone)]
pub enum Component {
    Resistor(Resistor),
    Capacitor(Capacitor),
    Inductor(Inductor),
    VoltageSource(VoltageSource),
    CurrentSource(CurrentSource),
    Diode(Diode),
    Bjt(Bjt),
    OpAmp(OpAmp),
    Potentiometer(Potentiometer),
    Switch(Switch),
}

impl Component {
    /// Create a component from a DSL definition.
    pub fn from_def(
        id: ComponentId,
        def: &ComponentDef,
        nodes: &[NodeId],
        model: Option<&ModelDef>,
        branch_counter: &mut usize,
    ) -> Result<Self> {
        match def.component_type {
            ComponentType::Resistor => {
                let value = def.value.ok_or_else(|| {
                    PedalerError::invalid_component(&def.name, def.line, "resistor requires a value")
                })?;

                // Check for modulation parameters
                if let Some(lfo_name) = def.params.get("mod").or(def.params.get("lfo")) {
                    // Modulated resistor: mod=lfo_name depth=0.8 range=4
                    // We store lfo_name as a string reference through the value
                    let lfo_name_str = format!("LFO{}", *lfo_name as u32); // Temp workaround
                    let depth = def.params.get("depth").copied().unwrap_or(0.8);
                    let range = def.params.get("range").copied().unwrap_or(4.0);
                    Ok(Component::Resistor(Resistor::new_modulated(
                        id,
                        def.name.clone(),
                        [nodes[0], nodes[1]],
                        value,
                        lfo_name_str,
                        depth,
                        range,
                    )))
                } else if let Some(lfo_ref) = def.model_ref.as_ref().filter(|s| s.to_uppercase().starts_with("LFO")) {
                    // Alternative syntax: R1 n1 n2 10k LFO1 depth=0.8
                    let depth = def.params.get("depth").copied().unwrap_or(0.8);
                    let range = def.params.get("range").copied().unwrap_or(4.0);
                    Ok(Component::Resistor(Resistor::new_modulated(
                        id,
                        def.name.clone(),
                        [nodes[0], nodes[1]],
                        value,
                        lfo_ref.clone(),
                        depth,
                        range,
                    )))
                } else {
                    Ok(Component::Resistor(Resistor::new(
                        id,
                        def.name.clone(),
                        [nodes[0], nodes[1]],
                        value,
                    )))
                }
            }

            ComponentType::Capacitor => {
                let value = def.value.ok_or_else(|| {
                    PedalerError::invalid_component(&def.name, def.line, "capacitor requires a value")
                })?;
                Ok(Component::Capacitor(Capacitor::new(
                    id,
                    def.name.clone(),
                    [nodes[0], nodes[1]],
                    value,
                )))
            }

            ComponentType::Inductor => {
                let value = def.value.ok_or_else(|| {
                    PedalerError::invalid_component(&def.name, def.line, "inductor requires a value")
                })?;
                let branch = BranchId(*branch_counter);
                *branch_counter += 1;
                Ok(Component::Inductor(Inductor::new(
                    id,
                    def.name.clone(),
                    [nodes[0], nodes[1]],
                    value,
                    branch,
                )))
            }

            ComponentType::VoltageSource => {
                let value = def.value.unwrap_or(0.0);
                let branch = BranchId(*branch_counter);
                *branch_counter += 1;
                let is_ac = def.params.contains_key("ac");
                Ok(Component::VoltageSource(VoltageSource::new(
                    id,
                    def.name.clone(),
                    [nodes[0], nodes[1]],
                    value,
                    branch,
                    is_ac,
                )))
            }

            ComponentType::CurrentSource => {
                let value = def.value.unwrap_or(0.0);
                Ok(Component::CurrentSource(CurrentSource::new(
                    id,
                    def.name.clone(),
                    [nodes[0], nodes[1]],
                    value,
                )))
            }

            ComponentType::Diode => {
                let params = if let Some(m) = model {
                    diode::DiodeParams::from_model(m)
                } else {
                    diode::DiodeParams::default()
                };
                Ok(Component::Diode(Diode::new(
                    id,
                    def.name.clone(),
                    [nodes[0], nodes[1]], // anode, cathode
                    params,
                )))
            }

            ComponentType::Bjt => {
                let (bjt_type, params) = if let Some(m) = model {
                    bjt::BjtParams::from_model(m)?
                } else {
                    (BjtType::Npn, bjt::BjtParams::default())
                };
                Ok(Component::Bjt(Bjt::new(
                    id,
                    def.name.clone(),
                    [nodes[0], nodes[1], nodes[2]], // C, B, E
                    bjt_type,
                    params,
                )))
            }

            ComponentType::OpAmp => {
                let params = if let Some(m) = model {
                    opamp::OpAmpParams::from_model(m)
                } else {
                    opamp::OpAmpParams::ideal()
                };
                // Op-amp now uses VCCS model, no branch needed
                // We use a dummy branch ID
                let branch = BranchId(0);
                Ok(Component::OpAmp(OpAmp::new(
                    id,
                    def.name.clone(),
                    [nodes[0], nodes[1], nodes[2]], // out, in+, in-
                    params,
                    branch,
                )))
            }

            ComponentType::Potentiometer => {
                let total_resistance = def.value.ok_or_else(|| {
                    PedalerError::invalid_component(&def.name, def.line, "potentiometer requires a value")
                })?;
                let position = def.params.get("position").copied().unwrap_or(0.5);
                Ok(Component::Potentiometer(Potentiometer::new(
                    id,
                    def.name.clone(),
                    [nodes[0], nodes[1], nodes[2]], // n1, wiper, n2
                    total_resistance,
                    position,
                )))
            }

            ComponentType::Switch => {
                let closed = def.params.get("state").map(|v| *v > 0.5).unwrap_or(true);
                Ok(Component::Switch(Switch::new(
                    id,
                    def.name.clone(),
                    [nodes[0], nodes[1]],
                    closed,
                )))
            }

            // Digital effects and LFOs are handled separately in Circuit::from_ast
            // and should never reach this function
            ComponentType::Delay | ComponentType::Reverb | ComponentType::Lfo => {
                Err(PedalerError::invalid_component(
                    &def.name,
                    def.line,
                    "digital effects and LFOs should be handled separately",
                ))
            }
        }
    }

    /// Get the component ID.
    pub fn id(&self) -> ComponentId {
        match self {
            Component::Resistor(r) => r.id,
            Component::Capacitor(c) => c.id,
            Component::Inductor(l) => l.id,
            Component::VoltageSource(v) => v.id,
            Component::CurrentSource(i) => i.id,
            Component::Diode(d) => d.id,
            Component::Bjt(q) => q.id,
            Component::OpAmp(o) => o.id,
            Component::Potentiometer(p) => p.id,
            Component::Switch(s) => s.id,
        }
    }

    /// Get the component name.
    pub fn name(&self) -> &str {
        match self {
            Component::Resistor(r) => &r.name,
            Component::Capacitor(c) => &c.name,
            Component::Inductor(l) => &l.name,
            Component::VoltageSource(v) => &v.name,
            Component::CurrentSource(i) => &i.name,
            Component::Diode(d) => &d.name,
            Component::Bjt(q) => &q.name,
            Component::OpAmp(o) => &o.name,
            Component::Potentiometer(p) => &p.name,
            Component::Switch(s) => &s.name,
        }
    }

    /// Check if this component is nonlinear (requires Newton-Raphson iteration).
    pub fn is_nonlinear(&self) -> bool {
        matches!(self, Component::Diode(_) | Component::Bjt(_))
    }
}
