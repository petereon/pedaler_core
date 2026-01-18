//! Low Frequency Oscillator (LFO) for modulation effects.
//!
//! The LFO generates a control signal (0.0 to 1.0) that can be used
//! to modulate other components like resistors in phaser/flanger circuits.

use std::f64::consts::PI;

/// LFO waveform shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LfoShape {
    /// Sine wave (smooth, classic phaser sound)
    #[default]
    Sine,
    /// Triangle wave (linear sweep)
    Triangle,
    /// Sawtooth wave (rising ramp)
    Sawtooth,
    /// Square wave (abrupt switching)
    Square,
}

impl LfoShape {
    /// Parse shape from string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "sine" | "sin" => Some(Self::Sine),
            "triangle" | "tri" => Some(Self::Triangle),
            "sawtooth" | "saw" => Some(Self::Sawtooth),
            "square" | "sq" => Some(Self::Square),
            _ => None,
        }
    }
}

/// Low Frequency Oscillator for modulation.
#[derive(Debug, Clone)]
pub struct Lfo {
    /// Component name
    pub name: String,
    /// Oscillation rate in Hz
    rate: f64,
    /// Waveform shape
    shape: LfoShape,
    /// Current phase (0.0 to 1.0)
    phase: f64,
    /// Phase increment per sample
    phase_increment: f64,
    /// Current output value (0.0 to 1.0)
    pub value: f64,
}

impl Lfo {
    /// Create a new LFO.
    ///
    /// # Arguments
    /// * `name` - Component name
    /// * `rate` - Oscillation rate in Hz (typically 0.1 to 10 Hz)
    /// * `shape` - Waveform shape
    /// * `sample_rate` - Audio sample rate in Hz
    pub fn new(name: String, rate: f64, shape: LfoShape, sample_rate: f64) -> Self {
        let phase_increment = rate / sample_rate;
        Self {
            name,
            rate,
            shape,
            phase: 0.0,
            phase_increment,
            value: 0.5, // Start at middle
        }
    }

    /// Get the current LFO rate in Hz.
    pub fn rate(&self) -> f64 {
        self.rate
    }

    /// Set the LFO rate in Hz.
    pub fn set_rate(&mut self, rate: f64, sample_rate: f64) {
        self.rate = rate;
        self.phase_increment = rate / sample_rate;
    }

    /// Advance the LFO by one sample and return the new value.
    ///
    /// Returns a value in the range [0.0, 1.0].
    pub fn tick(&mut self) -> f64 {
        // Calculate output based on shape
        self.value = match self.shape {
            LfoShape::Sine => {
                // Sine: 0.5 + 0.5 * sin(2π * phase)
                0.5 + 0.5 * (2.0 * PI * self.phase).sin()
            }
            LfoShape::Triangle => {
                // Triangle: rises from 0 to 1 in first half, falls from 1 to 0 in second half
                if self.phase < 0.5 {
                    2.0 * self.phase
                } else {
                    2.0 * (1.0 - self.phase)
                }
            }
            LfoShape::Sawtooth => {
                // Sawtooth: rises linearly from 0 to 1
                self.phase
            }
            LfoShape::Square => {
                // Square: 0 for first half, 1 for second half
                if self.phase < 0.5 { 0.0 } else { 1.0 }
            }
        };

        // Advance phase
        self.phase += self.phase_increment;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        self.value
    }

    /// Get the current output value without advancing.
    pub fn current_value(&self) -> f64 {
        self.value
    }

    /// Reset the LFO phase.
    pub fn reset(&mut self) {
        self.phase = 0.0;
        self.value = match self.shape {
            LfoShape::Sine => 0.5,
            LfoShape::Triangle => 0.0,
            LfoShape::Sawtooth => 0.0,
            LfoShape::Square => 0.0,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lfo_sine() {
        let mut lfo = Lfo::new("LFO1".to_string(), 1.0, LfoShape::Sine, 4.0);

        // At 1 Hz with 4 samples/sec, we get 4 samples per cycle
        // phase: 0.0, 0.25, 0.5, 0.75
        let v0 = lfo.tick(); // phase 0: sin(0) = 0.5
        assert!((v0 - 0.5).abs() < 0.01);

        let v1 = lfo.tick(); // phase 0.25: sin(π/2) = 1.0
        assert!((v1 - 1.0).abs() < 0.01);

        let v2 = lfo.tick(); // phase 0.5: sin(π) = 0.5
        assert!((v2 - 0.5).abs() < 0.01);

        let v3 = lfo.tick(); // phase 0.75: sin(3π/2) = 0.0
        assert!((v3 - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_lfo_triangle() {
        let mut lfo = Lfo::new("LFO1".to_string(), 1.0, LfoShape::Triangle, 4.0);

        let v0 = lfo.tick(); // phase 0: 0
        assert!((v0 - 0.0).abs() < 0.01);

        let v1 = lfo.tick(); // phase 0.25: 0.5
        assert!((v1 - 0.5).abs() < 0.01);

        let v2 = lfo.tick(); // phase 0.5: 1.0
        assert!((v2 - 1.0).abs() < 0.01);

        let v3 = lfo.tick(); // phase 0.75: 0.5
        assert!((v3 - 0.5).abs() < 0.01);
    }
}
