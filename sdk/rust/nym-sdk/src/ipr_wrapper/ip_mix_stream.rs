// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::network_env::NetworkEnvironment;
use crate::ip_packet_client::{
    discovery::{create_nym_api_client, get_best_ipr, parse_connect_response},
    handle_ipr_response,
    listener::check_ipr_message_version,
    MixnetMessageOutcome,
};
use crate::mixnet::{MixnetClient, MixnetStream, Recipient};
use crate::Error;

use bytes::Bytes;
use nym_ip_packet_requests::{
    v8::{request::IpPacketRequest, response::IpPacketResponse},
    IpPair,
};
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tracing::{debug, info};

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
///       → handle_ipr_response() → extract IP packets
/// ```
pub struct IpMixStream {
    stream: MixnetStream,
    client: MixnetClient,
    allocated_ips: IpPair,
    connected: bool,
}

impl IpMixStream {
    /// Discover the best IPR, connect through the mixnet, and establish the IP tunnel.
    ///
    /// Returns a ready-to-use tunnel with allocated IP addresses.
    pub async fn new(env: NetworkEnvironment) -> Result<Self, Error> {
        let network_defaults = env.network_defaults();
        let api_client = create_nym_api_client(network_defaults.nym_api_urls.unwrap_or_default())?;
        let ipr_address = get_best_ipr(api_client).await?;
        Self::new_with_ipr(env, ipr_address).await
    }

    /// Connect to a specific IPR address.
    ///
    /// Use this when you already know the IPR `Recipient` address (e.g. for
    /// testing against a specific exit node). For automatic discovery, use
    /// [`IpMixStream::new`] instead.
    pub async fn new_with_ipr(
        env: NetworkEnvironment,
        ipr_address: Recipient,
    ) -> Result<Self, Error> {
        nym_network_defaults::setup_env(Some(env.env_file_path()?));
        let mut client = MixnetClient::connect_new().await?;
        let mut stream = client.open_stream(ipr_address, Some(10)).await?;

        info!("Connecting to IP packet router at {ipr_address}");
        let allocated_ips = Self::connect_tunnel(&mut stream).await?;
        info!(
            "Connected — IPv4: {}, IPv6: {}",
            allocated_ips.ipv4, allocated_ips.ipv6
        );

        Ok(Self {
            stream,
            client,
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

                    check_ipr_message_version(&data)?;
                    if let Ok(response) = IpPacketResponse::from_bytes(&data) {
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

        match handle_ipr_response(&data) {
            Ok(Some(MixnetMessageOutcome::IpPackets(packets))) => {
                debug!("Extracted {} IP packets", packets.len());
                Ok(packets)
            }
            Ok(Some(MixnetMessageOutcome::Disconnect)) => {
                info!("Received disconnect");
                self.connected = false;
                Err(Error::IprTunnelDisconnected)
            }
            Ok(None) => Ok(Vec::new()),
            Err(e) => Err(e),
        }
    }

    /// Disconnect from the Mixnet. Disconnected clients cannot be reconnected.
    pub async fn disconnect(self) {
        debug!("Disconnecting");
        self.client.disconnect().await;
        debug!("Disconnected");
    }
}
