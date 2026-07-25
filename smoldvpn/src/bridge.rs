// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! QUIC bridge client — an alternative WireGuard data-plane transport for
//! clients blocked from pure UDP.
//!
//! The QUIC connection itself — cert-pinning verifier (ed25519 identity, SNI ∈
//! cert alt-names, cert SPKI == pinned key), ALPN `hq-29`, endpoint bind and
//! dial — is delegated to the canonical [`nym_bridges`] client
//! ([`nym_bridges::transport::quic::transport_conn`]) rather than reimplemented
//! here, so this crate can never drift from the bridge server. This module only
//! adds the datapath framing on top: one reliable `open_bi()` stream carrying
//! WireGuard packets, each prefixed by a 2-byte big-endian length.
//!
//! Only ever fronts the two-hop entry leg (the bridge is bound 1:1 to a gateway
//! and forwards to its WireGuard port); there is no QUIC one-hop mode and no
//! gateway-selection handshake.
//!
//! Note: [`nym_bridges`] dials the first address it is given and binds dual-stack
//! `[::]:0`; it does not set QUIC keep-alive/BBR. The directory lists a bridge's
//! IPv6 address first, but clients are IPv4-only for now, so [`connect`] reorders
//! the candidates to put IPv4 first before handing them over. WireGuard's own
//! persistent-keepalive keeps the long-lived session and its NAT mapping alive.

use std::net::SocketAddr;
use std::sync::Once;
use std::time::Duration;

use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use nym_bridges::transport::quic::{transport_conn, ClientOptions};
use quinn::{Connection, RecvStream, SendStream};
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};
use tokio_util::sync::CancellationToken;

use crate::error::{DvpnError, Result};

/// Per-WireGuard-packet length prefix width, big-endian.
const LENGTH_DELIMITER_BYTELEN: usize = 2;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

static INSTALL_PROVIDER: Once = Once::new();

/// Bridge connection parameters, sourced from the gateway directory / VPN API.
#[derive(Clone, Debug)]
pub struct BridgeParams {
    /// Candidate bridge socket addresses. The directory lists a bridge's IPv6
    /// address first; [`connect`] reorders these to prefer IPv4 (clients are
    /// IPv4-only for now) before the bridge client dials the first one.
    pub addresses: Vec<SocketAddr>,
    /// SNI host to present (falls back to the bridge IP string if `None`).
    pub sni_host: Option<String>,
    /// Pinned ed25519 identity public key, standard-base64 as carried in the
    /// gateway directory. The `nym_bridges` client decodes and verifies it
    /// against the server certificate at connect time.
    pub id_pubkey_base64: String,
}

/// Sending half of the QUIC bridge transport. Holds the connection so the QUIC
/// session (and its endpoint driver) stays alive for the lifetime of the
/// datapath.
pub(crate) struct QuicBridgeSender {
    framed: FramedWrite<SendStream, LengthDelimitedCodec>,
    _conn: Connection,
}

/// Receiving half of the QUIC bridge transport.
pub(crate) struct QuicBridgeReceiver {
    framed: FramedRead<RecvStream, LengthDelimitedCodec>,
    _conn: Connection,
}

impl QuicBridgeSender {
    pub(crate) async fn send(&mut self, packet: &[u8]) -> Result<()> {
        self.framed
            .send(Bytes::copy_from_slice(packet))
            .await
            .map_err(|e| DvpnError::Transport(format!("bridge send: {e}")))
    }
}

impl QuicBridgeReceiver {
    pub(crate) async fn recv(&mut self) -> Result<Vec<u8>> {
        match self.framed.next().await {
            Some(Ok(frame)) => Ok(frame.to_vec()),
            Some(Err(e)) => Err(DvpnError::Transport(format!("bridge recv: {e}"))),
            None => Err(DvpnError::Transport("bridge stream closed".into())),
        }
    }
}

fn framed_codec() -> LengthDelimitedCodec {
    LengthDelimitedCodec::builder()
        .length_field_length(LENGTH_DELIMITER_BYTELEN)
        .new_codec()
}

/// Diagnostic: attempt a full bridge connect (real ed25519 cert pinning + open
/// the WireGuard bi-stream), then drop it. `Ok(())` means the QUIC handshake and
/// stream open succeeded. Useful for testing QUIC-gateway reachability without
/// bringing up a whole tunnel (see the `quic-probe` example).
pub async fn probe(params: &BridgeParams, cancel: &CancellationToken) -> Result<()> {
    let (_send, _recv) = connect(params, cancel).await?;
    Ok(())
}

