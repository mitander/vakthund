use anyhow::Result;
use pcap::Device;
use tokio::sync::mpsc;

pub async fn start(settings: crate::Settings, tx: mpsc::Sender<Vec<u8>>) -> Result<()> {
    let device = Device::from(settings.interface.as_str());
    let mut cap = pcap::Capture::from_device(device)?
        .promisc(true)
        .timeout(0) // No timeout (block indefinitely)
        .open()?
        .setnonblock()?;

    tokio::task::spawn_blocking(move || {
        loop {
            match cap.next_packet() {
                Ok(packet) => {
                    let data = packet.data.to_vec();
                    if let Err(e) = tx.blocking_send(data) {
                        tracing::error!("Channel error: {}", e);
                        break;
                    }
                }
                Err(pcap::Error::TimeoutExpired) => continue, // Handle macOS-specific timeout
                Err(e) => {
                    tracing::error!("Capture error: {}", e);
                    break;
                }
            }
        }
    });

    Ok(())
}
