use std::net::{Ipv4Addr, Ipv6Addr};

use bytes::Bytes;
use nym_connection_monitor::{
    ConnectionStatusEvent, IcmpBeaconReply, Icmpv6BeaconReply, is_icmp_beacon_reply,
    is_icmp_v6_beacon_reply,
    packet_helpers::{
        create_icmpv4_echo_request, create_icmpv6_echo_request, wrap_icmp_in_ipv4,
        wrap_icmp_in_ipv6,
    },
};
use nym_ip_packet_requests::{IpPair, codec::MultiIpPacketCodec, v8::request::IpPacketRequest};
use nym_sdk::mixnet::{
    InputMessage, MixnetClient, MixnetMessageSender, Recipient, TransmissionLane,
};
use pnet_packet::Packet;

pub fn icmp_identifier() -> u16 {
    8475
}

pub async fn send_ping_v4(
    mixnet_client: &MixnetClient,
    our_ips: IpPair,
    sequence_number: u16,
    destination: Ipv4Addr,
    exit_router_address: Recipient,
) -> anyhow::Result<()> {
    let icmp_identifier = icmp_identifier();
    let icmp_echo_request = create_icmpv4_echo_request(sequence_number, icmp_identifier)?;
    let ipv4_packet = wrap_icmp_in_ipv4(icmp_echo_request, our_ips.ipv4, destination)?;

    // Wrap the IPv4 packet in a MultiIpPacket
    let bundled_packet =
        MultiIpPacketCodec::bundle_one_packet(ipv4_packet.packet().to_vec().into());

    // Wrap into a mixnet input message addressed to the IPR
    let mixnet_message = create_input_message(exit_router_address, bundled_packet)?;

    mixnet_client.send(mixnet_message).await?;
    Ok(())
}

pub async fn send_ping_v6(
    mixnet_client: &MixnetClient,
    our_ips: IpPair,
    sequence_number: u16,
    destination: Ipv6Addr,
    exit_router_address: Recipient,
) -> anyhow::Result<()> {
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

    // Wrap into a mixnet input message addressed to the IPR
    let mixnet_message = create_input_message(exit_router_address, bundled_packet)?;

    // Send across the mixnet
    mixnet_client.send(mixnet_message).await?;
    Ok(())
}

fn create_input_message(
    recipient: impl Into<Recipient>,
    bundled_packets: Bytes,
) -> anyhow::Result<InputMessage> {
    let packet = IpPacketRequest::new_data_request(bundled_packets).to_bytes()?;

    let lane = TransmissionLane::General;
    let packet_type = None;
    let surbs = 0;
    Ok(InputMessage::new_anonymous(
        recipient.into(),
        packet,
        surbs,
        lane,
        packet_type,
    ))
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
