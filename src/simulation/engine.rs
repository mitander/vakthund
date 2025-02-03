//! Simulation engine core using Xoshiro256++ RNG and SHA-256 checksums.
//!
use rand::{RngCore, SeedableRng};
use rand_xoshiro::Xoshiro256PlusPlus;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone)]
pub struct SimEvent {
    pub data: [u8; 256],
    pub checksum: [u8; 32],
}

pub struct SimulationEngine {
    rng: Xoshiro256PlusPlus,
    pub checksums: Vec<[u8; 32]>,
}

impl SimulationEngine {
    pub fn new(seed: u64) -> Self {
        Self {
            rng: Xoshiro256PlusPlus::seed_from_u64(seed),
            checksums: Vec::new(),
        }
    }

    #[inline(always)]
    pub fn next_event(&mut self) -> SimEvent {
        let mut data = [0u8; 256];
        self.rng.fill_bytes(&mut data);
        let checksum = Self::compute_checksum(&data);
        self.checksums.push(checksum);
        SimEvent { data, checksum }
    }

    #[inline(always)]
    fn compute_checksum(data: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.finalize().into()
    }

    pub fn verify(&self) -> bool {
        if let Some(last) = self.checksums.last() {
            let mut hasher = Sha256::new();
            for cs in &self.checksums {
                hasher.update(cs);
            }
            hasher.finalize().as_slice() == last
        } else {
            false
        }
    }
}

pub fn start(seed: u64, tx: crossbeam_channel::Sender<crate::message_bus::Event>) {
    std::thread::spawn(move || {
        let mut engine = SimulationEngine::new(seed);
        loop {
            let event = engine.next_event();
            let data = bytes::Bytes::copy_from_slice(&event.data);
            let timestamp = now_ns();
            if let Err(e) = tx.send(crate::message_bus::Event::Packet { timestamp, data }) {
                eprintln!("Simulation send error: {:?}", e);
            }
        }
    });
}

#[inline(always)]
fn now_ns() -> u64 {
    unsafe {
        let mut ts = std::mem::MaybeUninit::uninit();
        libc::clock_gettime(libc::CLOCK_MONOTONIC, ts.as_mut_ptr());
        let ts = ts.assume_init();
        (ts.tv_sec as u64) * 1_000_000_000 + (ts.tv_nsec as u64)
    }
}
