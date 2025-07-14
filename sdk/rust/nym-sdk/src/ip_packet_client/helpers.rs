// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::error::{Error, Result};
use crate::ip_packet_client::current::VERSION as CURRENT_VERSION;
pub use crate::mixnet::ReconstructedMessage;
use nym_config::defaults::mixnet_vpn::{NYM_TUN_DEVICE_ADDRESS_V4, NYM_TUN_DEVICE_ADDRESS_V6};

use crate::stream_wrapper::IpMixStream;

use nym_ip_packet_requests::{codec::MultiIpPacketCodec, IpPair};

use bytes::Bytes;
use pnet_packet::{
    icmp::{
        echo_reply::EchoReplyPacket,
        echo_request::{EchoRequestPacket, MutableEchoRequestPacket},
        IcmpPacket,
    },
    icmpv6,
    ipv4::{Ipv4Packet, MutableIpv4Packet},
    ipv6::{Ipv6Packet, MutableIpv6Packet},
    Packet,
};
use std::cmp::Ordering;
use std::net::{Ipv4Addr, Ipv6Addr};

/**
 * This function is from the original nym-ip-packet-client crate.
 */
pub(crate) fn check_ipr_message_version(message: &ReconstructedMessage) -> Result<()> {
    // Assuming it's a IPR message, it will have a version as its first byte
    if let Some(version) = message.message.first() {
        match version.cmp(&CURRENT_VERSION) {
            Ordering::Greater => Err(Error::ReceivedResponseWithNewVersion {
                expected: CURRENT_VERSION,
                received: *version,
            }),
            Ordering::Less => Err(Error::ReceivedResponseWithOldVersion {
                expected: CURRENT_VERSION,
                received: *version,
            }),
            Ordering::Equal => {
                // We're good
                Ok(())
            }
        }
    } else {
        Err(Error::NoVersionInMessage)
    }
}

/**
 * Functions below are from the nym-connection-monitor crate.
 */
pub fn create_icmpv4_echo_request(
    sequence_number: u16,
    identifier: u16,
) -> Result<EchoRequestPacket<'static>> {
    let buffer = vec![0; 64];
    let mut icmp_echo_request = MutableEchoRequestPacket::owned(buffer)
        .ok_or(Error::IcmpEchoRequestPacketCreationFailure)?;

    // Configure the ICMP echo request packet
    icmp_echo_request.set_identifier(identifier);
    icmp_echo_request.set_sequence_number(sequence_number);
    icmp_echo_request.set_icmp_type(pnet_packet::icmp::IcmpTypes::EchoRequest);
    icmp_echo_request.set_icmp_code(pnet_packet::icmp::IcmpCode::new(0));

    // Calculate checksum once we've set all the fields
    let icmp_packet =
        IcmpPacket::new(icmp_echo_request.packet()).ok_or(Error::IcmpPacketCreationFailure)?;
    let checksum = pnet_packet::icmp::checksum(&icmp_packet);
    icmp_echo_request.set_checksum(checksum);

    Ok(icmp_echo_request.consume_to_immutable())
}

pub fn create_icmpv6_echo_request(
    sequence_number: u16,
    identifier: u16,
    source: &Ipv6Addr,
    destination: &Ipv6Addr,
) -> Result<icmpv6::echo_request::EchoRequestPacket<'static>> {
    let buffer = vec![0; 64];
    // let mut icmp_echo_request = MutableEchoRequestPacket::owned(buffer)
    let mut icmp_echo_request = icmpv6::echo_request::MutableEchoRequestPacket::owned(buffer)
        .ok_or(Error::IcmpEchoRequestPacketCreationFailure)?;

    // Configure the ICMP echo request packet
    icmp_echo_request.set_identifier(identifier);
    icmp_echo_request.set_sequence_number(sequence_number);
    icmp_echo_request.set_icmpv6_type(pnet_packet::icmpv6::Icmpv6Types::EchoRequest);
    icmp_echo_request.set_icmpv6_code(pnet_packet::icmpv6::Icmpv6Code::new(0));

    // Calculate checksum once we've set all the fields
    let icmp_packet = icmpv6::Icmpv6Packet::new(icmp_echo_request.packet())
        .ok_or(Error::IcmpPacketCreationFailure)?;
    let checksum = pnet_packet::icmpv6::checksum(&icmp_packet, source, destination);
    icmp_echo_request.set_checksum(checksum);

    Ok(icmp_echo_request.consume_to_immutable())
}

