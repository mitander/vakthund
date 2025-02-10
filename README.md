# vakthund 🐶

**Vakthund** is a deterministic Intrusion Detection and Prevention System (IDPS) built for IoT products. Designed as a multi‑crate workspace, Vakthund emphasizes clear module boundaries, zero‑copy data processing, and reproducible simulation to enable both robust production operation and thorough testing/debugging.

---

## Overview

- **vakthund-core:**
    The foundation of the system. This crate provides shared types and utilities including configuration management, error handling, logging, packet structures, and a high‑performance event bus. It also contains key modules for memory allocation (using techniques like arena allocation with `bumpalo`) and simulation logging.

- **vakthund-capture:**
    This module implements a unified capture interface. Currently, live capture is implemented using [pcap](https://github.com/the-tcpdump-group/libpcap) for real‑time network packet acquisition. It’s designed to support zero‑copy processing through efficient packet types.

- **vakthund-protocols:**
    Contains protocol parsers for various network protocols (e.g. MQTT, CoAP). Parsers in this crate are optimized for zero‑copy parsing, meaning they efficiently process packet data without unnecessary memory allocations.

- **vakthund-detection:**
    Houses the threat detection logic. The detection engine uses signature‑based methods (powered by the Aho‑Corasick algorithm) to quickly scan network payloads, as well as potential anomaly‑based detection methods.

- **vakthund-prevention:**
    Implements prevention mechanisms such as an eBPF/XDP‑based firewall. This module is designed for fast, in‑kernel packet filtering and mitigation, ensuring low‑latency responses in live environments.

- **vakthund-telemetry:**
    Manages logging and metrics export. With built‑in support for Prometheus and OpenTelemetry, this crate allows for detailed observability of system events, detection latencies, and overall system health.

- **vakthund-simulator:**
    Provides a deterministic simulation engine that enables reproducible testing and debugging. By leveraging a virtual clock and configurable network simulation models (including fixed latency, jitter, and packet loss), you can simulate complex scenarios in a controlled, deterministic manner.

- **vakthund-cli:**
    A command‑line interface that unifies live capture and simulation modes. It provides a simple way to launch production or simulation environments, parse command‑line arguments, and initialize the underlying systems.

- **vakthund-config:**
    Provides a unified source for configuring all system components and ensures consistency and validation across the entire application.



---

## Build Instructions

To build the entire workspace in release mode, run:
```bash
cargo build --workspace --release
```

For running tests and benchmarks, use:
```bash
cargo test --workspace
cargo bench --workspace
```
