// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::network_env::NetworkEnvironment;
use crate::ip_packet_client::{
    discovery::{create_nym_api_client, get_random_ipr, parse_connect_response},
    helpers::check_ipr_message_version,
    IprListener, MixnetMessageOutcome,
};
use crate::mixnet::{MixnetClient, MixnetStream, Recipient};
use crate::Error;

use bytes::Bytes;
use nym_ip_packet_requests::{
    v8::{request::IpPacketRequest, response::IpPacketResponse},
    IpPair,
};
use nym_sphinx::receiver::ReconstructedMessage;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tracing::{debug, error, info};

const IPR_CONNECT_TIMEOUT: Duration = Duration::from_secs(60);

/// A bidirectional tunnel for sending and receiving IP packets through the mixnet.
///
/// Wraps a [`MixnetStream`] (opened to an IPR exit gateway) and provides a
/// high-level API for the IPR protocol. The underlying `MixnetStream` handles
/// LP Stream framing and stream multiplexing automatically.
///
/// # Data flow
///
/// ```text
/// IpMixStream.send_ip_packet(bytes)
///   → IpPacketRequest::to_bytes() → MixnetStream.write()
///       → LP Stream frame (stream_id, seq, Data)
///       → Sphinx packets → mixnet → IPR
///
/// IPR processes request → TUN → internet → response
///   → IPR wraps in LP Stream frame → Sphinx → mixnet → client
///       → stream router dispatches by stream_id
///       → MixnetStream.recv() → IpPacketResponse bytes
///       → IprListener → extract IP packets
/// ```
pub struct IpMixStream {
    stream: MixnetStream,
    client: MixnetClient,
    listener: IprListener,
    allocated_ips: IpPair,
    connected: bool,
}

impl IpMixStream {
    /// Discover an IPR, connect through the mixnet, and establish the IP tunnel.
    ///
    /// Returns a ready-to-use tunnel with allocated IP addresses.
    pub async fn new(env: NetworkEnvironment) -> Result<Self, Error> {
        let network_defaults = env.network_defaults();
        let api_client = create_nym_api_client(network_defaults.nym_api_urls.unwrap_or_default())?;
        let ipr_address = get_random_ipr(api_client).await?;

        nym_network_defaults::setup_env(Some(env.env_file_path()));
        let mut client = MixnetClient::connect_new().await?;
        let mut stream = client.open_stream(ipr_address, Some(10)).await?;

        info!("Connecting to IP packet router");
        let allocated_ips = Self::connect_tunnel(&mut stream).await?;
        info!(
            "Connected to IPv4: {}, IPv6: {}",
            allocated_ips.ipv4, allocated_ips.ipv6
        );

        Ok(Self {
            stream,
            client,
            listener: IprListener::new(),
            allocated_ips,
            connected: true,
        })
    }

    pub fn nym_address(&self) -> &Recipient {
        self.client.nym_address()
    }

    pub fn allocated_ips(&self) -> &IpPair {
        &self.allocated_ips
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }

    /// Check that the tunnel is connected, returning an error if not.
    pub fn check_connected(&self) -> Result<(), Error> {
        if self.connected {
            Ok(())
        } else {
            Err(Error::IprStreamClientNotConnected)
        }
    }

    async fn connect_tunnel(stream: &mut MixnetStream) -> Result<IpPair, Error> {
        let (request, request_id) = IpPacketRequest::new_connect_request(None);
        debug!("Sending connect request with ID: {}", request_id);

        let request_bytes = request.to_bytes()?;
        stream
            .write_all(&request_bytes)
            .await
            .map_err(|_| Error::MessageSendingFailure)?;

        let timeout = tokio::time::sleep(IPR_CONNECT_TIMEOUT);
        tokio::pin!(timeout);

        loop {
            tokio::select! {
                _ = &mut timeout => {
                    return Err(Error::IPRConnectResponseTimeout);
                }
                result = stream.recv() => {
                    let data = result.ok_or(Error::IPRClientStreamClosed)?;
                    let msg = ReconstructedMessage { message: data, sender_tag: None };

                    if let Err(e) = check_ipr_message_version(&msg) {
                        return Err(Error::IPRMessageVersionCheckFailed(e.to_string()));
                    }
                    if let Ok(response) = IpPacketResponse::from_reconstructed_message(&msg) {
                        if response.id() == Some(request_id) {
                            return parse_connect_response(response);
                        }
                    }
                }
            }
        }
    }

