# Vakthund IDPS 🐶

**Vakthund** is a deterministic Intrusion Detection and Prevention System (IDPS) built for IoT products. The project is organized as a multi‑crate workspace that emphasizes clear module boundaries, zero‑copy data processing, and reproducible simulation for testing and debugging.

---

## Overview

The project is split into several crates:

- **vakthund-common:**
  Shared types and utilities (configuration, errors, logging, packets, and simulation logging).

- **vakthund-capture:**
  Provides a unified capture interface (currently, simulation capture is implemented).

- **vakthund-protocol:**
  Implements protocol parsing (MQTT, COAP, etc.) using enums to avoid magic strings.

- **vakthund-detection:**
  Contains threat detection and analysis logic.

- **vakthund-monitor:**
  Monitors network traffic, applies quarantine, and enforces traffic thresholds.

- **vakthund-core:**
  Integrates all components into the main IDPS pipeline.

- **vakthund-simulation:**
  Contains the deterministic simulation engine and storage for simulation events, isolated from production code for reproducible testing.

---

## Build Instructions

To build the entire workspace, run:

```bash
cargo build --workspace
```

To run the Vakthund application (typically provided by the vakthund-core binary), use:

```bash
sudo ./target/debug/vakthund
```

## Simulation Mode

When running in simulation mode (as specified in the configuration), Vakthund uses a deterministic simulation engine that generates network packet events based on a fixed seed. This ensures that the same sequence of events is produced every time, enabling reproducible testing and debugging.
How It Works

- Deterministic Events:
    A seeded RNG generates events, each assigned an event ID and a computed SHA‑256 hash for traceability.

- Bug Injection:
    A bug is intentionally injected at event ID 3 (by generating a malformed packet).

- Structured Logging:
    JSON‑formatted logs are written to simulation_logs/simulation_<seed>.log with contextual metadata (seed, event ID, timestamp, etc.).

- Reproducibility:
    Running the simulation with the same seed (for example, 42) produces an identical event sequence. If a parsing error occurs, a bug report is generated in the bug_reports/ folder with details necessary for replay.

<div style="background-color: #E7F3FE; border-left: 4px solid #2196F3; padding: 8px; margin: 8px 0;">
  <strong>Note:</strong> To reproduce a bug, use the same seed as in the bug report. The bug report includes the event ID (e.g., event ID 3) and all necessary metadata to replay that event.
</div>

## Reproducing Bugs

When a packet fails to parse (e.g., due to the injected bug), a bug report is automatically generated in the bug_reports/ folder. Each bug report contains:

- Timestamp
- Configuration and simulation seed
- Event ID and packet content
- Error message

## To replay a bug:

Note the seed and event ID from the bug report.
Set the replay target in your configuration (or via a command‑line override) to that event ID.
Re-run the simulation. The engine will stop at the specified event, enabling you to debug the issue deterministically.

<div style="background-color: #FFCDD2; border-left: 4px solid #F44336; padding: 8px; margin: 8px 0;">
  <strong>Warning:</strong> Always run the simulation with the same seed as noted in the bug report to ensure deterministic replay.
</div>
