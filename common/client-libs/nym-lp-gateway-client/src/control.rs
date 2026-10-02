// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! A client's LP transport: the connections, the socket, and nothing else.

use crate::config::LpGatewayClientConfig;
use crate::error::{LpClientError, Result};
use crate::nested_session::connection::NestedConnection;
use nym_lp::LpTransportSession;
use nym_lp::peer::{LpLocalPeer, LpRemotePeer};
use nym_lp::psq::initiator::HandshakeMode;
use nym_lp::transport::traits::LpTransportChannel;
use nym_lp::transport::{LpHandshakeChannel, LpTransportError};
use nym_lp_data::packet::{EncryptedLpPacket, version};
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpStream;
use tracing::warn;

/// The client's control connections to the gateways it talks to.
///
/// Deliberately knows nothing about encryption. It opens control connections, runs handshakes and
/// **hands the resulting [`LpTransportSession`] to whoever asked for it**. Nothing that decides
/// what a packet means belongs here, and neither does carrying one: data goes over
/// [`LpDataSocket`](crate::LpDataSocket), which is a separate thing because the two planes want
/// opposite things from ownership.
///
/// A control connection ([`LpTransportChannel`]) is a stream, one per gateway, request/response -
/// the handshake needs ordering and delivery, and so does anything expecting an answer. That makes
/// it `&mut self` throughout and not shareable. `TcpStream` in production, generic so a test can
/// swap in an in-memory pair.
///
/// The connection is only a carrier. Once a handshake (and, for a mixnet client, a registration)
/// is done, it can be closed - the session it produced is used over the data socket from then on.
///
/// # Example
/// ```ignore
/// let mut client = LpGatewayControlClient::<TcpStream>::new(config);
/// client.connect(gateway).await?;
///
/// // the session is the caller's from here on
/// let mut session = client
///     .handshake(gateway, local_peer, remote_peer, lp_version, HandshakeMode::OneWayEntry)
///     .await?;
///
/// // ... register over the control connection, then drop it ...
/// client.disconnect(gateway);
/// ```
pub struct LpGatewayControlClient<S = TcpStream> {
    /// Live control connections, one per gateway currently being talked to.
    control: HashMap<SocketAddr, S>,

    /// Timeouts and TCP parameters.
    pub config: LpGatewayClientConfig,
}