    /// Send an IP packet through the tunnel.
    pub async fn send_ip_packet(&mut self, packet: &[u8]) -> Result<(), Error> {
        self.check_connected()?;
        let request = IpPacketRequest::new_data_request(packet.to_vec().into());
        let request_bytes = request.to_bytes()?;
        self.stream
            .write_all(&request_bytes)
            .await
            .map_err(|_| Error::MessageSendingFailure)
    }

    /// Handle incoming messages from the mixnet.
    ///
    /// Reads from the underlying `MixnetStream`, parses IPR responses, and
    /// extracts IP packets. Returns an empty vec on timeout (10 s).
    pub async fn handle_incoming(&mut self) -> Result<Vec<Bytes>, Error> {
        let data = match tokio::time::timeout(Duration::from_secs(10), self.stream.recv()).await {
            Err(_) => return Ok(Vec::new()),
            Ok(None) => {
                self.connected = false;
                return Err(Error::IPRClientStreamClosed);
            }
            Ok(Some(data)) => data,
        };

        let msg = ReconstructedMessage {
            message: data,
            sender_tag: None,
        };
        match self.listener.handle_reconstructed_message(msg).await {
            Ok(Some(MixnetMessageOutcome::IpPackets(packets))) => {
                debug!("Extracted {} IP packets", packets.len());
                Ok(packets)
            }
            Ok(Some(MixnetMessageOutcome::Disconnect)) => {
                info!("Received disconnect");
                self.connected = false;
                Ok(Vec::new())
            }
            Ok(Some(MixnetMessageOutcome::MixnetSelfPing)) => {
                debug!("Received mixnet self ping");
                Ok(Vec::new())
            }
            Ok(None) => Ok(Vec::new()),
            Err(e) => {
                error!("Failed to handle message: {}", e);
                Ok(Vec::new())
            }
        }
    }

