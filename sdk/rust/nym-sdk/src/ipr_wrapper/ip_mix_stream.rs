// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::network_env::NetworkEnvironment;
use crate::ip_packet_client::{
    helpers::check_ipr_message_version, IprListener, MixnetMessageOutcome,
};
use crate::mixnet::{MixnetClient, MixnetStream, Recipient};
use crate::Error;

use bytes::Bytes;
use nym_crypto::asymmetric::ed25519;
use nym_ip_packet_requests::{
    v8::{
        request::IpPacketRequest,
        response::{ConnectResponseReply, ControlResponse, IpPacketResponse, IpPacketResponseData},
    },
    IpPair,
};
use nym_network_defaults::ApiUrl;
use nym_sphinx::receiver::ReconstructedMessage;
use nym_validator_client::nym_api::NymApiClientExt;
use std::collections::HashMap;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{debug, error, info};

const IPR_CONNECT_TIMEOUT: Duration = Duration::from_secs(60);

/// Maximum size for a single IPR response read from the stream.
/// IPR responses fit within one Sphinx packet payload (~1.8 KB) so 64 KB
/// provides ample headroom.
const READ_BUF_SIZE: usize = 64 * 1024;

#[derive(Clone)]
pub struct IprWithPerformance {
    pub(crate) address: Recipient,
    pub(crate) identity: ed25519::PublicKey,
    pub(crate) performance: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
}

#[allow(clippy::result_large_err)]
fn create_nym_api_client(nym_api_urls: Vec<ApiUrl>) -> Result<nym_http_api_client::Client, Error> {
    let user_agent = format!("nym-sdk/{}", env!("CARGO_PKG_VERSION"));

    let urls = nym_api_urls
        .into_iter()
        .map(|url| url.url.parse())
        .collect::<Result<Vec<nym_http_api_client::Url>, _>>()
        .map_err(|err| {
            error!("malformed nym-api url: {err}");
            Error::NoNymAPIUrl
        })?;

    if urls.is_empty() {
        return Err(Error::NoNymAPIUrl);
    }

    let client = nym_http_api_client::ClientBuilder::new_with_urls(urls)?
        .with_user_agent(user_agent)
        .build()?;

    Ok(client)
}

async fn retrieve_exit_nodes_with_performance(
    client: nym_http_api_client::Client,
) -> Result<Vec<IprWithPerformance>, Error> {
    let all_nodes = client
        .get_all_described_nodes_v2()
        .await?
        .into_iter()
        .map(|described| (described.ed25519_identity_key(), described))
        .collect::<HashMap<_, _>>();

    let exit_gateways = client.get_all_basic_nodes_with_metadata().await?.nodes;

    let mut described = Vec::new();

    for exit in exit_gateways {
        if let Some(ipr_info) = all_nodes
            .get(&exit.ed25519_identity_pubkey)
            .and_then(|n| n.description.ip_packet_router.clone())
        {
            if let Ok(parsed_address) = ipr_info.address.parse() {
                described.push(IprWithPerformance {
                    address: parsed_address,
                    identity: exit.ed25519_identity_pubkey,
                    performance: exit.performance.round_to_integer(),
                })
            }
        }
    }

    Ok(described)
}

async fn get_random_ipr(client: nym_http_api_client::Client) -> Result<Recipient, Error> {
    let nodes = retrieve_exit_nodes_with_performance(client).await?;
    info!("Found {} Exit Gateways", nodes.len());

    let selected_gateway = nodes
        .into_iter()
        .max_by_key(|gw| gw.performance)
        .ok_or_else(|| Error::NoGatewayAvailable)?;

    let ipr_address = selected_gateway.address;

    info!(
        "Using IPR: {} (Gateway: {}, Performance: {:?})",
        ipr_address, selected_gateway.identity, selected_gateway.performance
    );

    Ok(ipr_address)
}

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
///       → MixnetStream.read() → IpPacketResponse bytes
///       → IprListener → extract IP packets
/// ```
pub struct IpMixStream {
    /// The underlying multiplexed stream to the IPR gateway.
    stream: MixnetStream,
    /// Kept for `nym_address()` and `disconnect()`.
    client: MixnetClient,
    /// Parses incoming IPR protocol responses.
    listener: IprListener,
    read_buf: Vec<u8>,
    allocated_ips: Option<IpPair>,
    connection_state: ConnectionState,
}

impl IpMixStream {
    /// Create a new IP packet router stream connected to the mixnet.
    ///
    /// Discovers an IPR gateway, connects a MixnetClient, and opens a
    /// `MixnetStream` to the IPR. Call [`connect_tunnel`](Self::connect_tunnel)
    /// to establish the IP tunnel.
    pub async fn new(env: NetworkEnvironment) -> Result<Self, Error> {
        let network_defaults = env.network_defaults();
        let api_client = create_nym_api_client(network_defaults.nym_api_urls.unwrap_or_default())?;
        let ipr_address = get_random_ipr(api_client).await?;

        nym_network_defaults::setup_env(Some(env.env_file_path()));
        let mut client = MixnetClient::connect_new().await?;

        // Open a stream to the IPR — this sends the LP Stream Open handshake
        // and starts the background stream router.
        let stream = client.open_stream(ipr_address, Some(10)).await?;

        Ok(Self {
            stream,
            client,
            listener: IprListener::new(),
            read_buf: vec![0u8; READ_BUF_SIZE],
            allocated_ips: None,
            connection_state: ConnectionState::Disconnected,
        })
    }

    /// Get the Nym network address of this stream.
    pub fn nym_address(&self) -> &Recipient {
        self.client.nym_address()
    }

