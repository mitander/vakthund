//! Vakthund Capture
//!
//! Proprietary and confidential. All rights reserved.
//!
//! Provides a unified capture interface. Currently, only the simulation capture is implemented.

pub mod simulation_capture;

pub use simulation_capture::simulate_capture_loop;
