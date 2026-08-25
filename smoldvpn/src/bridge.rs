// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Bridge client — an alternative WireGuard data-plane transport for clients
//! blocked from pure UDP.
//!
//! The bridge connection itself — cert-pinning verifier (ed25519 identity, SNI
//! ∈ cert alt-names, cert SPKI == pinned key), ALPN, endpoint bind and dial,
//! IPv4 endpoint selection — is delegated to the canonical [`nym_bridges`]
//! client ([`nym_bridges::connection::BridgeConn`]) rather than reimplemented
//! here, so this crate can never drift from the bridge server and picks up
//! new transports (today: `quic_plain` and `tls_plain`, both implemented by
//! `BridgeConn::try_connect`) without changes here. This module only adds the
//! datapath framing on top of `BridgeConn`'s raw duplex stream: one reliable
//! stream carrying WireGuard packets, each prefixed by a 2-byte big-endian
//! length — the same convention `nym_bridges`'s own `UdpForwarder` uses
//! internally.
//!
//! Only ever fronts the two-hop entry leg (the bridge is bound 1:1 to a gateway
//! and forwards to its WireGuard port); there is no bridge one-hop mode and no
//! gateway-selection handshake.
//!
//! Note: for the QUIC transport, `BridgeConn` picks the first IPv4 address
//! among the candidates and errors if none is present (clients are IPv4-only
//! for now, so an IPv6-only bridge just isn't usable yet; the TLS transport
//! has no such restriction). `nym_bridges` itself sets QUIC keep-alive (20s)
//! and BBR congestion control on that connection; WireGuard's own
//! persistent-keepalive is a separate, higher-layer mechanism that keeps the
//! long-lived session and its NAT mapping alive.

use std::sync::{Arc, Mutex, Once};
use std::time::Duration;

use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use nym_bridges::connection::{BridgeConn, TransportCloser};
use nym_bridges::error::TransportError;
use nym_bridges::types::ClientConfig;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};
use tokio_util::sync::CancellationToken;

use crate::error::{DvpnError, Result};

/// Per-WireGuard-packet length prefix width, big-endian.
const LENGTH_DELIMITER_BYTELEN: usize = 2;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

static INSTALL_PROVIDER: Once = Once::new();

/// Bridge connection parameters, sourced from the gateway directory / VPN API.
///
/// This *is* `nym_bridges`'s own [`ClientConfig`] (in turn
/// `nym_bridges_types::ClientConfig`, the shape shared with
/// nym-node-status-api and nym-sdk-session) rather than a locally duplicated
/// struct — one variant per transport kind (`QuicPlain`/`TlsPlain`), each
/// carrying candidate addresses, an optional SNI host override, and the pinned
/// ed25519 identity public key (standard-base64) verified against the server
/// certificate at connect time.
pub type BridgeParams = ClientConfig;

/// Ends the underlying `nym_bridges` transport connection (see
/// [`TransportCloser`]) once both the [`BridgeSender`] and [`BridgeReceiver`]
/// halves that share it have been dropped. This is distinct from shutting
/// down the reader/writer halves themselves: for QUIC, `reader`/`writer` are
/// only one stream on a connection that outlives them, and skipping this
/// step means the connection is never told to close and falls back on its
/// own idle timeout instead.
///
/// The `Mutex` only exists to make this `Sync` (`dyn TransportCloser` isn't)
/// so `Arc<BridgeCloser>` can be held across the `.await`s in the sender/
/// receiver tasks; it's touched once, in `drop`.
struct BridgeCloser(Mutex<Option<Box<dyn TransportCloser>>>);

impl Drop for BridgeCloser {
    fn drop(&mut self) {
        if let Some(closer) = self.0.get_mut().unwrap_or_else(|e| e.into_inner()).take() {
            tokio::spawn(closer.close());
        }
    }
}

/// Sending half of the bridge transport.
pub(crate) struct BridgeSender {
    framed: FramedWrite<Box<dyn AsyncWrite + Send + Unpin>, LengthDelimitedCodec>,
    _closer: Arc<BridgeCloser>,
}

