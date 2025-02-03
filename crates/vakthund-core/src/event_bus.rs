//! Event Bus Module
//!
//! Proprietary and confidential. All rights reserved.
//!
//! Implements a unified event‑based system using crossbeam‑channel. The Event enum
//! supports a unified stream of events from various sources (live capture, simulation, etc.).
//! Additional event types (AlertRaised, SnapshotTaken, PreventionAction) are defined here
//! for future extension.

use crossbeam_channel::{unbounded, Receiver, Sender};

use vakthund_common::packet::Packet;

#[derive(Debug)]
pub enum Event {
    /// A packet was captured.
    PacketCaptured(Packet),
    /// An alert event, e.g., when a threat is detected.
    AlertRaised { details: String, packet: Packet },
    /// A snapshot event, capturing system state.
    SnapshotTaken { snapshot_data: String },
    /// A prevention action event, e.g., to trigger active mitigation.
    PreventionAction { action: String, packet: Packet },
}

pub struct EventBus {
    sender: Sender<Event>,
    receiver: Receiver<Event>,
}

impl EventBus {
    /// Creates a new EventBus.
    pub fn new() -> Self {
        let (sender, receiver) = unbounded();
        Self { sender, receiver }
    }

    /// Returns a clone of the event sender.
    pub fn get_sender(&self) -> Sender<Event> {
        self.sender.clone()
    }

    /// Returns a clone of the event receiver.
    pub fn get_receiver(&self) -> Receiver<Event> {
        self.receiver.clone()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}
