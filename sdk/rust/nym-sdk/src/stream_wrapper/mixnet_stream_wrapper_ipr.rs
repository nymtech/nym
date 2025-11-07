// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::mixnet_stream_wrapper::{MixStream, MixStreamReader, MixStreamWriter};
use super::network_env::NetworkEnvironment;
use crate::ip_packet_client::{
    helpers::check_ipr_message_version, IprListener, MixnetMessageOutcome,
};
use crate::{mixnet::Recipient, Error};
use std::collections::HashMap;

use bytes::Bytes;
use futures::StreamExt;
use nym_crypto::asymmetric::ed25519;
use nym_ip_packet_requests::{
    v8::{
        request::IpPacketRequest,
        response::{ConnectResponseReply, ControlResponse, IpPacketResponse, IpPacketResponseData},
    },
    IpPair,
};
use nym_network_defaults::ApiUrl;
use nym_validator_client::nym_api::NymApiClientExt;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::sync::oneshot;
use tracing::{debug, error, info};

const IPR_CONNECT_TIMEOUT: Duration = Duration::from_secs(60);

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

/// Create a Nym API client with the provided URLs.
///
/// # Arguments
/// * `nym_api_urls` - Vector of API URLs to use for the client
///
/// # Returns
/// Configured `nym_http_api_client::Client` or an error if URLs are invalid or empty
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