impl<S> LpGatewayControlClient<S>
where
    S: LpTransportChannel + LpHandshakeChannel + Unpin,
{
    pub fn new(config: LpGatewayClientConfig) -> Self {
        LpGatewayControlClient {
            control: HashMap::new(),
            config,
        }
    }

    pub fn new_with_default_config() -> Self {
        Self::new(LpGatewayClientConfig::default())
    }

    // -------------------------------------------------------------------------
    // Control plane
    // -------------------------------------------------------------------------

    /// Open a control connection to a gateway, if there isn't one already.
    pub async fn connect(&mut self, gateway: SocketAddr) -> Result<()> {
        let Entry::Vacant(entry) = self.control.entry(gateway) else {
            return Ok(());
        };

        tracing::debug!("opening control connection to {gateway}");

        let mut stream = tokio::time::timeout(self.config.connect_timeout, S::connect(gateway))
            .await
            .map_err(|_| LpClientError::TcpConnection {
                address: gateway.to_string(),
                source: LpTransportError::ConnectionFailure(format!(
                    "Connection timeout after {:?}",
                    self.config.connect_timeout
                )),
            })?
            .map_err(|source| LpClientError::TcpConnection {
                address: gateway.to_string(),
                source,
            })?;

        // Set TCP_NODELAY for low latency
        stream
            .set_no_delay(self.config.tcp_nodelay)
            .map_err(|source| LpClientError::TcpConnection {
                address: gateway.to_string(),
                source,
            })?;

        entry.insert(stream);
        tracing::debug!("control connection established to {gateway}");
        Ok(())
    }

    /// Close the control connection to a gateway, signalling EOF.
    ///
    /// Safe to call for a gateway that was never connected. Any session established over that
    /// connection is unaffected - it belongs to the caller and outlives this.
    pub fn disconnect(&mut self, gateway: SocketAddr) {
        if self.control.remove(&gateway).is_some() {
            tracing::debug!("closed control connection to {gateway}");
        }
    }

    /// Close every control connection.
    pub fn disconnect_all(&mut self) {
        self.control.clear();
    }

    pub fn is_connected(&self, gateway: SocketAddr) -> bool {
        self.control.contains_key(&gateway)
    }

    /// The live control connection to a gateway.
    // Only exposed so integration tests can reach the other end of a mock stream.
    #[doc(hidden)]
    pub fn connection(&self, gateway: SocketAddr) -> Option<&S> {
        self.control.get(&gateway)
    }

    fn connection_mut(&mut self, gateway: SocketAddr) -> Result<&mut S> {
        self.control
            .get_mut(&gateway)
            .ok_or(LpClientError::NotConnected { gateway })
    }

    /// Run the KKT/PSQ handshake with a gateway and hand the session over.
    ///
    /// Opens the control connection if it is not already open, and drops it on failure: a
    /// half-finished handshake cannot be resumed, and keeping the connection would leave the
    /// gateway holding a session this client never got.
    pub async fn handshake(
        &mut self,
        gateway: SocketAddr,
        local_peer: LpLocalPeer,
        remote_peer: LpRemotePeer,
        gateway_lp_protocol_version: u8,
        mode: HandshakeMode,
    ) -> Result<LpTransportSession> {
        let version = version::negotiate(gateway_lp_protocol_version).ok_or(
            LpClientError::UnsupportedProtocolVersion {
                advertised: gateway_lp_protocol_version,
            },
        )?;

        if version != gateway_lp_protocol_version {
            warn!(
                "gateway {gateway} suggested LP protocol {gateway_lp_protocol_version}; speaking {version} instead"
            );
        }

        let result = tokio::time::timeout(
            self.config.handshake_timeout,
            self.handshake_inner(gateway, local_peer, remote_peer, version, mode),
        )
        .await;

        match result {
            Ok(Ok(session)) => Ok(session),
            Ok(Err(e)) => {
                self.disconnect(gateway);
                Err(e)
            }
            Err(_) => {
                self.disconnect(gateway);
                Err(LpClientError::HandshakeTimeout)
            }
        }
    }

    async fn handshake_inner(
        &mut self,
        gateway: SocketAddr,
        local_peer: LpLocalPeer,
        remote_peer: LpRemotePeer,
        version: u8,
        mode: HandshakeMode,
    ) -> Result<LpTransportSession> {
        tracing::debug!("starting LP handshake with {gateway} as initiator");

        self.connect(gateway).await?;
        let connection = self.connection_mut(gateway)?;

        let session = LpTransportSession::psq_handshake_initiator(
            connection,
            local_peer,
            remote_peer,
            version,
            mode,
        )?
        .complete_handshake()
        .await?;

        Ok(session)
    }

    /// Send an already-encrypted packet on a gateway's control connection.
    pub async fn send_control(
        &mut self,
        gateway: SocketAddr,
        packet: &EncryptedLpPacket,
    ) -> Result<()> {
        self.connection_mut(gateway)?
            .send_length_prefixed_transport_packet(packet)
            .await?;
        Ok(())
    }

    /// Read the next packet off a gateway's control connection.
    pub async fn receive_control(&mut self, gateway: SocketAddr) -> Result<EncryptedLpPacket> {
        Ok(self
            .connection_mut(gateway)?
            .receive_length_prefixed_transport_packet()
            .await?)
    }

    /// One packet out and one back on a gateway's control connection.
    pub async fn exchange_control(
        &mut self,
        gateway: SocketAddr,
        packet: &EncryptedLpPacket,
        timeout: Duration,
    ) -> Result<EncryptedLpPacket> {
        tokio::time::timeout(timeout, async {
            self.send_control(gateway, packet).await?;
            self.receive_control(gateway).await
        })
        .await
        .map_err(|_| LpClientError::ConnectionTimeout)?
    }

    /// Treat a gateway's control connection as a channel to an exit gateway behind it.
    ///
    /// `outer_session` is the session with `gateway`: it encrypts the forwarding envelope, so it
    /// has to be borrowed in rather than found here.
    pub fn as_nested_connection<'a>(
        &'a mut self,
        gateway: SocketAddr,
        exit_address: SocketAddr,
        outer_session: &'a mut LpTransportSession,
    ) -> NestedConnection<'a, S> {
        NestedConnection {
            exit_address,
            outer_gateway: gateway,
            outer_client: self,
            outer_session,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_client_has_no_connections() {
        let client = LpGatewayControlClient::<TcpStream>::new_with_default_config();
        let gateway = "127.0.0.1:41264".parse().unwrap();

        assert!(!client.is_connected(gateway));
    }
}
