//! Simulation replay submodule.
use crate::message_bus::Event;
use anyhow::Result;
use bytes::Bytes;
use crossbeam_channel::Sender;
use memmap::Mmap;
use std::fs::File;
use std::path::Path;

pub struct ReplayHandle {
    mmap: Mmap,
    position: usize,
}

impl ReplayHandle {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };
        Ok(Self { mmap, position: 0 })
    }

    pub fn next_event(&mut self) -> Option<Event> {
        let event_size = 256 + 32;
        if self.position + event_size > self.mmap.len() {
            return None;
        }
        let slice = &self.mmap[self.position..self.position + event_size];
        self.position += event_size;
        let data = Bytes::copy_from_slice(&slice[..256]);
        let timestamp = now_ns();
        Some(Event::Packet { timestamp, data })
    }
}

pub fn start<P: AsRef<std::path::Path> + Send + 'static>(path: P, tx: Sender<Event>) {
    std::thread::spawn(move || {
        let mut handle = ReplayHandle::open(path).expect("Failed to open replay file");
        while let Some(event) = handle.next_event() {
            if let Err(e) = tx.send(event) {
                eprintln!("Replay send error: {:?}", e);
                break;
            }
        }
        eprintln!("Replay finished. No more events to replay.");
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