    /// Disconnect from the Mixnet. Disconnected clients cannot be reconnected.
    pub async fn disconnect(self) {
        debug!("Disconnecting");
        self.client.disconnect().await;
        debug!("Disconnected");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ip_packet_client::helpers::{
        icmp_identifier, is_icmp_echo_reply, is_icmp_v6_echo_reply,
    };
    use std::net::{Ipv4Addr, Ipv6Addr};

    #[tokio::test]
    #[ignore]
    async fn connect_to_ipr() -> Result<(), Box<dyn std::error::Error>> {
        let stream = IpMixStream::new(NetworkEnvironment::Mainnet).await?;

        let ipv4: Ipv4Addr = stream.allocated_ips().ipv4;
        assert!(!ipv4.is_unspecified(), "IPv4 address should not be 0.0.0.0");

        let ipv6: Ipv6Addr = stream.allocated_ips().ipv6;
        assert!(!ipv6.is_unspecified(), "IPv6 address should not be ::");

        stream.disconnect().await;

        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn dns_ping_checks() -> Result<(), Box<dyn std::error::Error>> {
        use crate::ip_packet_client::helpers::{
            create_icmpv4_echo_request, create_icmpv6_echo_request, wrap_icmp_in_ipv4,
            wrap_icmp_in_ipv6,
        };
        use nym_ip_packet_requests::codec::MultiIpPacketCodec;
        use pnet_packet::Packet;

        let mut stream = IpMixStream::new(NetworkEnvironment::Mainnet).await?;
        let ip_pair = *stream.allocated_ips();

        info!(
            "Connected with IPs - IPv4: {}, IPv6: {}",
            ip_pair.ipv4, ip_pair.ipv6
        );

        let external_v4_targets = vec![
            ("Google DNS", Ipv4Addr::new(8, 8, 8, 8)),
            ("Cloudflare DNS", Ipv4Addr::new(1, 1, 1, 1)),
            ("Quad9 DNS", Ipv4Addr::new(9, 9, 9, 9)),
        ];

        let external_v6_targets = vec![
            ("Google DNS", "2001:4860:4860::8888".parse::<Ipv6Addr>()?),
            (
                "Cloudflare DNS",
                "2606:4700:4700::1111".parse::<Ipv6Addr>()?,
            ),
            ("Quad9 DNS", "2620:fe::fe".parse::<Ipv6Addr>()?),
        ];

        let identifier = icmp_identifier();
        let mut successful_v4_pings = 0;
        let mut total_v4_pings = 0;
        let mut successful_v6_pings = 0;
        let mut total_v6_pings = 0;

        for (name, target) in &external_v4_targets {
            info!("Testing IPv4 connectivity to {} ({})", name, target);

            for seq in 0..3 {
                let icmp = create_icmpv4_echo_request(seq, identifier)?;
                let ipv4_packet = wrap_icmp_in_ipv4(icmp, ip_pair.ipv4, *target)?;
                let bundled =
                    MultiIpPacketCodec::bundle_one_packet(ipv4_packet.packet().to_vec().into());
                stream.send_ip_packet(&bundled).await?;
                total_v4_pings += 1;
            }
        }

        for (name, target) in &external_v6_targets {
            info!("Testing IPv6 connectivity to {} ({})", name, target);

            for seq in 0..3 {
                let icmp = create_icmpv6_echo_request(seq, identifier, &ip_pair.ipv6, target)?;
                let ipv6_packet = wrap_icmp_in_ipv6(icmp, ip_pair.ipv6, *target)?;
                let bundled =
                    MultiIpPacketCodec::bundle_one_packet(ipv6_packet.packet().to_vec().into());
                stream.send_ip_packet(&bundled).await?;
                total_v6_pings += 1;
            }
        }

        let collect_timeout = tokio::time::sleep(Duration::from_secs(10));
        tokio::pin!(collect_timeout);

        loop {
            tokio::select! {
                _ = &mut collect_timeout => {
                    info!("Finished collecting replies");
                    break;
                }
                result = stream.handle_incoming() => {
                    if let Ok(packets) = result {
                        for packet in packets {
                            if let Some((reply_id, _source, dest)) = is_icmp_echo_reply(&packet) {
                                if reply_id == identifier && dest == ip_pair.ipv4 {
                                    successful_v4_pings += 1;
                                }
                            }

                            if let Some((reply_id, _source, dest)) = is_icmp_v6_echo_reply(&packet) {
                                if reply_id == identifier && dest == ip_pair.ipv6 {
                                    successful_v6_pings += 1;
                                }
                            }
                        }
                    }
                }
            }
        }

        let v4_success_rate = (successful_v4_pings as f64 / total_v4_pings as f64) * 100.0;
        let v6_success_rate = (successful_v6_pings as f64 / total_v6_pings as f64) * 100.0;

        info!(
            "IPv4: {}/{} ({:.1}%), IPv6: {}/{} ({:.1}%)",
            successful_v4_pings,
            total_v4_pings,
            v4_success_rate,
            successful_v6_pings,
            total_v6_pings,
            v6_success_rate
        );

        assert!(successful_v4_pings > 0, "No IPv4 pings successful");
        assert!(v4_success_rate >= 75.0, "IPv4 success rate < 75%");
        assert!(successful_v6_pings > 0, "No IPv6 pings successful");
        assert!(v6_success_rate >= 75.0, "IPv6 success rate < 75%");

        stream.disconnect().await;
        Ok(())
    }
}
