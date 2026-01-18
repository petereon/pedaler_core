//! Pedaler - Guitar Pedal Circuit Simulator
//!
//! A real-time circuit simulator for guitar effects pedals.
//!
//! # Usage
//!
//! ```bash
//! ffmpeg -i input.wav -f f32le -ac 1 -ar 48000 - | pedaler circuit.ped | ffmpeg -f f32le -ac 1 -ar 48000 -i - output.wav
//! ```

use std::path::PathBuf;

use clap::Parser;
use pedaler_core::{
    audio::process_audio,
    circuit::Circuit,
    dsl,
    error::Result,
    Simulator, DEFAULT_SAMPLE_RATE,
};

/// Guitar pedal circuit simulator
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the circuit description file (.ped)
    #[arg(value_name = "CIRCUIT_FILE")]
    circuit_file: PathBuf,

    /// Sample rate in Hz
    #[arg(short, long, default_value_t = DEFAULT_SAMPLE_RATE)]
    sample_rate: f32,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Parse the circuit file
    let ast = dsl::parse_file(&args.circuit_file)?;

    // Build the circuit
    let circuit = Circuit::from_ast(ast)?;

    // Validate
    pedaler_core::circuit::validate_circuit(&circuit)?;

    // Create simulator
    let mut simulator = Simulator::new(circuit, args.sample_rate);

    // Process audio
    process_audio(&mut simulator)?;

    Ok(())
}
