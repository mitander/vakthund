mod config;
mod packet_capture;
mod packet_parser;

use anyhow::Result;
use config::Settings;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let settings = Settings::new()?;
    info!("Loaded config: {:?}", settings);

    let (tx, mut rx) = tokio::sync::mpsc::channel(1024);
    let capture_task = tokio::spawn(async move { packet_capture::start(settings, tx).await });

    while let Some(raw_data) = rx.recv().await {
        match packet_parser::parse(&raw_data) {
            Ok(packet) if packet.is_mqtt() => {
                info!(
                    "MQTT packet: {}:{} -> {}:{} ({} bytes)",
                    packet.src_ip,
                    packet.src_port,
                    packet.dst_ip,
                    packet.dst_port,
                    packet.payload.len()
                );
            }
            Err(e) => tracing::error!("Parse error: {}", e),
            _ => {}
        }
    }

    capture_task.await??;
    Ok(())
}