/// Receiving half of the bridge transport.
pub(crate) struct BridgeReceiver {
    framed: FramedRead<Box<dyn AsyncRead + Send + Unpin>, LengthDelimitedCodec>,
    _closer: Arc<BridgeCloser>,
}

impl BridgeSender {
    pub(crate) async fn send(&mut self, packet: &[u8]) -> Result<()> {
        self.framed
            .send(Bytes::copy_from_slice(packet))
            .await
            .map_err(|e| DvpnError::Transport(format!("bridge send: {e}")))
    }
}

impl BridgeReceiver {
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
/// the WireGuard bi-stream), then drop it. `Ok(())` means the handshake and
/// stream open succeeded. Useful for testing bridge-gateway reachability
/// without bringing up a whole tunnel (see the `quic-probe` example).
pub async fn probe(params: &BridgeParams, cancel: &CancellationToken) -> Result<()> {
    let (_send, _recv) = connect(params, cancel).await?;
    Ok(())
}

/// Connect to the bridge (via the `nym_bridges` client) and open the single
/// WireGuard-carrying bi-stream. Cancellable via `cancel`.
pub(crate) async fn connect(
    params: &BridgeParams,
    cancel: &CancellationToken,
) -> Result<(BridgeSender, BridgeReceiver)> {
    // `nym_bridges` builds a rustls `ClientConfig` via the process-default crypto
    // provider; install ring before the first connect.
    INSTALL_PROVIDER.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });

    // `BridgeConn::try_connect` is itself cancel-aware (it races the dial and the
    // stream open against `cancel`), so the timeout is the only wrapping needed here.
    let bridge_conn = tokio::time::timeout(CONNECT_TIMEOUT, dial(params.clone(), cancel.clone()))
        .await
        .map_err(|_| DvpnError::Bridge("connect timed out".into()))?
        .map_err(|e| match e {
            TransportError::Cancelled => DvpnError::Cancelled,
            e => DvpnError::Bridge(format!("connect: {e}")),
        })?;

    // `closer` must be closed once both halves are done with the connection
    // (see `BridgeCloser`'s docs) -- shared via `Arc` so that happens when
    // whichever of `BridgeSender`/`BridgeReceiver` is dropped last runs it.
    let (reader, writer, closer) = bridge_conn.into_parts();
    let closer = Arc::new(BridgeCloser(Mutex::new(Some(closer))));
    Ok((
        BridgeSender {
            framed: FramedWrite::new(writer, framed_codec()),
            _closer: closer.clone(),
        },
        BridgeReceiver {
            framed: FramedRead::new(reader, framed_codec()),
            _closer: closer,
        },
    ))
}

/// `BridgeConn::try_connect` takes an extra `on_socket_open(RawFd)` callback on
/// Linux/Android (e.g. for a VPN's protect-socket hook) that doesn't exist on
/// other targets. Smoldvpn has no such hook today, so this just passes
/// `nym_bridges`'s own no-op (`SOCKET_OPEN_NOP`) to satisfy the signature.
#[cfg(any(target_os = "linux", target_os = "android"))]
async fn dial(
    params: ClientConfig,
    cancel: CancellationToken,
) -> std::result::Result<BridgeConn, TransportError> {
    BridgeConn::try_connect(params, cancel, nym_bridges::connection::SOCKET_OPEN_NOP, None).await
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
async fn dial(
    params: ClientConfig,
    cancel: CancellationToken,
) -> std::result::Result<BridgeConn, TransportError> {
    BridgeConn::try_connect(params, cancel).await
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
    use nym_bridges::transport::quic::{create_endpoint, ClientOptions, ServerConfig};
    use std::net::SocketAddr;

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
        let params = BridgeParams::QuicPlain(ClientOptions {
            addresses: vec![addr],
            host: Some(sni),
            id_pubkey: id_pubkey_base64,
        });
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
        let params = BridgeParams::QuicPlain(ClientOptions {
            addresses: vec![addr],
            host: Some(sni),
            id_pubkey: BASE64_STANDARD.encode(bytes),
        });
        let cancel = CancellationToken::new();
        assert!(
            connect(&params, &cancel).await.is_err(),
            "connection must be rejected with a mismatched pin"
        );
    }
}
