// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0
use crate::ip_packet_client::{
    discovery::{create_nym_api_client, get_best_ipr, lookup_node_version},
    MixnetMessageOutcome,
};
use crate::mixnet::{MixnetClient, MixnetStream, Recipient};
use crate::Error;
use bytes::Bytes;
use nym_ip_packet_requests::response_helpers;
use nym_ip_packet_requests::{
    best_supported_version,
    v10::{self, response::IpPacketResponse as IpPacketResponseV10},
    v9::{self, response::IpPacketResponse},
    IpPair,
};
use nym_network_defaults::NymNetworkDetails;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tracing::{debug, info, warn};

const IPR_CONNECT_TIMEOUT: Duration = Duration::from_secs(60);

/// Cap on the v10 connect attempt when a v9 fallback is still reachable. During a
/// rollout the directory version can lead the running IPR (directory says 1.37,
/// process still v9), so a v10 request to it never gets answered; bound the wait
/// well under `IPR_CONNECT_TIMEOUT` so the fallback fires promptly rather than
/// after the full 60 s. Kept generously above a healthy v10 connect (~1-2 s) so a
/// merely-slow node isn't downgraded spuriously.
const IPR_V10_ATTEMPT_TIMEOUT: Duration = Duration::from_secs(15);

/// Env var overriding the node version used for IPR protocol gating (semver,
/// e.g. "1.37.0"). Debug knob for exercising a protocol version against a node
/// the directory reports as older, such as a local IPR that isn't a directory
/// node. Native SDK only; the wasm/smolmix build cannot read process env.
/// Mirrors `SMOLMIX_MTU`.
const SMOLMIX_IPR_VERSION_ENV: &str = "SMOLMIX_IPR_VERSION";