/// Connect to the bridge (via the `nym_bridges` client) and open the single
/// WireGuard-carrying bi-stream. Cancellable via `cancel`.
pub(crate) async fn connect(
    params: &BridgeParams,
    cancel: &CancellationToken,
) -> Result<(QuicBridgeSender, QuicBridgeReceiver)> {
    // `nym_bridges` builds a rustls `ClientConfig` via the process-default crypto
    // provider; install ring before the first connect.
    INSTALL_PROVIDER.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });

    // The bridge client dials the first address; the directory lists IPv6 first but clients are
    // IPv4-only for now, so prefer IPv4 (stable sort keeps relative order within each family).
    let mut addresses = params.addresses.clone();
    addresses.sort_by_key(|a| a.is_ipv6());

    let options = ClientOptions {
        addresses,
        host: params.sni_host.clone(),
        id_pubkey: params.id_pubkey_base64.clone(),
    };

    let conn: Connection = tokio::select! {
        _ = cancel.cancelled() => return Err(DvpnError::Cancelled),
        r = tokio::time::timeout(CONNECT_TIMEOUT, transport_conn(&options)) => {
            r.map_err(|_| DvpnError::Bridge("connect timed out".into()))?
             .map_err(|e| DvpnError::Bridge(format!("connect: {e}")))?
        }
    };

    // Bound `open_bi()` by the same timeout + cancellation as the handshake: quinn can block it
    // waiting for bidirectional-stream credit, so a stalled/adversarial bridge must not be able to
    // leave `connect` hanging past the caller's cancel or the connect deadline.
    let (send, recv) = tokio::select! {
        _ = cancel.cancelled() => return Err(DvpnError::Cancelled),
        r = tokio::time::timeout(CONNECT_TIMEOUT, conn.open_bi()) => {
            r.map_err(|_| DvpnError::Bridge("open_bi timed out".into()))?
             .map_err(|e| DvpnError::Bridge(format!("open_bi: {e}")))?
        }
    };

    // quinn's SendStream/RecvStream implement tokio AsyncWrite/AsyncRead, so they
    // frame directly. Park the connection in both halves so the endpoint driver
    // keeps running for the lifetime of the datapath (quinn keeps the driver
    // alive while any connection is live, even after its `Endpoint` is dropped).
    Ok((
        QuicBridgeSender {
            framed: FramedWrite::new(send, framed_codec()),
            _conn: conn.clone(),
        },
        QuicBridgeReceiver {
            framed: FramedRead::new(recv, framed_codec()),
            _conn: conn,
        },
    ))
}

#[cfg(test)]
mod tests {
    //! Conformance test (OpenSpec task 6.6): the real inline bridge client
    //! against a real `nym_bridges` QUIC server. Proves ALPN `hq-29`,
    //! 2-byte-BE framing over one bi-stream, and ed25519 cert pinning (positive
    //! + negative), all through the crate's own `connect`.

    use super::*;
    use base64::prelude::{Engine as _, BASE64_STANDARD};
    use futures::{SinkExt, StreamExt};
    use nym_bridges::transport::quic::{create_endpoint, ServerConfig};

    /// Spawn a `nym_bridges` QUIC bridge server (fixed identity) that echoes each
    /// framed WireGuard packet back. Returns its address, the pinned public key
    /// (standard-base64), and the base58 SNI its self-signed certificate is
    /// issued for.
    fn spawn_mock_bridge() -> (SocketAddr, String, String) {
        INSTALL_PROVIDER.call_once(|| {
            let _ = rustls::crypto::ring::default_provider().install_default();
        });

        // Any 32 bytes are a valid ed25519 seed; use a fixed one for determinism.
        let secret = [7u8; 32];
        let cfg = ServerConfig {
            identity_key: Some(BASE64_STANDARD.encode(secret)),
            listen: "127.0.0.1:0".parse().unwrap(),
            ..Default::default()
        };
        let id_pubkey_base64 = cfg.get_id_pubkey().unwrap();
        // The server issues its cert with CN/SAN = base58(identity key), so that
        // is the only SNI the pinning verifier will accept.
        let sni = bs58::encode(BASE64_STANDARD.decode(&id_pubkey_base64).unwrap()).into_string();

        let endpoint = create_endpoint(&cfg).unwrap();
        let addr = endpoint.local_addr().unwrap();

        tokio::spawn(async move {
            let _endpoint = endpoint.clone();
            if let Some(incoming) = endpoint.accept().await {
                if let Ok(conn) = incoming.await {
                    if let Ok((send, recv)) = conn.accept_bi().await {
                        let mut w = FramedWrite::new(send, framed_codec());
                        let mut r = FramedRead::new(recv, framed_codec());
                        while let Some(Ok(frame)) = r.next().await {
                            if w.send(Bytes::from(frame.to_vec())).await.is_err() {
                                break;
                            }
                        }
                    }
                }
            }
        });
        (addr, id_pubkey_base64, sni)
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn bridge_framing_and_pinning_roundtrip() {
        let (addr, id_pubkey_base64, sni) = spawn_mock_bridge();
        let params = BridgeParams {
            addresses: vec![addr],
            sni_host: Some(sni),
            id_pubkey_base64,
        };
        let cancel = CancellationToken::new();
        let (mut sender, mut receiver) = connect(&params, &cancel).await.expect("bridge connect");

        let wg: Vec<u8> = (0u16..1200).map(|i| (i % 251) as u8).collect();
        sender.send(&wg).await.expect("send");
        let echo = receiver.recv().await.expect("recv");
        assert_eq!(echo, wg, "framed round-trip mismatch");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn bridge_rejects_wrong_pin() {
        let (addr, id_pubkey_base64, sni) = spawn_mock_bridge();
        // Corrupt the pinned key while keeping the (correct) SNI.
        let mut bytes = BASE64_STANDARD.decode(&id_pubkey_base64).unwrap();
        bytes[0] ^= 0xFF;
        let params = BridgeParams {
            addresses: vec![addr],
            sni_host: Some(sni),
            id_pubkey_base64: BASE64_STANDARD.encode(bytes),
        };
        let cancel = CancellationToken::new();
        assert!(
            connect(&params, &cancel).await.is_err(),
            "connection must be rejected with a mismatched pin"
        );
    }
}
