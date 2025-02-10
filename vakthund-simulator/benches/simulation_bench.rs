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
            // Create the Simulator with fixed latency and jitter parameters.
            let mut simulator = Simulator::new(seed, false, 100, 20, None);
            // Use black_box to ensure the result is not optimized away.
            black_box(simulator.run(num_events));
        })
    });
}

criterion_group!(benches, benchmark_simulation_throughput);
criterion_main!(benches);
