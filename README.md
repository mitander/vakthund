# Vakthund IDPS 🐶

**Vakthund** is a deterministic Intrusion Detection and Prevention System (IDPS) built for IoT products. The project is organized as a multi‑crate workspace that emphasizes clear module boundaries, zero‑copy data processing, and reproducible simulation for testing and debugging.

---

## Overview

The project is split into several crates:

- **vakthund-core:**
  Shared types and utilities (configuration, errors, logging, packets, and simulation logging). Foundation layer for event processing and resource management.

- **vakthund-capture:**
  Provides a unified capture interface for Vakthund (currently, live capture using pcap is implemented).

- **vakthund-protocols:**
  Implements protocol parsing (MQTT, COAP, etc.) using efficient, zero-copy parsers.

- **vakthund-detection:**
  Contains threat detection and analysis logic, including signature-based detection using Aho-Corasick.

- **vakthund-prevention:**
  Provides prevention mechanisms, currently including eBPF/XDP-based firewall capabilities.

- **vakthund-telemetry:**
  Handles telemetry and monitoring functionalities, including logging and metrics export via Prometheus.

- **vakthund-simulator:**
  Contains the deterministic simulation engine for reproducible testing and debugging.

- **vakthund-cli:**
  Provides a unified command-line interface for operating Vakthund in both live capture and simulation modes.

---

## Build Instructions

To build the entire workspace, run:

```bash
cargo build --workspace --release
