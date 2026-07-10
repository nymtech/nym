// Copyright 2024-2026 - Nym Technologies SA <contact@nymtech.net>

//! Inline QUIC bridge client (design D6), an alternative WireGuard data-plane
//! transport for clients blocked from pure UDP. Reimplemented here (does NOT
//! depend on any `nym_bridges` crate) to byte-match the `nym-vpn-client`
//! reference (`.../tunnel/transports/{mod.rs,certs.rs}`):
//!
//! - ALPN `hq-29`, keep-alive + max idle timeout + BBR for the long-lived session.
//! - [`IdentityBasedVerifier`]: ed25519-only, SNI ∈ cert alt-names, cert SPKI ==
//!   pinned `id_pubkey`.
//! - One reliable `open_bi()` stream, each WireGuard packet prefixed by a 2-byte
//!   big-endian length.
//!
//! Only ever fronts the two-hop entry leg (the bridge is bound 1:1 to a gateway
//! and forwards to its WireGuard port); there is no QUIC one-hop mode and no
//! gateway-selection handshake. Proven in conformance spike B.

use std::net::SocketAddr;
use std::sync::{Arc, Once};
use std::time::Duration;

use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use quinn::crypto::rustls::QuicClientConfig;
use quinn::{Connection, Endpoint, RecvStream, SendStream};
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{DigitallySignedStruct, SignatureScheme};
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};
use tokio_util::sync::CancellationToken;

use crate::error::{DvpnError, Result};

/// ALPN protocol identifier used by the bridge (reference `ALPN_QUIC_HTTP`).
const ALPN_QUIC_HTTP: &[u8] = b"hq-29";
/// Per-WireGuard-packet length prefix width, big-endian.
const LENGTH_DELIMITER_BYTELEN: usize = 2;
const KEEPALIVE: Duration = Duration::from_secs(20);
const IDLE_TIMEOUT_MS: u32 = 60_000;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

static INSTALL_PROVIDER: Once = Once::new();

/// Bridge connection parameters, sourced from the gateway directory / VPN API.
#[derive(Clone, Debug)]
pub struct BridgeParams {
    /// Candidate bridge socket addresses (the first IPv4 is used).
    pub addresses: Vec<SocketAddr>,
    /// SNI host to present (falls back to the bridge IP string if `None`).
    pub sni_host: Option<String>,
    /// Pinned ed25519 identity public key (decode from the directory's base64).
    pub id_pubkey: [u8; 32],
}

impl BridgeParams {
    /// Decode the base64 `id_pubkey` field from the gateway directory into the
    /// pinned 32-byte ed25519 key.
    pub fn id_pubkey_from_base64(b64: &str) -> Result<[u8; 32]> {
        let bytes = base64_decode(b64)
            .ok_or_else(|| DvpnError::Config("invalid base64 id_pubkey".into()))?;
        let arr: [u8; 32] = bytes
            .try_into()
            .map_err(|_| DvpnError::Config("id_pubkey is not 32 bytes".into()))?;
        Ok(arr)
    }

    fn ipv4_address(&self) -> Result<SocketAddr> {
        self.addresses
            .iter()
            .find(|a| a.is_ipv4())
            .copied()
            .ok_or_else(|| DvpnError::Config("bridge has no IPv4 address".into()))
    }
}

/// ed25519-SPKI cert-pinning verifier, mirroring the reference `certs.rs`.
/// Unlike the reference, an SPKI parse failure is a hard reject (finding #2).
#[derive(Debug)]
struct IdentityBasedVerifier {
    pinned_id_pubkey: [u8; 32],
    alt_names: Vec<String>,
}

fn cert_spki_ed25519(cert_der: &[u8]) -> std::result::Result<[u8; 32], String> {
    use ed25519_dalek::pkcs8::DecodePublicKey;
    let (_, cert) = x509_parser::parse_x509_certificate(cert_der).map_err(|e| e.to_string())?;
    let spki_der = cert.tbs_certificate.subject_pki.raw;
    let pk =
        ed25519_dalek::VerifyingKey::from_public_key_der(spki_der).map_err(|e| e.to_string())?;
    Ok(pk.to_bytes())
}

