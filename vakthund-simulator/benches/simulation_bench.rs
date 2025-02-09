#[macro_use]
extern crate criterion;

use criterion::{black_box, Criterion};
use vakthund_simulator::Simulator;

/// Benchmark the simulation throughput by running the simulator for a fixed number of events.
fn benchmark_simulation_throughput(c: &mut Criterion) {
    // Number of events to simulate per iteration.
    let num_events = 100_000;
    // Fixed seed for reproducibility.
    let seed = 42;

    c.bench_function("simulation_throughput", |b| {
        b.iter(|| {
            // black_box ensures that the compiler does not optimize away the simulation
            let mut simulator = Simulator::new(seed, false);
            black_box(simulator.run(num_events));
        })
    });
}

criterion_group!(benches, benchmark_simulation_throughput);
criterion_main!(benches);
