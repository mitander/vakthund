#[macro_use]
extern crate criterion;

use bytes::Bytes;
use criterion::Criterion;

use vakthund_core::event_bus::{EventBus, NetworkEvent};

fn bench_event_bus_push_pop(c: &mut Criterion) {
    let mut group = c.benchmark_group("event_bus_throughput");

    for capacity in [128, 1024, 16384] {
        group.throughput(criterion::Throughput::Elements(capacity as u64)); // Events per second
        group.bench_function(format!("capacity_{}", capacity), |b| {
            let event_bus = EventBus::with_capacity(capacity).unwrap();
            let event = NetworkEvent {
                timestamp: 0,
                payload: Bytes::from_static(b"test_payload"),
            };
            b.iter(|| {
                event_bus.try_push(event.clone()).unwrap();
                event_bus.try_pop().unwrap();
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_event_bus_push_pop);
criterion_main!(benches);
