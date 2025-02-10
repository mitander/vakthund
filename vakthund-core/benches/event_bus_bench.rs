#[macro_use]
extern crate criterion;

use bytes::Bytes;
use criterion::{black_box, Criterion};
use vakthund_core::events::{bus::EventBus, network::NetworkEvent};

fn benchmark_event_bus_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("event_bus_throughput");

    for capacity in [128, 1024, 16384] {
        group.throughput(criterion::Throughput::Elements(capacity as u64));
        group.bench_function(format!("capacity_{}", capacity), |b| {
            let event_bus = EventBus::with_capacity(capacity).unwrap();
            let event = NetworkEvent {
                timestamp: 0,
                payload: Bytes::from_static(b"test_payload"),
                source: None,
                destination: None,
            };
            b.iter(|| {
                // Use black_box to prevent over‑optimization.
                black_box(event_bus.try_push(event.clone()).unwrap());
                black_box(event_bus.try_pop().unwrap());
            });
        });
    }
    group.finish();
}

criterion_group!(benches, benchmark_event_bus_throughput);
criterion_main!(benches);