/// Retrieve all exit nodes with their performance scores.
///
/// Queries the network for all described nodes and filters for those with IP packet router
/// capabilities, combining node information with performance metadata.
///
/// # Arguments
/// * `client` - Nym API client to use for queries
///
/// # Returns
/// Vector of `IprWithPerformance` containing exit node addresses, identities, and performance scores
async fn retrieve_exit_nodes_with_performance(
    client: nym_http_api_client::Client,
) -> Result<Vec<IprWithPerformance>, Error> {
    // retrieve all nym-nodes on the network
    let all_nodes = client
        .get_all_described_nodes()
        .await?
        .into_iter()
        .map(|described| (described.ed25519_identity_key(), described))
        .collect::<HashMap<_, _>>();

    // annoyingly there's no convenient way of doing this in a single query
    // retrieve performance scores of all exit gateways
    let exit_gateways = client
        .get_all_basic_exit_assigned_nodes_with_metadata()
        .await?
        .nodes;

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

/// Select a random IPR (IP Packet Router) from available exit gateways.
///
/// Currently selects the gateway with the highest performance score.
///
/// # Arguments
/// * `client` - Nym API client to use for gateway discovery
///
/// # Returns
/// `Recipient` address of the selected IPR
async fn get_random_ipr(client: nym_http_api_client::Client) -> Result<Recipient, Error> {
    let nodes = retrieve_exit_nodes_with_performance(client).await?;
    info!("Found {} Exit Gateways", nodes.len());

    // JS: I'm leaving the old behaviour here of choosing node with the highest performance,
    // but I think you should reconsider making a pseudorandom selection weighted by some scaled performance
    // otherwise all clients might end up choosing exactly the same node (I will leave this as PR comment when I get here : D)
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

/// A bidirectional stream for sending and receiving IP packets through the mixnet.
///
/// Manages connection to an IP Packet Router (IPR), handles tunnel establishment,
/// and maintains allocated IP addresses. Implements `AsyncRead` and `AsyncWrite` for
/// standard async I/O operations.
///
/// # Example
/// ```no_run
/// use nym_sdk::stream_wrapper::{IpMixStream, NetworkEnvironment};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let mut stream = IpMixStream::new(NetworkEnvironment::Mainnet).await?;
///     let ip_pair = stream.connect_tunnel().await?;
///     let packet_data = vec![0u8; 100];
///     stream.send_ip_packet(&packet_data).await?;
///     Ok(())
/// }
/// ```
pub struct IpMixStream {
    stream: MixStream,
    ipr_address: Recipient,
    listener: IprListener,
    allocated_ips: Option<IpPair>,
    connection_state: ConnectionState,
}

impl IpMixStream {
    /// Create a new IP packet router stream connected to the mixnet.
    ///
    /// Initializes connection to mainnet by default and selects an IPR gateway.
    /// Does not establish tunnel connection - call `connect_tunnel()` separately.
    ///
    /// # Returns
    /// New `IpMixStream` instance ready to connect
    pub async fn new(env: NetworkEnvironment) -> Result<Self, Error> {
        let network_defaults = env.network_defaults();
        let api_client = create_nym_api_client(network_defaults.nym_api_urls.unwrap_or_default())?;
        let ipr_address = get_random_ipr(api_client).await?;

        let stream = MixStream::new(None, Some(ipr_address), Some(env.env_file_path())).await?;

        Ok(Self {
            stream,
            ipr_address,
            listener: IprListener::new(),
            allocated_ips: None,
            connection_state: ConnectionState::Disconnected,
        })
    }

    /// Get the Nym network address of this stream.
    ///
    /// # Returns
    /// Reference to the stream's `Recipient` address
    pub fn nym_address(&self) -> &Recipient {
        self.stream.client.nym_address()
    }

    /// Send an IP packet request to the connected IPR.
    ///
    /// # Arguments
    /// * `request` - The `IpPacketRequest` to send
    ///
    /// # Returns
    /// `Ok(())` on success, error otherwise
    async fn send_ipr_request(&mut self, request: IpPacketRequest) -> Result<(), Error> {
        let request_bytes = request.to_bytes()?;
        self.stream.send(&request_bytes).await
    }

    /// Establish tunnel connection with the IPR and allocate IP addresses.
    ///
    /// Sends a connect request and waits for IP allocation response.
    /// Updates connection state and stores allocated IPs on success.
    ///
    /// # Returns
    /// `IpPair` containing allocated IPv4 and IPv6 addresses
    pub async fn connect_tunnel(&mut self) -> Result<IpPair, Error> {
        if self.connection_state != ConnectionState::Disconnected {
            return Err(Error::IprStreamClientAlreadyConnectedOrConnecting);
        }

        self.connection_state = ConnectionState::Connecting;
        info!("Connecting to IP packet router: {}", self.ipr_address);

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

    /// Internal connection logic for establishing the tunnel.
    ///
    /// # Returns
    /// `IpPair` containing allocated IP addresses
    async fn connect_inner(&mut self) -> Result<IpPair, Error> {
        let (request, request_id) = IpPacketRequest::new_connect_request(None);
        debug!("Sending connect request with ID: {}", request_id);

        self.send_ipr_request(request).await?;
        self.listen_for_connect_response(request_id).await
    }

    /// Listen for and process the connect response from the IPR.
    ///
    /// Waits up to `IPR_CONNECT_TIMEOUT` for a response matching the request ID.
    ///
    /// # Arguments
    /// * `request_id` - ID of the connect request to match against responses
    ///
    /// # Returns
    /// `IpPair` containing allocated IP addresses
    async fn listen_for_connect_response(&mut self, request_id: u64) -> Result<IpPair, Error> {
        let timeout = tokio::time::sleep(IPR_CONNECT_TIMEOUT);
        tokio::pin!(timeout);

        let mut framed = self.stream.framed_read();

        loop {
            tokio::select! {
                _ = &mut timeout => {
                    return Err(Error::IPRConnectResponseTimeout);
                }
                frame = framed.next() => {
                    match frame {
                        None => {
                            return Err(Error::IPRClientStreamClosed);
                        }
                        Some(Err(e)) => {
                            return Err(Error::MessageRecovery(e));
                        }
                        Some(Ok(reconstructed)) => {
                            if let Err(e) = check_ipr_message_version(&reconstructed) {
                                return Err(Error::IPRMessageVersionCheckFailed(e.to_string()));

                            }
                            if let Ok(response) = IpPacketResponse::from_reconstructed_message(&reconstructed) {
                                if response.id() == Some(request_id) {
                                    return self.handle_connect_response(response).await;
                                }
                                else {
                                    return Err(Error::IPRNoId)
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Handle the connect response from the IPR.
    ///
    /// Extracts IP allocation from successful response or returns error on failure.
    ///
    /// # Arguments
    /// * `response` - The `IpPacketResponse` to process
    ///
    /// # Returns
    /// `IpPair` on successful connection, error otherwise
    async fn handle_connect_response(&self, response: IpPacketResponse) -> Result<IpPair, Error> {
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
    ///
    /// Requires an active tunnel connection.
    ///
    /// # Arguments
    /// * `packet` - Raw IP packet bytes to send
    ///
    /// # Returns
    /// `Ok(())` on success, error if not connected or send fails
    pub async fn send_ip_packet(&mut self, packet: &[u8]) -> Result<(), Error> {
        if self.connection_state != ConnectionState::Connected {
            return Err(Error::IprStreamClientNotConnected);
        }
        let request = IpPacketRequest::new_data_request(packet.to_vec().into());
        self.send_ipr_request(request).await
    }

    /// Handle incoming messages from the mixnet.
    ///
    /// Processes reconstructed messages and extracts IP packets, disconnect signals,
    /// or self-ping messages. Times out after 10 seconds if no message received.
    ///
    /// # Returns
    /// Vector of received IP packet data as `Bytes`, empty vector if no packets or on timeout
    pub async fn handle_incoming(&mut self) -> Result<Vec<Bytes>, Error> {
        let mut framed = self.stream.framed_read();

        match tokio::time::timeout(Duration::from_secs(10), framed.next()).await {
            Ok(Some(reconstructed)) => {
                match self
                    .listener
                    .handle_reconstructed_message(reconstructed?)
                    .await
                {
                    Ok(Some(MixnetMessageOutcome::IpPackets(packets))) => {
                        info!("Extracted {} IP packets", packets.len());
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
            _ => Ok(Vec::new()),
        }
    }

    /// Get the allocated IP addresses for this tunnel.
    ///
    /// # Returns
    /// `Some(&IpPair)` if IPs are allocated, `None` otherwise
    pub fn allocated_ips(&self) -> Option<&IpPair> {
        self.allocated_ips.as_ref()
    }

    /// Check if the tunnel is currently connected.
    ///
    /// # Returns
    /// `true` if connected, `false` otherwise
    pub fn is_connected(&self) -> bool {
        self.connection_state == ConnectionState::Connected
    }

    /// Disconnect inner stream client from the Mixnet - note that disconnected clients cannot currently be reconnected.
    pub async fn disconnect_stream(self) {
        debug!("Disconnecting");
        self.stream.client.disconnect().await;
        debug!("Disconnected");
    }

    /// Split the stream into separate reader and writer halves.
    ///
    /// Enables concurrent read and write operations similar to `TcpStream::split()`.
    /// State updates (connection status, allocated IPs) are synchronized between halves
    /// via oneshot channels.
    ///
    /// # Returns
    /// Tuple of `(IpMixStreamReader, IpMixStreamWriter)`
    pub fn split(self) -> (IpMixStreamReader, IpMixStreamWriter) {
        debug!("Splitting IpMixStream");
        let local_addr = *self.stream.client.nym_address();
        let (stream_reader, stream_writer) = self.stream.split();
        debug!("Split IpMixStream into Reader and Writer");

        let (state_tx, state_rx) = oneshot::channel();
        let (ips_tx, ips_rx) = oneshot::channel();

        (
            IpMixStreamReader {
                stream_reader,
                listener: self.listener,
                allocated_ips: self.allocated_ips,
                connection_state: self.connection_state.clone(),
                state_tx: Some(state_tx),
                ips_tx: Some(ips_tx),
            },
            IpMixStreamWriter {
                stream_writer,
                local_addr,
                allocated_ips: self.allocated_ips,
                connection_state: self.connection_state,
                state_rx: Some(state_rx),
                ips_rx: Some(ips_rx),
            },
        )
    }
}

impl AsyncRead for IpMixStream {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::pin::Pin::new(&mut self.stream).poll_read(cx, buf)
    }
}

impl AsyncWrite for IpMixStream {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        std::pin::Pin::new(&mut self.stream).poll_write(cx, buf)
    }

    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::pin::Pin::new(&mut self.stream).poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::pin::Pin::new(&mut self.stream).poll_shutdown(cx)
    }
}

/// Read half of a split `IpMixStream`.
///
/// Handles incoming messages from the mixnet and processes IP packets, disconnect
/// signals, and control messages. Synchronizes connection state changes with the
/// writer half via oneshot channels.
///
/// Created by calling `IpMixStream::split()`. Implements `AsyncRead` for standard
/// async read operations.
pub struct IpMixStreamReader {
    stream_reader: MixStreamReader,
    listener: IprListener,
    allocated_ips: Option<IpPair>,
    connection_state: ConnectionState,
    state_tx: Option<oneshot::Sender<ConnectionState>>,
    ips_tx: Option<oneshot::Sender<Option<IpPair>>>,
}

impl IpMixStreamReader {
    /// Get the Nym network address of this reader.
    ///
    /// # Returns
    /// The reader's `Recipient` address
    pub fn nym_address(self) -> Recipient {
        *self.stream_reader.local_addr()
    }

    /// Handle incoming messages from the mixnet (reader half).
    ///
    /// Processes reconstructed messages and extracts IP packets, disconnect signals,
    /// or self-ping messages. Updates connection state and notifies writer on disconnect.
    /// Times out after 10 seconds if no message received.
    ///
    /// # Returns
    /// Vector of received IP packet data as `Bytes`, empty vector if no packets or on timeout
    pub async fn handle_incoming(&mut self) -> Result<Vec<Bytes>, Error> {
        let mut framed = self.stream_reader.framed();

        match tokio::time::timeout(Duration::from_secs(10), framed.next()).await {
            Ok(Some(reconstructed)) => {
                match self
                    .listener
                    .handle_reconstructed_message(reconstructed?)
                    .await
                {
                    Ok(Some(MixnetMessageOutcome::IpPackets(packets))) => {
                        info!("Extracted {} IP packets", packets.len());
                        Ok(packets)
                    }
                    Ok(Some(MixnetMessageOutcome::Disconnect)) => {
                        info!("Received disconnect");
                        self.connection_state = ConnectionState::Disconnected;
                        self.allocated_ips = None;
                        // Send state update to writer
                        if let Some(tx) = self.state_tx.take() {
                            let _ = tx.send(ConnectionState::Disconnected);
                        }
                        if let Some(tx) = self.ips_tx.take() {
                            let _ = tx.send(None);
                        }
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
            _ => Ok(Vec::new()),
        }
    }

    /// Get the allocated IP addresses (reader half).
    ///
    /// # Returns
    /// `Some(&IpPair)` if IPs are allocated, `None` otherwise
    pub fn allocated_ips(&self) -> Option<&IpPair> {
        self.allocated_ips.as_ref()
    }

    /// Check if the tunnel is currently connected (reader half).
    ///
    /// # Returns
    /// `true` if connected, `false` otherwise
    pub fn is_connected(&self) -> bool {
        self.connection_state == ConnectionState::Connected
    }
}

impl AsyncRead for IpMixStreamReader {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.stream_reader).poll_read(cx, buf)
    }
}

/// Write half of a split `IpMixStream`.
///
/// Handles outgoing IP packets to the mixnet. Receives connection state updates
/// from the reader half via oneshot channels to maintain synchronized state.
///
/// Created by calling `IpMixStream::split()`. Implements `AsyncWrite` for standard
/// async write operations.
pub struct IpMixStreamWriter {
    stream_writer: MixStreamWriter,
    local_addr: Recipient,
    allocated_ips: Option<IpPair>,
    connection_state: ConnectionState,
    state_rx: Option<oneshot::Receiver<ConnectionState>>,
    ips_rx: Option<oneshot::Receiver<Option<IpPair>>>,
}

impl IpMixStreamWriter {
    /// Get the Nym network address of this writer.
    ///
    /// # Returns
    /// Reference to the writer's `Recipient` address
    pub fn nym_address(&self) -> &Recipient {
        &self.local_addr
    }

    /// Send an IP packet request to the IPR (writer half).
    ///
    /// # Arguments
    /// * `request` - The `IpPacketRequest` to send
    ///
    /// # Returns
    /// `Ok(())` on success, error otherwise
    async fn send_ipr_request(&mut self, request: IpPacketRequest) -> Result<(), Error> {
        let request_bytes = request.to_bytes()?;
        self.stream_writer.write_bytes(&request_bytes).await
    }

    /// Send an IP packet through the tunnel (writer half).
    ///
    /// Checks for state updates from reader before sending.
    /// Requires an active tunnel connection.
    ///
    /// # Arguments
    /// * `packet` - Raw IP packet bytes to send
    ///
    /// # Returns
    /// `Ok(())` on success, error if not connected or send fails
    pub async fn send_ip_packet(&mut self, packet: &[u8]) -> Result<(), Error> {
        // Check for state updates from reader
        if let Some(mut rx) = self.state_rx.take() {
            if let Ok(new_state) = rx.try_recv() {
                self.connection_state = new_state;
            } else {
                self.state_rx = Some(rx);
            }
        }

        if let Some(mut rx) = self.ips_rx.take() {
            if let Ok(new_ips) = rx.try_recv() {
                self.allocated_ips = new_ips;
            } else {
                self.ips_rx = Some(rx);
            }
        }

        if self.connection_state != ConnectionState::Connected {
            return Err(Error::IprStreamClientNotConnected);
        }

        let request = IpPacketRequest::new_data_request(packet.to_vec().into());
        self.send_ipr_request(request).await
    }

    /// Get the allocated IP addresses (writer half).
    ///
    /// # Returns
    /// `Some(&IpPair)` if IPs are allocated, `None` otherwise
    pub fn allocated_ips(&self) -> Option<&IpPair> {
        self.allocated_ips.as_ref()
    }

    /// Check if the tunnel is connected (writer half).
    ///
    /// # Returns
    /// `true` if connected, `false` otherwise
    pub fn is_connected(&self) -> bool {
        self.connection_state == ConnectionState::Connected
    }
}

impl AsyncWrite for IpMixStreamWriter {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.stream_writer).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.stream_writer).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.stream_writer).poll_shutdown(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ip_packet_client::helpers::{
        icmp_identifier, is_icmp_echo_reply, is_icmp_v6_echo_reply, send_ping_v4, send_ping_v6,
    };
    use std::net::{Ipv4Addr, Ipv6Addr};
    use std::sync::Once;

    static INIT: Once = Once::new();

    fn init_logging() {
        if tracing::dispatcher::has_been_set() {
            return;
        }
        INIT.call_once(|| {
            nym_bin_common::logging::setup_tracing_logger();
        });
    }

    #[tokio::test]
    async fn connect_to_ipr() -> Result<(), Box<dyn std::error::Error>> {
        init_logging();

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
    async fn dns_ping_checks() -> Result<(), Box<dyn std::error::Error>> {
        init_logging();

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
                send_ping_v4(&mut stream, &ip_pair, seq, identifier, *target).await?;
                total_v4_pings += 1;
            }
        }

        for (name, target) in &external_v6_targets {
            info!("Testing IPv6 connectivity to {} ({})", name, target);

            for seq in 0..3 {
                send_ping_v6(&mut stream, &ip_pair, seq, *target).await?;
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
                            if let Some((reply_id, source, dest)) = is_icmp_echo_reply(&packet) {
                                if reply_id == identifier && dest == ip_pair.ipv4 {
                                    successful_v4_pings += 1;
                                    debug!("IPv4 reply from {}", source);
                                }
                            }

                            if let Some((reply_id, source, dest)) = is_icmp_v6_echo_reply(&packet) {
                                if reply_id == identifier && dest == ip_pair.ipv6 {
                                    successful_v6_pings += 1;
                                    debug!("IPv6 reply from {}", source);
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
            "IPv4 external connectivity: {}/{} pings successful ({:.1}%)",
            successful_v4_pings, total_v4_pings, v4_success_rate
        );
        info!(
            "IPv6 external connectivity: {}/{} pings successful ({:.1}%)",
            successful_v6_pings, total_v6_pings, v6_success_rate
        );

        assert!(successful_v4_pings > 0, "No IPv4 pings successful");
        assert!(
            v4_success_rate >= 75.0,
            "IPv4 success rate < 75% (got {:.1}%)",
            v4_success_rate
        );

        assert!(successful_v6_pings > 0, "No IPv6 pings successful");
        assert!(
            v6_success_rate >= 75.0,
            "IPv6 success rate < 75% (got {:.1}%)",
            v6_success_rate
        );

        stream.disconnect_stream().await;
        Ok(())
    }

    #[tokio::test]
    async fn split_dns_ping_checks() -> Result<(), Box<dyn std::error::Error>> {
        init_logging();

        let mut stream = IpMixStream::new(NetworkEnvironment::Mainnet).await?;
        let ip_pair = stream.connect_tunnel().await?;

        info!(
            "Connected with IPs - IPv4: {}, IPv6: {}",
            ip_pair.ipv4, ip_pair.ipv6
        );

        let (mut reader, mut writer) = stream.split();

        let external_v4_targets = vec![("Google DNS", Ipv4Addr::new(8, 8, 8, 8))];

        let identifier = icmp_identifier();
        let mut successful_v4_pings = 0;
        let mut total_v4_pings = 0;

        for (name, target) in &external_v4_targets {
            info!("Testing IPv4 connectivity to {} ({})", name, target);

            for seq in 0..2 {
                use crate::ip_packet_client::helpers::{
                    create_icmpv4_echo_request, wrap_icmp_in_ipv4,
                };
                use nym_ip_packet_requests::codec::MultiIpPacketCodec;
                use pnet_packet::Packet;

                let icmp_echo_request = create_icmpv4_echo_request(seq, identifier)?;
                let ipv4_packet = wrap_icmp_in_ipv4(icmp_echo_request, ip_pair.ipv4, *target)?;
                let bundled_packet =
                    MultiIpPacketCodec::bundle_one_packet(ipv4_packet.packet().to_vec().into());

                writer.send_ip_packet(&bundled_packet).await?;
                total_v4_pings += 1;
            }
        }

        let collect_timeout = tokio::time::sleep(Duration::from_secs(10));
        tokio::pin!(collect_timeout);

        loop {
            tokio::select! {
                _ = &mut collect_timeout => {
                    info!("Finished collecting responses");
                    break;
                }
                result = reader.handle_incoming() => {
                    if let Ok(packets) = result {
                        for packet in packets {
                            if let Some((reply_id, source, dest)) = is_icmp_echo_reply(&packet) {
                                if reply_id == identifier && dest == ip_pair.ipv4 {
                                    successful_v4_pings += 1;
                                    debug!("IPv4 reply from {}", source);
                                }
                            }
                        }
                    }
                }
            }
        }

        let v4_success_rate = if total_v4_pings > 0 {
            (successful_v4_pings as f64 / total_v4_pings as f64) * 100.0
        } else {
            0.0
        };

        info!(
            "Split test - IPv4 external connectivity: {}/{} pings successful ({:.1}%)",
            successful_v4_pings, total_v4_pings, v4_success_rate
        );

        assert!(successful_v4_pings > 0, "No pings successful");
        assert!(
            v4_success_rate >= 75.0,
            "IPv4 success rate < 75% (got {:.1}%)",
            v4_success_rate
        );

        Ok(())
    }
}
