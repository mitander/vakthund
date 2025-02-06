/// A simple packet type used for capture.
/// In this example the packet contains only the raw data.
#[derive(Debug, Clone)]
pub struct Packet {
    pub data: Vec<u8>,
}

impl Packet {
    /// Creates a new Packet from raw data.
    pub fn new(data: Vec<u8>) -> Self {
        Packet { data }
    }
}