/// Parse the [`SMOLMIX_IPR_VERSION_ENV`] override; `None` if unset or unparseable.
fn forced_ipr_version() -> Option<semver::Version> {
    let raw = std::env::var(SMOLMIX_IPR_VERSION_ENV).ok()?;
    match semver::Version::parse(&raw) {
        Ok(version) => Some(version),
        Err(e) => {
            warn!("ignoring unparseable {SMOLMIX_IPR_VERSION_ENV}='{raw}': {e}");
            None
        }
    }
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
///       → MixnetStream.recv() → IpPacketResponse bytes
///       → handle_ipr_response() → extract IP packets
/// ```
pub struct IpMixStream {
    stream: MixnetStream,
    client: MixnetClient,
    allocated_ips: IpPair,
    /// MTU reported by the IPR in a v10 connect response; `None` against a v9 IPR.
    negotiated_mtu: Option<u16>,
    /// The IPR protocol version this tunnel connected with (v9 or v10). Stamped
    /// on outgoing data requests and expected on every inbound frame: the IPR
    /// mirrors the connect version on all traffic for the connection's lifetime.
    protocol_version: u8,
    connected: bool,
}

impl IpMixStream {
    /// Discover the best IPR, connect through the mixnet, and establish the IP tunnel.
    ///
    /// Returns a ready-to-use tunnel with allocated IP addresses.
    pub async fn new() -> Result<Self, Error> {
        let network_defaults = NymNetworkDetails::new_mainnet();
        let api_urls = network_defaults.nym_api_urls();

        if api_urls.is_empty() {
            return Err(Error::NoNymAPIUrl);
        }

        let api_client = create_nym_api_client(api_urls)?;
        let (ipr_address, node_version) = get_best_ipr(api_client).await?;
        Self::connect(ipr_address, Some(node_version)).await
    }

    /// Connect to a specific IPR address.
    ///
    /// Use this when you already know the IPR `Recipient` address (e.g. for
    /// testing against a specific exit node). For automatic discovery, use
    /// [`IpMixStream::new`] instead.
    pub async fn new_with_ipr(ipr_address: Recipient) -> Result<Self, Error> {
        nym_network_defaults::setup_env(None::<&str>);
        let node_version = Self::lookup_ipr_version(&ipr_address).await;
        Self::connect(ipr_address, node_version).await
    }

    /// Best-effort version lookup for an explicit IPR address, from the directory
    /// of the env-configured network. `None` (node not in the directory: custom
    /// deployment, brand-new node, or lookup failure) leaves connect defaulting to
    /// v9 rather than hard-failing.
    async fn lookup_ipr_version(ipr_address: &Recipient) -> Option<semver::Version> {
        let urls = NymNetworkDetails::new_from_env().nym_api_urls();
        if urls.is_empty() {
            return None;
        }

        let api_client = create_nym_api_client(urls).ok()?;
        match lookup_node_version(&api_client, ipr_address.gateway()).await {
            Ok(version) => Some(version),
            // Distinguish a transient nym-api failure from a legitimately-absent
            // node in the logs; both degrade to v9, so keep returning None.
            Err(e) => {
                debug!("IPR version lookup failed ({e}); defaulting to v9");
                None
            }
        }
    }

    /// Open the mixnet stream and run the version-gated connect handshake.
    async fn connect(
        ipr_address: Recipient,
        node_version: Option<semver::Version>,
    ) -> Result<Self, Error> {
        let mut client = MixnetClient::connect_new().await?;
        let mut stream = client.open_stream(ipr_address, Some(10)).await?;

        info!("Connecting to IP packet router at {ipr_address}");
        let (allocated_ips, negotiated_mtu, protocol_version) =
            Self::connect_tunnel(&mut stream, node_version.as_ref()).await?;
        info!(
            "Connected — IPv4: {}, IPv6: {}, MTU: {}",
            allocated_ips.ipv4,
            allocated_ips.ipv6,
            negotiated_mtu
                .map(|m| m.to_string())
                .unwrap_or_else(|| "v9 (unreported)".into())
        );

        Ok(Self {
            stream,
            client,
            allocated_ips,
            negotiated_mtu,
            protocol_version,
            connected: true,
        })
    }

    pub fn nym_address(&self) -> &Recipient {
        self.client.nym_address()
    }

    pub fn allocated_ips(&self) -> &IpPair {
        &self.allocated_ips
    }

    /// The MTU the IPR reported in its v10 connect response, or `None` if the IPR
    /// only speaks v9 (in which case the caller applies a conservative default).
    pub fn negotiated_mtu(&self) -> Option<u16> {
        self.negotiated_mtu
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

    /// Connect using the IPR protocol version the node's release supports, chosen
    /// from its directory version rather than a per-connect probe. `None` version
    /// (unknown node) defaults to v9. Returns the allocated IPs, the MTU (v10
    /// only), and the protocol version the tunnel settled on.
    async fn connect_tunnel(
        stream: &mut MixnetStream,
        node_version: Option<&semver::Version>,
    ) -> Result<(IpPair, Option<u16>, u8), Error> {
        let forced = forced_ipr_version();
        let node_version = forced.as_ref().or(node_version);
        if node_version.is_none() {
            debug!("no directory version for IPR node; connecting v9 (MTU unreported)");
        }

        // Fall through to the v9 connect below unless we settle on v10. A v10
        // timeout also falls through, since the directory version can lead the
        // running IPR (mid-upgrade, stale describe cache).
        match node_version.and_then(best_supported_version) {
            Some(v) if v == v10::VERSION => {
                match Self::connect_v10(stream, IPR_V10_ATTEMPT_TIMEOUT).await {
                    Ok((ips, mtu)) => return Ok((ips, Some(mtu), v10::VERSION)),
                    Err(Error::IPRConnectResponseTimeout) => {
                        debug!("v10 connect timed out; retrying v9");
                    }
                    Err(e) => return Err(e),
                }
            }
            Some(v) if v == v9::VERSION => {}
            Some(other) => warn!("node advertises unsupported IPR version v{other}; connecting v9"),
            None => {} // unknown node, logged above
        }
        let ips = Self::connect_v9(stream).await?;
        Ok((ips, None, v9::VERSION))
    }

    /// v10 connect: reports the IPR's accepted MTU alongside the allocated IPs.
    /// `deadline` bounds the wait so a v9 fallback stays responsive (see
    /// [`IPR_V10_ATTEMPT_TIMEOUT`]).
    async fn connect_v10(
        stream: &mut MixnetStream,
        deadline: Duration,
    ) -> Result<(IpPair, u16), Error> {
        let (request, request_id) = v10::new_connect_request(None);
        debug!("Sending v10 connect request with ID: {request_id}");

        let request_bytes = request.to_bytes()?;
        stream
            .write_all(&request_bytes)
            .await
            .map_err(|_| Error::MessageSendingFailure)?;

        let timeout = tokio::time::sleep(deadline);
        tokio::pin!(timeout);

        loop {
            tokio::select! {
                _ = &mut timeout => {
                    return Err(Error::IPRConnectResponseTimeout);
                }
                result = stream.recv() => {
                    let data = result.ok_or(Error::IPRClientStreamClosed)?;

                    // Ignore stragglers from an earlier version; we selected v10
                    // from the node's directory version.
                    if data.first() != Some(&v10::VERSION) {
                        continue;
                    }

                    if let Ok(response) = IpPacketResponseV10::from_bytes(&data) {
                        if response.id() == Some(request_id) {
                            return response_helpers::parse_connect_response_v10(response)
                                .map(|success| (success.ips, success.mtu))
                                .map_err(|e| match e {
                                    response_helpers::IprResponseError::ConnectDenied(r) => Error::ConnectDenied(r),
                                    response_helpers::IprResponseError::UnexpectedResponse(d) => Error::UnexpectedResponseType(d),
                                    other => Error::IPRMessageVersionCheckFailed(other.to_string()),
                                });
                        }
                    }
                }
            }
        }
    }

    /// v9 connect: pre-v10 path, no MTU reported.
    async fn connect_v9(stream: &mut MixnetStream) -> Result<IpPair, Error> {
        let (request, request_id) = v9::new_connect_request(None);
        debug!("Sending v9 connect request with ID: {request_id}");

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

                    // Skip frames from another version rather than aborting: in the
                    // v10-to-v9 fallback a late v10 response can land here, and it
                    // must not kill a v9 connect that would otherwise succeed.
                    if data.first() != Some(&v9::VERSION) {
                        debug!("ignoring frame with unexpected version during v9 connect");
                        continue;
                    }
                    if let Ok(response) = IpPacketResponse::from_bytes(&data) {
                        if response.id() == Some(request_id) {
                            return response_helpers::parse_connect_response(response)
                                .map_err(|e| match e {
                                    response_helpers::IprResponseError::ConnectDenied(r) => Error::ConnectDenied(r),
                                    response_helpers::IprResponseError::UnexpectedResponse(d) => Error::UnexpectedResponseType(d),
                                    other => Error::IPRMessageVersionCheckFailed(other.to_string()),
                                });
                        }
                    }
                }
            }
        }
    }

    /// Send an IP packet through the tunnel, stamped with the connect-time
    /// protocol version so requests and responses stay on one version.
    pub async fn send_ip_packet(&mut self, packet: &[u8]) -> Result<(), Error> {
        self.check_connected()?;
        let request = if self.protocol_version == v10::VERSION {
            v10::new_data_request(packet.to_vec().into())
        } else {
            v9::new_data_request(packet.to_vec().into())
        };
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

        // The IPR mirrors the connect-time version on all traffic (data included),
        // so gate on the negotiated version, not a fixed one. Skip a mismatched
        // frame rather than tearing down the tunnel over one stray frame.
        if data.first() != Some(&self.protocol_version) {
            warn!(
                "ignoring frame with version {:?}, expected v{}",
                data.first(),
                self.protocol_version
            );
            return Ok(Vec::new());
        }

        match response_helpers::handle_ipr_response(&data) {
            Some(MixnetMessageOutcome::IpPackets(packets)) => {
                debug!("Extracted {} IP packets", packets.len());
                Ok(packets)
            }
            Some(MixnetMessageOutcome::Disconnect) => {
                info!("Received disconnect");
                self.connected = false;
                Err(Error::IprTunnelDisconnected)
            }
            None => Ok(Vec::new()),
        }
    }

    /// Disconnect from the Mixnet. Disconnected clients cannot be reconnected.
    pub async fn disconnect(self) {
        debug!("Disconnecting");
        self.client.disconnect().await;
        debug!("Disconnected");
    }
}
