//! Audio I/O for the CLI frontend.
//!
//! Handles reading raw PCM audio from stdin and writing to stdout.

use std::io::{self, Read, Write};

use crate::error::{PedalerError, Result};

/// Buffer size for audio processing (in samples).
pub const BUFFER_SIZE: usize = 256;

/// Audio input reader from stdin.
pub struct AudioInput {
    buffer: Vec<u8>,
}

impl AudioInput {
    /// Create a new audio input reader.
    pub fn new() -> Self {
        Self {
            buffer: vec![0u8; BUFFER_SIZE * 4], // 4 bytes per f32
        }
    }

    /// Read a block of samples from stdin.
    /// Returns the number of samples read, or 0 on EOF.
    pub fn read_block(&mut self, samples: &mut [f32]) -> Result<usize> {
        let bytes_to_read = samples.len() * 4;
        let buffer = &mut self.buffer[..bytes_to_read];

        let bytes_read = io::stdin()
            .read(buffer)
            .map_err(|e| PedalerError::AudioInputError {
                message: e.to_string(),
            })?;

        if bytes_read == 0 {
            return Ok(0);
        }

        let samples_read = bytes_read / 4;

        for i in 0..samples_read {
            let bytes: [u8; 4] = [
                buffer[i * 4],
                buffer[i * 4 + 1],
                buffer[i * 4 + 2],
                buffer[i * 4 + 3],
            ];
            samples[i] = f32::from_le_bytes(bytes);
        }

        Ok(samples_read)
    }
}

impl Default for AudioInput {
    fn default() -> Self {
        Self::new()
    }
}

/// Audio output writer to stdout.
pub struct AudioOutput {
    buffer: Vec<u8>,
}

impl AudioOutput {
    /// Create a new audio output writer.
    pub fn new() -> Self {
        Self {
            buffer: vec![0u8; BUFFER_SIZE * 4],
        }
    }

    /// Write a block of samples to stdout.
    pub fn write_block(&mut self, samples: &[f32]) -> Result<()> {
        let bytes_needed = samples.len() * 4;
        if self.buffer.len() < bytes_needed {
            self.buffer.resize(bytes_needed, 0);
        }

        for (i, &sample) in samples.iter().enumerate() {
            let bytes = sample.to_le_bytes();
            self.buffer[i * 4] = bytes[0];
            self.buffer[i * 4 + 1] = bytes[1];
            self.buffer[i * 4 + 2] = bytes[2];
            self.buffer[i * 4 + 3] = bytes[3];
        }

        io::stdout()
            .write_all(&self.buffer[..bytes_needed])
            .map_err(|e| PedalerError::AudioOutputError {
                message: e.to_string(),
            })?;

        Ok(())
    }

    /// Flush the output stream.
    pub fn flush(&mut self) -> Result<()> {
        io::stdout()
            .flush()
            .map_err(|e| PedalerError::AudioOutputError {
                message: e.to_string(),
            })
    }
}

impl Default for AudioOutput {
    fn default() -> Self {
        Self::new()
    }
}

/// Process audio from stdin to stdout using the given simulator.
pub fn process_audio(simulator: &mut crate::Simulator) -> Result<()> {
    let mut input = AudioInput::new();
    let mut output = AudioOutput::new();

    let mut in_samples = vec![0.0f32; BUFFER_SIZE];
    let mut out_samples = vec![0.0f32; BUFFER_SIZE];

    loop {
        let samples_read = input.read_block(&mut in_samples)?;

        if samples_read == 0 {
            break;
        }

        simulator.process_block(&in_samples[..samples_read], &mut out_samples[..samples_read])?;
        output.write_block(&out_samples[..samples_read])?;
    }

    output.flush()?;
    Ok(())
}
