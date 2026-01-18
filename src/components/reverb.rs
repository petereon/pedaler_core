//! FDN (Feedback Delay Network) reverb component.
//!
//! Implements an algorithmic reverb using multiple delay lines with a
//! Hadamard feedback matrix. This creates dense, natural-sounding
//! reverberation without requiring impulse response files.

use crate::circuit::NodeId;

/// Default number of delay lines in the FDN.
const NUM_DELAY_LINES: usize = 4;

/// Base delay times in seconds (mutually prime in samples at common rates).
/// These create a natural-sounding reverb without metallic resonances.
const BASE_DELAY_TIMES: [f64; NUM_DELAY_LINES] = [0.029, 0.037, 0.043, 0.053];

/// FDN Reverb parameters.
#[derive(Debug, Clone)]
pub struct ReverbParams {
    /// Decay amount (0.0 to 1.0) - controls reverb tail length
    pub decay: f32,
    /// Room size (0.0 to 1.0) - scales delay line lengths
    pub size: f32,
    /// High-frequency damping (0.0 to 1.0) - higher = more damping
    pub damping: f32,
    /// Dry/wet mix (0.0 = dry only, 1.0 = wet only)
    pub mix: f32,
    /// Pre-delay in seconds
    pub predelay: f64,
}

impl Default for ReverbParams {
    fn default() -> Self {
        Self {
            decay: 0.5,
            size: 0.5,
            damping: 0.3,
            mix: 0.5,
            predelay: 0.0,
        }
    }
}

impl ReverbParams {
    /// Create parameters from a hashmap of string keys and f64 values.
    pub fn from_params(params: &std::collections::HashMap<String, f64>) -> Self {
        let mut p = Self::default();
        if let Some(&v) = params.get("decay") {
            p.decay = v.clamp(0.0, 0.99) as f32;
        }
        if let Some(&v) = params.get("size") {
            p.size = v.clamp(0.0, 1.0) as f32;
        }
        if let Some(&v) = params.get("damping") {
            p.damping = v.clamp(0.0, 1.0) as f32;
        }
        if let Some(&v) = params.get("mix") {
            p.mix = v.clamp(0.0, 1.0) as f32;
        }
        if let Some(&v) = params.get("predelay") {
            p.predelay = v.max(0.0);
        }
        p
    }
}

/// A Feedback Delay Network reverb.
#[derive(Debug, Clone)]
pub struct FdnReverb {
    /// Component name
    pub name: String,
    /// Input node
    pub input_node: NodeId,
    /// Output node
    pub output_node: NodeId,
    /// Parameters
    pub params: ReverbParams,
    /// Delay line buffers
    delay_buffers: [Vec<f32>; NUM_DELAY_LINES],
    /// Write positions for each delay line
    write_positions: [usize; NUM_DELAY_LINES],
    /// Delay lengths in samples
    delay_lengths: [usize; NUM_DELAY_LINES],
    /// One-pole lowpass filter states for damping
    lp_states: [f32; NUM_DELAY_LINES],
    /// Pre-delay buffer (optional)
    predelay_buffer: Vec<f32>,
    /// Pre-delay write position
    predelay_pos: usize,
    /// Pre-delay length in samples
    predelay_len: usize,
}

impl FdnReverb {
    /// Create a new FDN reverb.
    pub fn new(
        name: String,
        input_node: NodeId,
        output_node: NodeId,
        params: ReverbParams,
        sample_rate: f32,
    ) -> Self {
        // Calculate delay lengths based on size parameter
        // Size scales from 0.5x to 2x the base delay times
        let size_scale = 0.5 + params.size as f64 * 1.5;

        let mut delay_lengths = [0usize; NUM_DELAY_LINES];
        let mut delay_buffers: [Vec<f32>; NUM_DELAY_LINES] = Default::default();

        for i in 0..NUM_DELAY_LINES {
            let delay_time = BASE_DELAY_TIMES[i] * size_scale;
            let len = ((delay_time * sample_rate as f64) as usize).max(1);
            delay_lengths[i] = len;
            delay_buffers[i] = vec![0.0; len];
        }

        // Pre-delay
        let predelay_len = ((params.predelay * sample_rate as f64) as usize).max(1);
        let predelay_buffer = vec![0.0; predelay_len];

        Self {
            name,
            input_node,
            output_node,
            params,
            delay_buffers,
            write_positions: [0; NUM_DELAY_LINES],
            delay_lengths,
            lp_states: [0.0; NUM_DELAY_LINES],
            predelay_buffer,
            predelay_pos: 0,
            predelay_len,
        }
    }

