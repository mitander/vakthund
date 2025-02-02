//! Event Bus Module
//!
//! Proprietary and confidential. All rights reserved.
//!
//! Implements an event‑based system using crossbeam‑channel. This module defines an
//! `Event` enum and an `EventBus` to decouple packet capture from processing.

use crossbeam_channel::{unbounded, Receiver, Sender};
use vakthund_common::packet::Packet;

/// Represents system events.
#[derive(Debug)]
pub enum Event {
    PacketCaptured(Packet),
    // Future events (e.g. Snapshot, Alert, etc.) can be added here.
}

/// A simple event bus built on an unbounded channel.
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
