// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use std::{
    net::{Ipv4Addr, Ipv6Addr},
    time::Duration,
};

use bytes::Bytes;
use nym_common::trace_err_chain;
use nym_config::defaults::mixnet_vpn::{NYM_TUN_DEVICE_ADDRESS_V4, NYM_TUN_DEVICE_ADDRESS_V6};
use nym_ip_packet_requests::{IpPair, codec::MultiIpPacketCodec};
use nym_sdk::mixnet::{
    InputMessage, MixnetClientSender, MixnetMessageSender, Recipient, TransmissionLane,
};
use pnet_packet::Packet;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{debug, trace};

use crate::{
    Error,
    error::Result,
    nym_ip_packet_requests_current::request::IpPacketRequest,
    packet_helpers::{
        create_icmpv4_echo_request, create_icmpv6_echo_request, is_icmp_echo_reply,
        is_icmp_v6_echo_reply, wrap_icmp_in_ipv4, wrap_icmp_in_ipv6,
    },
};

const ICMP_BEACON_PING_INTERVAL: Duration = Duration::from_millis(1000);

// This can be anything really, we just want to check if the exit IPR can reach the internet
// TODO: have a pool of IPs to ping
const ICMP_IPR_TUN_EXTERNAL_PING_V4: Ipv4Addr = Ipv4Addr::new(8, 8, 8, 8);
const ICMP_IPR_TUN_EXTERNAL_PING_V6: Ipv6Addr =
    Ipv6Addr::new(0x2001, 0x4860, 0x4860, 0, 0, 0, 0, 0x8888);

struct IcmpConnectionBeacon {
    mixnet_client_sender: MixnetClientSender,
    our_ips: IpPair,
    ipr_address: Recipient,
    sequence_number: u16,
    icmp_identifier: u16,
}

impl IcmpConnectionBeacon {
    fn new(
        mixnet_client_sender: MixnetClientSender,
        our_ips: IpPair,
        ipr_address: Recipient,
        icmp_identifier: u16,
    ) -> Self {
        IcmpConnectionBeacon {
            mixnet_client_sender,
            our_ips,
            ipr_address,
            sequence_number: 0,
            icmp_identifier,
        }
    }

    fn get_next_sequence_number(&mut self) -> u16 {
        let sequence_number = self.sequence_number;
        self.sequence_number = self.sequence_number.wrapping_add(1);
        sequence_number
    }

    async fn send_icmp_v4_ping(&mut self, destination: Ipv4Addr) -> Result<()> {
        // Create ICMP/IPv4 echo request packet
        let sequence_number = self.get_next_sequence_number();
        let identifier = self.icmp_identifier;
        let icmp_echo_request = create_icmpv4_echo_request(sequence_number, identifier)?;
        let ipv4_packet = wrap_icmp_in_ipv4(icmp_echo_request, self.our_ips.ipv4, destination)?;

        // Wrap the IPv4 packet in a MultiIpPacket
        let bundled_packet =
            MultiIpPacketCodec::bundle_one_packet(ipv4_packet.packet().to_vec().into());

        // Wrap into a mixnet input message addressed to the IPR
        let mixnet_message = wrap_in_mixnet_message(self.ipr_address, bundled_packet)?;

        // Send across the mixnet
        self.mixnet_client_sender
            .send(mixnet_message)
            .await
            .map_err(|err| Error::NymSdkError(Box::new(err)))
    }

    async fn send_icmp_v6_ping(&mut self, destination: Ipv6Addr) -> Result<()> {
        // Create ICMP/IPv6 echo request packet
        let sequence_number = self.get_next_sequence_number();
        let identifier = self.icmp_identifier;
        let icmp_echo_request = create_icmpv6_echo_request(
            sequence_number,
            identifier,
            &self.our_ips.ipv6,
            &destination,
        )?;
        let ipv6_packet = wrap_icmp_in_ipv6(icmp_echo_request, self.our_ips.ipv6, destination)?;

        // Wrap the IPv6 packet in a MultiIpPacket
        let bundled_packet =
            MultiIpPacketCodec::bundle_one_packet(ipv6_packet.packet().to_vec().into());

        // Wrap into a mixnet input message addressed to the IPR
        let mixnet_message = wrap_in_mixnet_message(self.ipr_address, bundled_packet)?;

        // Send across the mixnet
        self.mixnet_client_sender
            .send(mixnet_message)
            .await
            .map_err(|err| Error::NymSdkError(Box::new(err)))
    }

    async fn ping_v4_ipr_tun_device_over_the_mixnet(&mut self) -> Result<()> {
        self.send_icmp_v4_ping(NYM_TUN_DEVICE_ADDRESS_V4).await
    }

