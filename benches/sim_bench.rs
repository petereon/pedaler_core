use std::path::Path;

use criterion::{criterion_group, criterion_main, Criterion};
use pedaler_core::{Circuit, Simulator, dsl}; // Adjust imports as needed

fn bench_opamp_overdrive_step(c: &mut Criterion) {
    // Load the circuit from your example file
    let ast = dsl::parse_file(Path::new("examples/circuits/opamp_overdrive.ped")).unwrap();
    let circuit = Circuit::from_ast(ast).unwrap();
    let mut sim = Simulator::new(circuit, 48000.0);

    // Benchmark a single step
    c.bench_function("opamp_overdrive_step", |b| {
        b.iter(|| {
            sim.set_input(0.5);
            let _ = sim.step().unwrap();
        });
    });
}

fn bench_distortion_step(c: &mut Criterion) {
    // Load the circuit from your example file
    let ast = dsl::parse_file(Path::new("examples/circuits/distortion.ped")).unwrap();
    let circuit = Circuit::from_ast(ast).unwrap();
    let mut sim = Simulator::new(circuit, 48000.0);

    // Benchmark a single step
    c.bench_function("distortion_step", |b| {
        b.iter(|| {
            sim.set_input(0.5);
            let _ = sim.step().unwrap();
        });
    });
}

criterion_group!(benches, bench_opamp_overdrive_step, bench_distortion_step);
criterion_main!(benches);