pub fn wrap_icmp_in_ipv4(
    icmp_echo_request: EchoRequestPacket,
    source: Ipv4Addr,
    destination: Ipv4Addr,
) -> Result<Ipv4Packet> {
    // 20 bytes for IPv4 header + ICMP payload
    let total_length = 20 + icmp_echo_request.packet().len();
    // IPv4 header + ICMP payload
    let ipv4_buffer = vec![0u8; 20 + icmp_echo_request.packet().len()];
    let mut ipv4_packet =
        MutableIpv4Packet::owned(ipv4_buffer).ok_or(Error::Ipv4PacketCreationFailure)?;

    ipv4_packet.set_version(4);
    ipv4_packet.set_header_length(5);
    ipv4_packet.set_total_length(total_length as u16);
    ipv4_packet.set_ttl(64);
    ipv4_packet.set_next_level_protocol(pnet_packet::ip::IpNextHeaderProtocols::Icmp);
    ipv4_packet.set_source(source);
    ipv4_packet.set_destination(destination);
    ipv4_packet.set_flags(pnet_packet::ipv4::Ipv4Flags::DontFragment);
    ipv4_packet.set_checksum(0);
    ipv4_packet.set_payload(icmp_echo_request.packet());

    let ipv4_checksum = compute_ipv4_checksum(&ipv4_packet.to_immutable());
    ipv4_packet.set_checksum(ipv4_checksum);

    Ok(ipv4_packet.consume_to_immutable())
}

pub fn wrap_icmp_in_ipv6(
    icmp_echo_request: icmpv6::echo_request::EchoRequestPacket,
    source: Ipv6Addr,
    destination: Ipv6Addr,
) -> Result<Ipv6Packet> {
    let ipv6_buffer = vec![0u8; 40 + icmp_echo_request.packet().len()];
    let mut ipv6_packet =
        MutableIpv6Packet::owned(ipv6_buffer).ok_or(Error::Ipv4PacketCreationFailure)?;

    ipv6_packet.set_version(6);
    ipv6_packet.set_payload_length(icmp_echo_request.packet().len() as u16);
    ipv6_packet.set_next_header(pnet_packet::ip::IpNextHeaderProtocols::Icmpv6);
    ipv6_packet.set_hop_limit(64);
    ipv6_packet.set_source(source);
    ipv6_packet.set_destination(destination);
    ipv6_packet.set_payload(icmp_echo_request.packet());

    Ok(ipv6_packet.consume_to_immutable())
}

// Compute IPv4 checksum: sum all 16-bit words, add carry, take one's complement
pub(crate) fn compute_ipv4_checksum(header: &Ipv4Packet) -> u16 {
    // Header length in 16-bit words
    let len = header.get_header_length() as usize * 2;
    let mut sum = 0u32;

    for i in 0..len {
        let word = ((header.packet()[2 * i] as u32) << 8) | header.packet()[2 * i + 1] as u32;
        sum += word;
    }

    // Add the carry
    while (sum >> 16) > 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }

    // One's complement
    !sum as u16
}

pub(crate) fn is_icmp_echo_reply(packet: &Bytes) -> Option<(u16, Ipv4Addr, Ipv4Addr)> {
    if let Some(ipv4_packet) = Ipv4Packet::new(packet) {
        if let Some(icmp_packet) = IcmpPacket::new(ipv4_packet.payload()) {
            if let Some(echo_reply) = EchoReplyPacket::new(icmp_packet.packet()) {
                return Some((
                    echo_reply.get_identifier(),
                    ipv4_packet.get_source(),
                    ipv4_packet.get_destination(),
                ));
            }
        }
    }
    None
}

pub(crate) fn is_icmp_v6_echo_reply(packet: &Bytes) -> Option<(u16, Ipv6Addr, Ipv6Addr)> {
    if let Some(ipv6_packet) = Ipv6Packet::new(packet) {
        if let Some(icmp_packet) = IcmpPacket::new(ipv6_packet.payload()) {
            if let Some(echo_reply) =
                pnet_packet::icmpv6::echo_reply::EchoReplyPacket::new(icmp_packet.packet())
            {
                return Some((
                    echo_reply.get_identifier(),
                    ipv6_packet.get_source(),
                    ipv6_packet.get_destination(),
                ));
            }
        }
    }
    None
}

/**
 * Types and functions below are from the nym-connection-monitor crate.
 * The `send_ping_v4` + `_v6` functions have been modified to work with the IPMixStream wrapper instead of relying on a shared MixnetClient.
 */
#[derive(Debug)]
pub enum ConnectionStatusEvent {
    MixnetSelfPing,
    Icmpv4IprTunDevicePingReply,
    Icmpv6IprTunDevicePingReply,
    Icmpv4IprExternalPingReply,
    Icmpv6IprExternalPingReply,
}

#[derive(Debug, Clone, Default)]
pub struct IpPingReplies {
    pub ipr_tun_ip_v4: bool,
    pub ipr_tun_ip_v6: bool,
    pub external_ip_v4: bool,
    pub external_ip_v6: bool,
}

impl IpPingReplies {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_event(&mut self, event: &ConnectionStatusEvent) {
        match event {
            ConnectionStatusEvent::MixnetSelfPing => {}
            ConnectionStatusEvent::Icmpv4IprTunDevicePingReply => self.ipr_tun_ip_v4 = true,
            ConnectionStatusEvent::Icmpv6IprTunDevicePingReply => self.ipr_tun_ip_v6 = true,
            ConnectionStatusEvent::Icmpv4IprExternalPingReply => self.external_ip_v4 = true,
            ConnectionStatusEvent::Icmpv6IprExternalPingReply => self.external_ip_v6 = true,
        }
    }
}