    async fn ping_v6_ipr_tun_device_over_the_mixnet(&mut self) -> Result<()> {
        self.send_icmp_v6_ping(NYM_TUN_DEVICE_ADDRESS_V6).await
    }

    async fn ping_v4_some_external_ip_over_the_mixnet(&mut self) -> Result<()> {
        // TODO: ramdon external IP from a pool
        self.send_icmp_v4_ping(ICMP_IPR_TUN_EXTERNAL_PING_V4).await
    }

    async fn ping_v6_some_external_ip_over_the_mixnet(&mut self) -> Result<()> {
        // TODO: ramdon external IP from a pool
        self.send_icmp_v6_ping(ICMP_IPR_TUN_EXTERNAL_PING_V6).await
    }

    pub async fn run(mut self, shutdown: CancellationToken) -> Result<()> {
        debug!("Icmp connection beacon is running");
        let mut ping_interval = tokio::time::interval(ICMP_BEACON_PING_INTERVAL);
        while !shutdown.is_cancelled() {
            tokio::select! {
                _ = shutdown.cancelled() => {
                    trace!("IcmpConnectionBeacon: Received shutdown");
                    break;
                }
                _ = ping_interval.tick() => {
                    let cancellable_fut = async {
                        if let Err(err) = self.ping_v4_ipr_tun_device_over_the_mixnet().await {
                            trace_err_chain!(err, "Failed to send ICMP ping");
                        }
                        if let Err(err) = self.ping_v6_ipr_tun_device_over_the_mixnet().await {
                            trace_err_chain!(err, "Failed to send ICMPv6 ping");
                        }
                        if let Err(err) = self.ping_v4_some_external_ip_over_the_mixnet().await {
                            trace_err_chain!(err, "Failed to send ICMP ping");
                        }
                        if let Err(err) = self.ping_v6_some_external_ip_over_the_mixnet().await {
                            trace_err_chain!(err, "Failed to send ICMPv6 ping");
                        }
                    };

                    tokio::select! {
                        _ = cancellable_fut => {
                            continue;
                        },
                        _ = shutdown.cancelled() => {
                            trace!("IcmpConnectionBeacon: Received shutdown");
                            break;
                        }
                    }
                }
            }
        }
        debug!("IcmpConnectionBeacon: Exiting");
        Ok(())
    }
}

fn wrap_in_mixnet_message(recipient: Recipient, bundled_packets: Bytes) -> Result<InputMessage> {
    let packet = IpPacketRequest::new_data_request(bundled_packets).to_bytes()?;
    let surbs = 0;
    let mixnet_message = nym_sdk::mixnet::InputMessage::new_anonymous(
        recipient,
        packet,
        surbs,
        TransmissionLane::General,
        None,
    )
    .with_max_retransmissions(0);
    Ok(mixnet_message)
}

pub enum IcmpBeaconReply {
    TunDeviceReply,
    ExternalPingReply(Ipv4Addr),
}

pub enum Icmpv6BeaconReply {
    TunDeviceReply,
    ExternalPingReply(Ipv6Addr),
}

pub fn is_icmp_beacon_reply(
    packet: &Bytes,
    identifier: u16,
    destination: Ipv4Addr,
) -> Option<IcmpBeaconReply> {
    if let Some((reply_identifier, reply_source, reply_destination)) = is_icmp_echo_reply(packet)
        && reply_identifier == identifier
        && reply_destination == destination
    {
        if reply_source == NYM_TUN_DEVICE_ADDRESS_V4 {
            return Some(IcmpBeaconReply::TunDeviceReply);
        } else if reply_source == ICMP_IPR_TUN_EXTERNAL_PING_V4 {
            return Some(IcmpBeaconReply::ExternalPingReply(reply_source));
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
        && reply_identifier == identifier
        && reply_destination == destination
    {
        if reply_source == NYM_TUN_DEVICE_ADDRESS_V6 {
            return Some(Icmpv6BeaconReply::TunDeviceReply);
        } else if reply_source == ICMP_IPR_TUN_EXTERNAL_PING_V6 {
            return Some(Icmpv6BeaconReply::ExternalPingReply(reply_source));
        }
    }
    None
}

pub fn start_icmp_connection_beacon(
    mixnet_client_sender: MixnetClientSender,
    our_ips: IpPair,
    ipr_address: Recipient,
    icmp_identifier: u16,
    shutdown_listener: CancellationToken,
) -> JoinHandle<Result<()>> {
    debug!("Creating icmp connection beacon");
    let beacon =
        IcmpConnectionBeacon::new(mixnet_client_sender, our_ips, ipr_address, icmp_identifier);
    tokio::spawn(async move {
        beacon.run(shutdown_listener).await.inspect_err(|err| {
            trace_err_chain!(err, "IcmpConnectionBeacon failed");
        })
    })
}
