#[macro_use]
extern crate criterion;

use bytes::Bytes;
use criterion::{black_box, Criterion};

use vakthund_protocols::{CoapParser, ModbusParser, MqttParser};

// Source: https://www.hivemq.com/mqtt-essentials/mqtt-message-format/
// Example of a complete MQTT Connect package
const MQTT_DATA: &[u8] = &[
    0x10, 0x0E, // Connect packet, remaining length
    0x00, 0x04, 0x4D, 0x51, 0x54, 0x54, // MQTT
    0x04, // Protocol level
    0x02, // Connect flags
    0x00, 0x3C, // Keepalive
    0x00, 0x0B, // Client ID length
    0x74, 0x65, 0x73, 0x74, 0x63, 0x6c, 0x69, 0x65, 0x6e, 0x74, 0x31, 0x32,
];

// Source: https://datatracker.ietf.org/doc/html/rfc7252#section-3
// Example confirmable request with payload
const COAP_DATA: &[u8] = &[
    0x44, // Version 1, Type 0 (Confirmable), Token Length 4
    0x01, // GET Method
    0x6A, 0x50, // Message ID
    0x74, 0x65, 0x73, 0x74, 0xFF, // Payload Marker
    0x48, 0x65, 0x6c, 0x6c, 0x6f, // Payload "Hello"
];

// Example of Modbus TCP Read Holding Registers
// Source: https://simplymodbus.ca/tcp.htm
const MODBUS_DATA: &[u8] = &[
    0x00, 0x01, // Transaction ID
    0x00, 0x00, // Protocol ID
    0x00, 0x06, // Length
    0x01, // Unit ID
    0x03, // Function Code (Read Holding Registers)
    0x00, 0x6B, // Start Address (99)
    0x00, 0x03, // Quantity of Registers (3)
];

fn benchmark_mqtt_parsing(c: &mut Criterion) {
    let parser = MqttParser::new();
    let mqtt_data = Bytes::from_static(MQTT_DATA);

    c.bench_function("mqtt_parsing", |b| {
        b.iter(|| {
            black_box(parser.parse(&mqtt_data)).unwrap();
        })
    });
}

fn benchmark_coap_parsing(c: &mut Criterion) {
    let parser = CoapParser::new();
    let coap_data = Bytes::from_static(COAP_DATA);

    c.bench_function("coap_parsing", |b| {
        b.iter(|| {
            black_box(parser.parse(&coap_data)).unwrap();
        })
    });
}

fn benchmark_modbus_parsing(c: &mut Criterion) {
    let parser = ModbusParser::new();
    let modbus_data = Bytes::from_static(MODBUS_DATA);

    c.bench_function("modbus_parsing", |b| {
        b.iter(|| {
            black_box(parser.parse(&modbus_data)).unwrap();
        })
    });
}

criterion_group!(
    benches,
    benchmark_mqtt_parsing,
    benchmark_coap_parsing,
    benchmark_modbus_parsing
);
criterion_main!(benches);