impl ServerCertVerifier for IdentityBasedVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> std::result::Result<ServerCertVerified, rustls::Error> {
        let sni = match server_name {
            ServerName::DnsName(d) => d.as_ref().to_string(),
            ServerName::IpAddress(ip) => format!("{ip:?}"),
            other => format!("{other:?}"),
        };
        if !self.alt_names.iter().any(|n| n == &sni) {
            return Err(rustls::Error::General(format!(
                "SNI {sni} not in alt-names"
            )));
        }
        let spki = cert_spki_ed25519(end_entity.as_ref())
            .map_err(|e| rustls::Error::General(format!("SPKI parse failed: {e}")))?;
        if spki != self.pinned_id_pubkey {
            return Err(rustls::Error::General(
                "SPKI does not match pinned id_pubkey".into(),
            ));
        }
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            cert,
            dss,
            &rustls::crypto::ring::default_provider().signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &rustls::crypto::ring::default_provider().signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![SignatureScheme::ED25519]
    }
}

fn client_config(params: &BridgeParams, sni: &str) -> Result<quinn::ClientConfig> {
    INSTALL_PROVIDER.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });

    let verifier = IdentityBasedVerifier {
        pinned_id_pubkey: params.id_pubkey,
        alt_names: vec![sni.to_string()],
    };
    let mut crypto = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(verifier))
        .with_no_client_auth();
    crypto.alpn_protocols = vec![ALPN_QUIC_HTTP.to_vec()];

    let quic_crypto = QuicClientConfig::try_from(crypto)
        .map_err(|e| DvpnError::Bridge(format!("quic crypto config: {e}")))?;
    let mut cfg = quinn::ClientConfig::new(Arc::new(quic_crypto));

    let mut transport = quinn::TransportConfig::default();
    transport.keep_alive_interval(Some(KEEPALIVE));
    transport.max_idle_timeout(Some(quinn::VarInt::from_u32(IDLE_TIMEOUT_MS).into()));
    transport.congestion_controller_factory(Arc::new(quinn::congestion::BbrConfig::default()));
    cfg.transport_config(Arc::new(transport));
    Ok(cfg)
}

