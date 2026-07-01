#![allow(unused_imports)]

use crate::consts::*;

pub const SOCK_STREAM: u32 = 1;
pub const SOCK_DGRAM: u32 = 2;
pub const SOCK_RAW: u32 = 3;
pub const AF_INET: u32 = 2;
pub const AF_INET6: u32 = 10;
pub const AF_UNIX: u32 = 1;

pub enum SocketState {
    Closed,
    Listen,
    SynSent,
    SynRecvd,
    Established,
    FinWait1,
    FinWait2,
    TimeWait,
    CloseWait,
    LastAck,
    Closing,
}

// Refactor: this can be implemented completely in using other helper functions.
pub fn tcp_checksum(source_ip: u32, destination_ip: u32, payload: &[u8]) -> u16 {
    let mut checksum_data = build_pseudo_header(source_ip, destination_ip, 6, payload.len() as u16);
    checksum_data.extend_from_slice(payload);
    compute_inet_checksum(&checksum_data)
}

/// Parses the fixed fields needed by the network simulation from an IPv4 packet.
///
/// Returns source address, destination address, protocol, and total length.
pub fn parse_ipv4_header(packet: &[u8]) -> Option<(u32, u32, u8, u16)> {
    if packet.len() < 20 {
        return None;
    }

    let version = packet[0] >> 4;
    if version != 4 {
        return None;
    }

    let header_len = ((packet[0] & 0x0F) as usize).checked_mul(4)?;
    if header_len < 20 || packet.len() < header_len {
        return None;
    }

    let total_len = u16::from_be_bytes([packet[2], packet[3]]);
    let protocol = packet[9];
    let src_ip = u32::from_be_bytes([packet[12], packet[13], packet[14], packet[15]]);
    let dst_ip = u32::from_be_bytes([packet[16], packet[17], packet[18], packet[19]]);
    Some((src_ip, dst_ip, protocol, total_len))
}

/// Builds the 12-byte IPv4 pseudo-header used by TCP/UDP checksums.
pub fn build_pseudo_header(src_ip: u32, dst_ip: u32, protocol: u8, length: u16) -> Vec<u8> {
    let mut header = Vec::with_capacity(12);
    header.extend_from_slice(&src_ip.to_be_bytes());
    header.extend_from_slice(&dst_ip.to_be_bytes());
    header.push(0);
    header.push(protocol);
    header.extend_from_slice(&length.to_be_bytes());
    header
}

/// Computes the standard one's-complement Internet checksum.
pub fn compute_inet_checksum(data: &[u8]) -> u16 {
    let mut sum: u32 = 0;
    let mut chunks = data.chunks_exact(2);
    for chunk in &mut chunks {
        sum += u16::from_be_bytes([chunk[0], chunk[1]]) as u32;
    }
    if let Some(&last_byte) = chunks.remainder().first() {
        sum += (last_byte as u32) << 8;
    }

    while sum > 0xFFFF {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }
    !sum as u16
}