    /// Process one sample through the reverb.
    pub fn process(&mut self, input: f32) -> f32 {
        // Apply pre-delay if configured
        let predelayed = if self.predelay_len > 1 {
            let out = self.predelay_buffer[self.predelay_pos];
            self.predelay_buffer[self.predelay_pos] = input;
            self.predelay_pos = (self.predelay_pos + 1) % self.predelay_len;
            out
        } else {
            input
        };

        // Read delayed outputs
        let mut delayed = [0.0f32; NUM_DELAY_LINES];
        for i in 0..NUM_DELAY_LINES {
            delayed[i] = self.delay_buffers[i][self.write_positions[i]];
        }

        // Apply damping (one-pole lowpass filter)
        let damping = self.params.damping;
        for i in 0..NUM_DELAY_LINES {
            self.lp_states[i] = self.lp_states[i] * damping + delayed[i] * (1.0 - damping);
            delayed[i] = self.lp_states[i];
        }

        // Apply Hadamard feedback matrix (4x4)
        // H = 1/2 * [[ 1,  1,  1,  1],
        //           [ 1, -1,  1, -1],
        //           [ 1,  1, -1, -1],
        //           [ 1, -1, -1,  1]]
        let feedback = hadamard_4x4(&delayed);

        // Scale by decay and write back to delay lines
        let decay = self.params.decay;
        for i in 0..NUM_DELAY_LINES {
            let new_sample = predelayed + feedback[i] * decay;
            self.delay_buffers[i][self.write_positions[i]] = new_sample;
            self.write_positions[i] = (self.write_positions[i] + 1) % self.delay_lengths[i];
        }

        // Sum delayed outputs for wet signal
        let wet = (delayed[0] + delayed[1] + delayed[2] + delayed[3]) * 0.25;

        // Mix dry and wet
        let mix = self.params.mix;
        input * (1.0 - mix) + wet * mix
    }

    /// Reset the reverb state.
    pub fn reset(&mut self) {
        for buf in &mut self.delay_buffers {
            buf.fill(0.0);
        }
        self.write_positions = [0; NUM_DELAY_LINES];
        self.lp_states = [0.0; NUM_DELAY_LINES];
        self.predelay_buffer.fill(0.0);
        self.predelay_pos = 0;
    }
}

/// Apply 4x4 Hadamard matrix to input vector.
/// The Hadamard matrix is unitary (energy-preserving) which prevents
/// the reverb from building up or dying out unnaturally.
#[inline]
fn hadamard_4x4(x: &[f32; 4]) -> [f32; 4] {
    // H * x where H is the 4x4 Hadamard matrix divided by 2
    let a = x[0] + x[1] + x[2] + x[3];
    let b = x[0] - x[1] + x[2] - x[3];
    let c = x[0] + x[1] - x[2] - x[3];
    let d = x[0] - x[1] - x[2] + x[3];
    [a * 0.5, b * 0.5, c * 0.5, d * 0.5]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hadamard() {
        let input = [1.0, 0.0, 0.0, 0.0];
        let output = hadamard_4x4(&input);
        // First column of Hadamard/2: [0.5, 0.5, 0.5, 0.5]
        assert!((output[0] - 0.5).abs() < 1e-6);
        assert!((output[1] - 0.5).abs() < 1e-6);
        assert!((output[2] - 0.5).abs() < 1e-6);
        assert!((output[3] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_reverb_decay() {
        let params = ReverbParams {
            decay: 0.5,
            size: 0.5,
            damping: 0.0,
            mix: 1.0, // 100% wet
            predelay: 0.0,
        };

        let mut reverb = FdnReverb::new(
            "RV1".to_string(),
            NodeId(1),
            NodeId(2),
            params,
            48000.0,
        );

        // Send an impulse
        let _ = reverb.process(1.0);

        // Process many samples and check decay
        let mut max_output = 0.0f32;
        for _ in 0..48000 {
            let out = reverb.process(0.0);
            max_output = max_output.max(out.abs());
        }

        // After 1 second with decay=0.5, should have decayed significantly
        // The FDN recirculates so it won't fully decay, but should be below 0.5
        assert!(max_output < 0.5, "Reverb should decay, got max {}", max_output);
    }

    #[test]
    fn test_reverb_mix() {
        let dry_params = ReverbParams {
            mix: 0.0, // 100% dry
            ..Default::default()
        };

        let mut reverb = FdnReverb::new(
            "RV1".to_string(),
            NodeId(1),
            NodeId(2),
            dry_params,
            48000.0,
        );

        // With 100% dry, output should equal input
        let out = reverb.process(0.5);
        assert!((out - 0.5).abs() < 1e-6);
    }
}
