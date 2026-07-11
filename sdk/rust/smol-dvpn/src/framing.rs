// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Inner IP/UDP framing for two-hop nesting.
//!
//! In two-hop mode the exit tunnel's ciphertext must be delivered to the exit
//! gateway *through* the entry tunnel. We frame that ciphertext as an IPv4/UDP
//! datagram (src = tunnel addr : fixed exit client port, dst = exit endpoint)
//! and feed it to the entry `Tunn`, which encrypts it to the entry gateway. On
//! the way back the entry tunnel decrypts to recover this datagram, whose UDP
//! payload is the exit ciphertext. This is the pure-Rust equivalent of the
//! reference's in-gVisor UDP forwarder.

use std::net::{Ipv4Addr, SocketAddrV4};

use smoltcp::phy::ChecksumCapabilities;
use smoltcp::wire::{IpAddress, IpProtocol, Ipv4Packet, Ipv4Repr, UdpPacket, UdpRepr};

/// Build an IPv4/UDP datagram carrying `payload`.
pub fn build_ipv4_udp(src: SocketAddrV4, dst: SocketAddrV4, payload: &[u8]) -> Vec<u8> {
    let udp_repr = UdpRepr {
        src_port: src.port(),
        dst_port: dst.port(),
    };
    let ip_repr = Ipv4Repr {
        src_addr: *src.ip(),
        dst_addr: *dst.ip(),
        next_header: IpProtocol::Udp,
        payload_len: udp_repr.header_len() + payload.len(),
        hop_limit: 64,
    };

    let mut buf = vec![0u8; ip_repr.buffer_len() + ip_repr.payload_len];
    let mut ip_pkt = Ipv4Packet::new_unchecked(&mut buf);
    ip_repr.emit(&mut ip_pkt, &ChecksumCapabilities::default());

    let mut udp_pkt = UdpPacket::new_unchecked(ip_pkt.payload_mut());
    udp_repr.emit(
        &mut udp_pkt,
        &IpAddress::Ipv4(*src.ip()),
        &IpAddress::Ipv4(*dst.ip()),
        payload.len(),
        |b| b.copy_from_slice(payload),
        &ChecksumCapabilities::default(),
    );
    buf
}

/// Parsed inner datagram. On the return path the source confirms the frame came
/// from the expected exit endpoint.
pub struct ParsedUdp {
    pub src: SocketAddrV4,
    pub payload: Vec<u8>,
}

/// Parse an IPv4/UDP datagram. Returns `None` if it is not a well-formed
/// IPv4/UDP packet (e.g. an unexpected inner protocol).
pub fn parse_ipv4_udp(bytes: &[u8]) -> Option<ParsedUdp> {
    let ip_pkt = Ipv4Packet::new_checked(bytes).ok()?;
    if ip_pkt.next_header() != IpProtocol::Udp {
        return None;
    }
    let src_ip: Ipv4Addr = ip_pkt.src_addr();
    let udp_pkt = UdpPacket::new_checked(ip_pkt.payload()).ok()?;
    Some(ParsedUdp {
        src: SocketAddrV4::new(src_ip, udp_pkt.src_port()),
        payload: udp_pkt.payload().to_vec(),
    })
}
