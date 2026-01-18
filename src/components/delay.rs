//! Delay line component for time-based effects.
//!
//! A digital delay line stores samples in a ring buffer and outputs
//! the delayed signal. This is the fundamental building block for
//! delay, echo, chorus, flanger, and reverb effects.

use crate::circuit::NodeId;

/// A digital delay line with mix and feedback controls.
#[derive(Debug, Clone)]
pub struct DelayLine {
    /// Component name
    pub name: String,
    /// Input node (reads voltage from here)
    pub input_node: NodeId,
    /// Output node (writes delayed voltage here)
    pub output_node: NodeId,
    /// Ring buffer for storing samples
    buffer: Vec<f32>,
    /// Current write position in the buffer
    write_pos: usize,
    /// Delay time in samples
    delay_samples: usize,
    /// Dry/wet mix (0.0 = dry only, 1.0 = wet only, 0.5 = equal mix)
    mix: f32,
    /// Feedback amount (0.0 = no feedback, 0.5 = 50% feedback for echo)
    feedback: f32,
}

impl DelayLine {
    /// Create a new delay line.
    ///
    /// # Arguments
    /// * `name` - Component name
    /// * `input_node` - Node to read input voltage from
    /// * `output_node` - Node to write delayed voltage to
    /// * `delay_time` - Delay time in seconds
    /// * `sample_rate` - Sample rate in Hz
    /// * `mix` - Dry/wet mix (0.0-1.0, default 0.5)
    /// * `feedback` - Feedback amount (0.0-1.0, default 0.3)
    pub fn new(
        name: String,
        input_node: NodeId,
        output_node: NodeId,
        delay_time: f64,
        sample_rate: f32,
        mix: f32,
        feedback: f32,
    ) -> Self {
        let delay_samples = ((delay_time * sample_rate as f64) as usize).max(1);
        let buffer = vec![0.0; delay_samples];

        Self {
            name,
            input_node,
            output_node,
            buffer,
            write_pos: 0,
            delay_samples,
            mix: mix.clamp(0.0, 1.0),
            feedback: feedback.clamp(0.0, 0.95), // Limit to prevent runaway
        }
    }

    /// Get the delay time in samples.
    pub fn delay_samples(&self) -> usize {
        self.delay_samples
    }

    /// Get the delay time in seconds at the given sample rate.
    pub fn delay_time(&self, sample_rate: f32) -> f64 {
        self.delay_samples as f64 / sample_rate as f64
    }

    /// Process one sample through the delay line.
    ///
    /// # Arguments
    /// * `input` - Input sample value
    ///
    /// # Returns
    /// Mixed output: dry * (1-mix) + wet * mix
    pub fn process(&mut self, input: f32) -> f32 {
        // Read the delayed sample
        let delayed = self.buffer[self.write_pos];

        // Write input + feedback to buffer
        self.buffer[self.write_pos] = input + delayed * self.feedback;

        // Advance the write position
        self.write_pos = (self.write_pos + 1) % self.delay_samples;

        // Mix dry and wet signals
        input * (1.0 - self.mix) + delayed * self.mix
    }

    /// Reset the delay line (clear buffer).
    pub fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.write_pos = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delay_line_basic() {
        // Using mix=1.0 to get pure delayed output for testing
        let mut delay = DelayLine::new(
            "D1".to_string(),
            NodeId(1),
            NodeId(2),
            0.01,   // 10ms delay
            1000.0, // 1kHz sample rate = 10 samples delay
            1.0,    // 100% wet for testing
            0.0,    // No feedback
        );

        assert_eq!(delay.delay_samples(), 10);

        // First 10 samples should output 0 (buffer is empty)
        for i in 0..10 {
            let out = delay.process((i + 1) as f32);
            assert_eq!(out, 0.0);
        }

        // Now we should get the delayed samples back
        for i in 0..10 {
            let out = delay.process((i + 11) as f32);
            assert_eq!(out, (i + 1) as f32);
        }
    }

    #[test]
    fn test_delay_line_with_mix() {
        // Test dry/wet mixing
        let mut delay = DelayLine::new(
            "D1".to_string(),
            NodeId(1),
            NodeId(2),
            0.005,  // 5ms delay
            1000.0, // 1kHz sample rate = 5 samples delay
            0.5,    // 50% mix
            0.0,    // No feedback
        );

        // Fill buffer first
        for _ in 0..5 {
            delay.process(0.0);
        }

        // Now input 1.0, delayed is 0.0
        // Output should be: 1.0 * 0.5 + 0.0 * 0.5 = 0.5
        let out = delay.process(1.0);
        assert!((out - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_delay_line_reset() {
        let mut delay = DelayLine::new(
            "D1".to_string(),
            NodeId(1),
            NodeId(2),
            0.005,
            1000.0,
            1.0,
            0.0,
        );

        // Fill with some values
        for i in 0..10 {
            delay.process(i as f32);
        }

        // Reset
        delay.reset();

        // Should output zeros again
        for _ in 0..5 {
            assert_eq!(delay.process(1.0), 0.0);
        }
    }
}
