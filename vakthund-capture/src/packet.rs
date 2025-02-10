/// A simple packet type used for capture.
use bytes::Bytes;

#[derive(Debug, Clone)]
pub struct Packet {
    pub data: Bytes,
}

impl Packet {
    /// Creates a new Packet from raw data.
    pub fn new(data: Vec<u8>) -> Self {
        // `Bytes::from` will take ownership of the Vec<u8>
        Packet {
            data: Bytes::from(data),
        }
    }
}
