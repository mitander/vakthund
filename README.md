# vakthund 🐶

A deterministic Intrusion Detection and Prevention System (IDPS) in Rust. Currently in early development phase.

## Current State

- **Core Components**:
  - Memory management with arena allocation
  - Event bus for inter-component communication
  - Basic packet capture using pcap
  - Protocol parsing (WIP)
  - Deterministic simulation framework

- **Design Goals**:
  - Zero-copy data processing
  - Deterministic behavior
  - Clear module boundaries
  - Performance-focused

## Building

```bash
cargo build --workspace --release
```

## Testing

```bash
cargo test --workspace
```

## Benchmarking

```bash
cargo bench --workspace
```

## License

MIT License - See [LICENSE](LICENSE)
