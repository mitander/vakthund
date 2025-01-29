use anyhow::{Context, Result};
use etherparse::{PacketHeaders, TransportHeader};
use std::net::Ipv4Addr;

#[derive(Debug)]
pub struct ParsedPacket {
    pub src_mac: String,
    pub dst_mac: String,
    pub src_ip: String,
    pub dst_ip: String,
    pub src_port: u16,
    pub dst_port: u16,
    pub payload: Vec<u8>,
}

impl ParsedPacket {
    pub fn is_mqtt(&self) -> bool {
        self.dst_port == 1883 || self.src_port == 1883
    }
}

pub fn parse(raw: &[u8]) -> Result<ParsedPacket> {
    let headers =
        PacketHeaders::from_ethernet_slice(raw).context("Failed to parse packet headers")?;

    let eth = match headers.link {
        Some(etherparse::LinkHeader::Ethernet2(eth)) => eth,
        _ => anyhow::bail!("Not an Ethernet packet"),
    };

    let (src_ip, dst_ip) = match headers.net {
        Some(etherparse::NetHeaders::Ipv4(ipv4, _)) => (
            Ipv4Addr::from(ipv4.source).to_string(),
            Ipv4Addr::from(ipv4.destination).to_string(),
        ),
        _ => anyhow::bail!("IPv6 not supported yet"),
    };

    let (src_port, dst_port) = match headers.transport {
        Some(TransportHeader::Tcp(tcp)) => (tcp.source_port, tcp.destination_port),
        Some(TransportHeader::Udp(udp)) => (udp.source_port, udp.destination_port),
        _ => (0, 0),
    };

    Ok(ParsedPacket {
        src_mac: format!("{:x?}", eth.source),
        dst_mac: format!("{:x?}", eth.destination),
        src_ip,
        dst_ip,
        src_port,
        dst_port,
        payload: headers.payload.slice().to_vec(), // CORRECTED CONVERSION
    })
}
