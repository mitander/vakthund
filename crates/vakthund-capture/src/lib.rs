//! Vakthund Capture
//!
//! Proprietary and confidential. All rights reserved.
//!
//! Provides a unified capture interface. Currently, only the simulation capture is implemented.

pub mod live_capture;

pub use live_capture::live_capture_loop;