pub enum IcmpBeaconReply {
    TunDeviceReply,
    ExternalPingReply(Ipv4Addr),
}

pub enum Icmpv6BeaconReply {
    TunDeviceReply,
    ExternalPingReply(Ipv6Addr),
}

pub fn icmp_identifier() -> u16 {
    8475
}

// The only real change here is that we don't have to use the wrap() function to turn the incoming data in an InputMessage as this is done by the stream abstraction's write_bytes() via `send_ip_packet()`.
pub async fn send_ping_v4(
    stream: &mut IpMixStream,
    our_ips: &IpPair,
    sequence_number: u16,
    identifier: u16,
    destination: Ipv4Addr,
) -> Result<()> {
    let icmp_echo_request = create_icmpv4_echo_request(sequence_number, identifier)?;
    let ipv4_packet = wrap_icmp_in_ipv4(icmp_echo_request, our_ips.ipv4, destination)?;

    let bundled_packet =
        MultiIpPacketCodec::bundle_one_packet(ipv4_packet.packet().to_vec().into());

    stream.send_ip_packet(&bundled_packet).await?;
    Ok(())
}

// One difference to note here is that since the IPR address is part of the stream (stored on connection) we don't have to pass it manually as in the original version of this code. The other diff is the same as the v4 function above re: not having to wrap the incoming in an InputMessage manually.
pub async fn send_ping_v6(
    stream: &mut IpMixStream,
    our_ips: &IpPair,
    sequence_number: u16,
    destination: Ipv6Addr,
) -> Result<()> {
    let icmp_identifier = icmp_identifier();
    let icmp_echo_request = create_icmpv6_echo_request(
        sequence_number,
        icmp_identifier,
        &our_ips.ipv6,
        &destination,
    )?;
    let ipv6_packet = wrap_icmp_in_ipv6(icmp_echo_request, our_ips.ipv6, destination)?;

    // Wrap the IPv6 packet in a MultiIpPacket
    let bundled_packet =
        MultiIpPacketCodec::bundle_one_packet(ipv6_packet.packet().to_vec().into());

    stream.send_ip_packet(&bundled_packet).await?;

    Ok(())
}

pub fn check_for_icmp_beacon_reply(
    packet: &Bytes,
    icmp_beacon_identifier: u16,
    our_ips: IpPair,
) -> Option<ConnectionStatusEvent> {
    match is_icmp_beacon_reply(packet, icmp_beacon_identifier, our_ips.ipv4) {
        Some(IcmpBeaconReply::TunDeviceReply) => {
            tracing::debug!("Received ping response from ipr tun device");
            return Some(ConnectionStatusEvent::Icmpv4IprTunDevicePingReply);
        }
        Some(IcmpBeaconReply::ExternalPingReply(_source)) => {
            tracing::debug!("Received ping response from an external ip through the ipr");
            return Some(ConnectionStatusEvent::Icmpv4IprExternalPingReply);
        }
        None => {}
    }

    match is_icmp_v6_beacon_reply(packet, icmp_beacon_identifier, our_ips.ipv6) {
        Some(Icmpv6BeaconReply::TunDeviceReply) => {
            tracing::debug!("Received ping v6 response from ipr tun device");
            return Some(ConnectionStatusEvent::Icmpv6IprTunDevicePingReply);
        }
        Some(Icmpv6BeaconReply::ExternalPingReply(_source)) => {
            tracing::debug!("Received ping v6 response from an external ip through the ipr");
            return Some(ConnectionStatusEvent::Icmpv6IprExternalPingReply);
        }
        None => {}
    }

    None
}

pub fn is_icmp_beacon_reply(
    packet: &Bytes,
    identifier: u16,
    destination: Ipv4Addr,
) -> Option<IcmpBeaconReply> {
    if let Some((reply_identifier, reply_source, reply_destination)) = is_icmp_echo_reply(packet) {
        if reply_identifier == identifier && reply_destination == destination {
            if reply_source == NYM_TUN_DEVICE_ADDRESS_V4 {
                return Some(IcmpBeaconReply::TunDeviceReply);
            } else {
                // For external replies, we check if the source is NOT the TUN device
                // and NOT our own IP (since external hosts reply from their own IPs)
                return Some(IcmpBeaconReply::ExternalPingReply(reply_source));
            }
        }
    }
    None
}

pub fn is_icmp_v6_beacon_reply(
    packet: &Bytes,
    identifier: u16,
    destination: Ipv6Addr,
) -> Option<Icmpv6BeaconReply> {
    if let Some((reply_identifier, reply_source, reply_destination)) = is_icmp_v6_echo_reply(packet)
    {
        if reply_identifier == identifier && reply_destination == destination {
            if reply_source == NYM_TUN_DEVICE_ADDRESS_V6 {
                return Some(Icmpv6BeaconReply::TunDeviceReply);
            } else {
                // For external replies, check if source is NOT the TUN device
                return Some(Icmpv6BeaconReply::ExternalPingReply(reply_source));
            }
        }
    }
    None
}