    /// Establish tunnel connection with the IPR and allocate IP addresses.
    pub async fn connect_tunnel(&mut self) -> Result<IpPair, Error> {
        if self.connection_state != ConnectionState::Disconnected {
            return Err(Error::IprStreamClientAlreadyConnectedOrConnecting);
        }

        self.connection_state = ConnectionState::Connecting;
        info!("Connecting to IP packet router");

        match self.connect_inner().await {
            Ok(ip_pair) => {
                self.allocated_ips = Some(ip_pair);
                self.connection_state = ConnectionState::Connected;
                info!(
                    "Connected to IPv4: {}, IPv6: {}",
                    ip_pair.ipv4, ip_pair.ipv6
                );
                Ok(ip_pair)
            }
            Err(e) => {
                self.connection_state = ConnectionState::Disconnected;
                error!("Failed to connect: {:?}", e);
                Err(e)
            }
        }
    }

    async fn connect_inner(&mut self) -> Result<IpPair, Error> {
        let (request, request_id) = IpPacketRequest::new_connect_request(None);
        debug!("Sending connect request with ID: {}", request_id);

        let request_bytes = request.to_bytes()?;
        self.stream
            .write_all(&request_bytes)
            .await
            .map_err(|_| Error::MessageSendingFailure)?;

        self.listen_for_connect_response(request_id).await
    }

    async fn listen_for_connect_response(&mut self, request_id: u64) -> Result<IpPair, Error> {
        let timeout = tokio::time::sleep(IPR_CONNECT_TIMEOUT);
        tokio::pin!(timeout);

        loop {
            tokio::select! {
                _ = &mut timeout => {
                    return Err(Error::IPRConnectResponseTimeout);
                }
                result = self.stream.read(&mut self.read_buf) => {
                    match result {
                        Ok(0) => return Err(Error::IPRClientStreamClosed),
                        Ok(n) => {
                            let msg = ReconstructedMessage {
                                message: self.read_buf[..n].to_vec(),
                                sender_tag: None,
                            };
                            if let Err(e) = check_ipr_message_version(&msg) {
                                return Err(Error::IPRMessageVersionCheckFailed(e.to_string()));
                            }
                            if let Ok(response) = IpPacketResponse::from_reconstructed_message(&msg) {
                                if response.id() == Some(request_id) {
                                    return self.handle_connect_response(response);
                                }
                            }
                        }
                        Err(_) => return Err(Error::IPRClientStreamClosed),
                    }
                }
            }
        }
    }

    fn handle_connect_response(&self, response: IpPacketResponse) -> Result<IpPair, Error> {
        let control_response = match response.data {
            IpPacketResponseData::Control(c) => c,
            other => return Err(Error::UnexpectedResponseType(other)),
        };

        match *control_response {
            ControlResponse::Connect(connect_resp) => match connect_resp.reply {
                ConnectResponseReply::Success(success) => Ok(success.ips),
                ConnectResponseReply::Failure(reason) => Err(Error::ConnectDenied(reason)),
            },
            _ => Err(Error::UnexpectedResponseType(
                IpPacketResponseData::Control(control_response.clone()),
            )),
        }
    }

    /// Send an IP packet through the tunnel.
    pub async fn send_ip_packet(&mut self, packet: &[u8]) -> Result<(), Error> {
        if self.connection_state != ConnectionState::Connected {
            return Err(Error::IprStreamClientNotConnected);
        }
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
        match tokio::time::timeout(
            Duration::from_secs(10),
            self.stream.read(&mut self.read_buf),
        )
        .await
        {
            // Timeout — no data yet, not an error
            Err(_) => Ok(Vec::new()),
            // EOF — stream router shut down, channel dead
            Ok(Ok(0)) => {
                self.connection_state = ConnectionState::Disconnected;
                Err(Error::IPRClientStreamClosed)
            }
            // IO error
            Ok(Err(_)) => {
                self.connection_state = ConnectionState::Disconnected;
                Err(Error::IPRClientStreamClosed)
            }
            Ok(Ok(n)) => {
                let msg = ReconstructedMessage {
                    message: self.read_buf[..n].to_vec(),
                    sender_tag: None,
                };
                match self.listener.handle_reconstructed_message(msg).await {
                    Ok(Some(MixnetMessageOutcome::IpPackets(packets))) => {
                        debug!("Extracted {} IP packets", packets.len());
                        Ok(packets)
                    }
                    Ok(Some(MixnetMessageOutcome::Disconnect)) => {
                        info!("Received disconnect");
                        self.connection_state = ConnectionState::Disconnected;
                        self.allocated_ips = None;
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
        }
    }

    pub fn allocated_ips(&self) -> Option<&IpPair> {
        self.allocated_ips.as_ref()
    }

    pub fn is_connected(&self) -> bool {
        self.connection_state == ConnectionState::Connected
    }

    /// Disconnect from the Mixnet. Disconnected clients cannot be reconnected.
    pub async fn disconnect_stream(self) {
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
        let mut stream = IpMixStream::new(NetworkEnvironment::Mainnet).await?;
        let ip_pair = stream.connect_tunnel().await?;

        let ipv4: Ipv4Addr = ip_pair.ipv4;
        assert!(!ipv4.is_unspecified(), "IPv4 address should not be 0.0.0.0");

        let ipv6: Ipv6Addr = ip_pair.ipv6;
        assert!(!ipv6.is_unspecified(), "IPv6 address should not be ::");

        assert!(stream.is_connected(), "Stream should be connected");
        assert!(
            stream.allocated_ips().is_some(),
            "Should have allocated IPs"
        );

        stream.disconnect_stream().await;

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
        let ip_pair = stream.connect_tunnel().await?;

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

        stream.disconnect_stream().await;
        Ok(())
    }
}