/// Sending half of the QUIC bridge transport. Holds the connection + endpoint
/// so the QUIC session stays alive for the lifetime of the datapath.
pub(crate) struct QuicBridgeSender {
    framed: FramedWrite<SendStream, LengthDelimitedCodec>,
    _conn: Connection,
    _endpoint: Endpoint,
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

/// Connect to the bridge and open the single WireGuard-carrying bi-stream.
/// Cancellable via `cancel`.
pub(crate) async fn connect(
    params: &BridgeParams,
    cancel: &CancellationToken,
) -> Result<(QuicBridgeSender, QuicBridgeReceiver)> {
    let addr = params.ipv4_address()?;
    let sni = params
        .sni_host
        .clone()
        .unwrap_or_else(|| addr.ip().to_string());

    let cfg = client_config(params, &sni)?;
    let mut endpoint = quinn::Endpoint::client("0.0.0.0:0".parse().expect("valid bind addr"))
        .map_err(|e| DvpnError::Bridge(format!("endpoint bind: {e}")))?;
    endpoint.set_default_client_config(cfg);

    let connecting = endpoint
        .connect(addr, &sni)
        .map_err(|e| DvpnError::Bridge(format!("connect: {e}")))?;

    let conn = tokio::select! {
        _ = cancel.cancelled() => return Err(DvpnError::Cancelled),
        r = tokio::time::timeout(CONNECT_TIMEOUT, connecting) => {
            r.map_err(|_| DvpnError::Bridge("connect timed out".into()))?
             .map_err(|e| DvpnError::Bridge(format!("connect: {e}")))?
        }
    };

    let (send, recv) = conn
        .open_bi()
        .await
        .map_err(|e| DvpnError::Bridge(format!("open_bi: {e}")))?;

    // quinn's SendStream/RecvStream implement tokio AsyncWrite/AsyncRead, so they
    // frame directly. Keep the connection + endpoint alive by parking them in the
    // halves (both live as long as the datapath task).
    Ok((
        QuicBridgeSender {
            framed: FramedWrite::new(send, framed_codec()),
            _conn: conn.clone(),
            _endpoint: endpoint,
        },
        QuicBridgeReceiver {
            framed: FramedRead::new(recv, framed_codec()),
            _conn: conn,
        },
    ))
}

// --- Minimal standard-base64 decoder (avoids an extra workspace dep for the
// --- single id_pubkey field). Accepts standard alphabet with optional padding.
fn base64_decode(input: &str) -> Option<Vec<u8>> {
    fn val(c: u8) -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    let cleaned: Vec<u8> = input
        .bytes()
        .filter(|&b| b != b'=' && !b.is_ascii_whitespace())
        .collect();
    let mut out = Vec::with_capacity(cleaned.len() * 3 / 4);
    for chunk in cleaned.chunks(4) {
        let mut buf = [0u8; 4];
        let mut n = 0;
        for (i, &c) in chunk.iter().enumerate() {
            buf[i] = val(c)?;
            n += 1;
        }
        if n >= 2 {
            out.push((buf[0] << 2) | (buf[1] >> 4));
        }
        if n >= 3 {
            out.push((buf[1] << 4) | (buf[2] >> 2));
        }
        if n == 4 {
            out.push((buf[2] << 6) | buf[3]);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    //! Conformance test (OpenSpec task 6.6): the real inline bridge client
    //! against a local mock bridge with an ed25519 self-signed cert. Proves
    //! ALPN `hq-29`, 2-byte-BE framing over one bi-stream, and ed25519-SPKI
    //! pinning (positive + negative), all through the crate's own `connect`.

    use super::*;
    use futures::{SinkExt, StreamExt};
    use quinn::crypto::rustls::QuicServerConfig;
    use rustls::pki_types::PrivatePkcs8KeyDer;

    const SNI: &str = "bridge.example";

    fn server_identity() -> (
        rustls::pki_types::CertificateDer<'static>,
        PrivatePkcs8KeyDer<'static>,
        [u8; 32],
    ) {
        let kp = rcgen::KeyPair::generate_for(&rcgen::PKCS_ED25519).unwrap();
        let cert = rcgen::CertificateParams::new(vec![SNI.to_string()])
            .unwrap()
            .self_signed(&kp)
            .unwrap();
        let cert_der = cert.der().clone();
        let key_der = PrivatePkcs8KeyDer::from(kp.serialize_der());
        let pin = cert_spki_ed25519(cert_der.as_ref()).unwrap();
        (cert_der, key_der, pin)
    }

    fn spawn_mock_bridge() -> (std::net::SocketAddr, [u8; 32]) {
        INSTALL_PROVIDER.call_once(|| {
            let _ = rustls::crypto::ring::default_provider().install_default();
        });
        let (cert, key, pin) = server_identity();
        let mut crypto = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![cert], key.into())
            .unwrap();
        crypto.alpn_protocols = vec![ALPN_QUIC_HTTP.to_vec()];
        let scfg =
            quinn::ServerConfig::with_crypto(Arc::new(QuicServerConfig::try_from(crypto).unwrap()));
        let endpoint = quinn::Endpoint::server(scfg, "127.0.0.1:0".parse().unwrap()).unwrap();
        let addr = endpoint.local_addr().unwrap();

        tokio::spawn(async move {
            let _endpoint = endpoint.clone();
            if let Some(incoming) = endpoint.accept().await {
                if let Ok(conn) = incoming.await {
                    if let Ok((send, recv)) = conn.accept_bi().await {
                        let mut w = FramedWrite::new(send, framed_codec());
                        let mut r = FramedRead::new(recv, framed_codec());
                        while let Some(Ok(frame)) = r.next().await {
                            if w.send(bytes::Bytes::from(frame.to_vec())).await.is_err() {
                                break;
                            }
                        }
                    }
                }
            }
        });
        (addr, pin)
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn bridge_framing_and_pinning_roundtrip() {
        let (addr, pin) = spawn_mock_bridge();
        let params = BridgeParams {
            addresses: vec![addr],
            sni_host: Some(SNI.to_string()),
            id_pubkey: pin,
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
        let (addr, mut pin) = spawn_mock_bridge();
        pin[0] ^= 0xFF; // corrupt the pinned key
        let params = BridgeParams {
            addresses: vec![addr],
            sni_host: Some(SNI.to_string()),
            id_pubkey: pin,
        };
        let cancel = CancellationToken::new();
        assert!(
            connect(&params, &cancel).await.is_err(),
            "connection must be rejected with a mismatched pin"
        );
    }

    #[test]
    fn id_pubkey_base64_roundtrip() {
        let raw = [7u8; 32];
        // standard base64 of 32 bytes of 0x07
        let b64 = "BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc=";
        assert_eq!(BridgeParams::id_pubkey_from_base64(b64).unwrap(), raw);
    }
}
