//! Spike B — QUIC bridge one-packet round-trip (OpenSpec task 1.2).
//!
//! Mirrors the three protocol invariants of the `nym-vpn-client` inline bridge
//! client (`.../tunnel/transports/mod.rs` + `certs.rs`) with a local `quinn`
//! server standing in for the bridge — no external bridge server needed:
//!   1. ALPN `hq-29`.
//!   2. `IdentityBasedVerifier`: ed25519-only, SNI ∈ cert alt-names, and cert
//!      SPKI == pinned `id_pubkey`.
//!   3. One reliable `open_bi()` stream, each WireGuard packet prefixed by a
//!      2-byte big-endian length (tokio_util `LengthDelimitedCodec`).
//! Plus a negative test: pinning the wrong key must reject the connection.
//!
//! Run: `cargo run --bin spike_b_quic` (exit code 0 = PASS).

use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use quinn::crypto::rustls::{QuicClientConfig, QuicServerConfig};
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, PrivatePkcs8KeyDer, ServerName, UnixTime};
use rustls::{DigitallySignedStruct, SignatureScheme};
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};

const ALPN_QUIC_HTTP: &[u8] = b"hq-29";
const LENGTH_DELIMITER_BYTELEN: usize = 2;
const SNI_HOST: &str = "bridge.example";
const KEEPALIVE: Duration = Duration::from_secs(20);
const IDLE_TIMEOUT_MS: u32 = 60_000;

/// ed25519-SPKI cert-pinning verifier, mirroring the reference `certs.rs`.
#[derive(Debug)]
struct IdentityBasedVerifier {
    pinned_id_pubkey: [u8; 32],
    alt_names: Vec<String>,
}

impl IdentityBasedVerifier {
    fn new(pinned_id_pubkey: [u8; 32], alt_names: Vec<String>) -> Self {
        Self { pinned_id_pubkey, alt_names }
    }
}

/// Extract the raw ed25519 public key from a certificate's SubjectPublicKeyInfo,
/// exactly as the reference does (parse SPKI DER via `ed25519_dalek::pkcs8`).
fn cert_spki_ed25519(cert_der: &[u8]) -> Result<[u8; 32], Box<dyn Error + Send + Sync>> {
    use ed25519_dalek::pkcs8::DecodePublicKey;
    let (_, cert) = x509_parser::parse_x509_certificate(cert_der)?;
    let spki_der = cert.tbs_certificate.subject_pki.raw;
    let pk = ed25519_dalek::VerifyingKey::from_public_key_der(spki_der)?;
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
    ) -> Result<ServerCertVerified, rustls::Error> {
        // 1. SNI must be an accepted alt-name.
        let sni = match server_name {
            ServerName::DnsName(d) => d.as_ref().to_string(),
            other => format!("{other:?}"),
        };
        if !self.alt_names.iter().any(|n| n == &sni) {
            return Err(rustls::Error::General(format!("SNI {sni} not in alt-names")));
        }

        // 2. Cert SPKI must equal the pinned ed25519 identity key.
        let spki = cert_spki_ed25519(end_entity.as_ref())
            .map_err(|e| rustls::Error::General(format!("SPKI parse failed: {e}")))?;
        if spki != self.pinned_id_pubkey {
            return Err(rustls::Error::General("SPKI does not match pinned id_pubkey".into()));
        }
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
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
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
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

/// A self-signed ed25519 server cert + its pinned public key.
struct ServerIdentity {
    cert_der: CertificateDer<'static>,
    key_der: PrivatePkcs8KeyDer<'static>,
    id_pubkey: [u8; 32],
}

fn make_server_identity() -> Result<ServerIdentity, Box<dyn Error + Send + Sync>> {
    let key_pair = rcgen::KeyPair::generate_for(&rcgen::PKCS_ED25519)?;
    let cert = rcgen::CertificateParams::new(vec![SNI_HOST.to_string()])?.self_signed(&key_pair)?;
    let cert_der = cert.der().clone();
    let key_der = PrivatePkcs8KeyDer::from(key_pair.serialize_der());
    // Pin exactly what the verifier will read back out of the cert.
    let id_pubkey = cert_spki_ed25519(cert_der.as_ref())?;
    Ok(ServerIdentity { cert_der, key_der, id_pubkey })
}

fn server_config(id: &ServerIdentity) -> Result<quinn::ServerConfig, Box<dyn Error + Send + Sync>> {
    let mut crypto = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![id.cert_der.clone()], id.key_der.clone_key().into())?;
    crypto.alpn_protocols = vec![ALPN_QUIC_HTTP.to_vec()];
    let server_cfg = quinn::ServerConfig::with_crypto(Arc::new(QuicServerConfig::try_from(crypto)?));
    Ok(server_cfg)
}

fn client_config(
    pinned: [u8; 32],
) -> Result<quinn::ClientConfig, Box<dyn Error + Send + Sync>> {
    let verifier = IdentityBasedVerifier::new(pinned, vec![SNI_HOST.to_string()]);
    let mut crypto = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(verifier))
        .with_no_client_auth();
    crypto.alpn_protocols = vec![ALPN_QUIC_HTTP.to_vec()];

    let mut client_cfg = quinn::ClientConfig::new(Arc::new(QuicClientConfig::try_from(crypto)?));
    let mut transport = quinn::TransportConfig::default();
    transport.keep_alive_interval(Some(KEEPALIVE));
    transport.max_idle_timeout(Some(quinn::VarInt::from_u32(IDLE_TIMEOUT_MS).into()));
    transport.congestion_controller_factory(Arc::new(quinn::congestion::BbrConfig::default()));
    client_cfg.transport_config(Arc::new(transport));
    Ok(client_cfg)
}

/// Bridge echo server: accepts one connection, one bi-stream, and echoes each
/// length-framed WireGuard packet straight back.
async fn run_bridge(endpoint: quinn::Endpoint) -> Result<(), Box<dyn Error + Send + Sync>> {
    let incoming = endpoint.accept().await.ok_or("no incoming connection")?;
    let conn = incoming.await?;
    let (send, recv) = conn.accept_bi().await?;
    let mut writer = FramedWrite::new(
        send,
        LengthDelimitedCodec::builder()
            .length_field_length(LENGTH_DELIMITER_BYTELEN)
            .new_codec(),
    );
    let mut reader = FramedRead::new(
        recv,
        LengthDelimitedCodec::builder()
            .length_field_length(LENGTH_DELIMITER_BYTELEN)
            .new_codec(),
    );
    while let Some(frame) = reader.next().await {
        let frame = frame?;
        writer.send(Bytes::from(frame.to_vec())).await?;
    }
    Ok(())
}

/// Client half: connect with the pinning verifier, open one bi-stream, send one
/// length-framed WG packet, read the framed reply.
async fn bridge_roundtrip(
    server_addr: SocketAddr,
    pinned: [u8; 32],
    packet: &[u8],
) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
    let mut endpoint = quinn::Endpoint::client("0.0.0.0:0".parse()?)?;
    endpoint.set_default_client_config(client_config(pinned)?);

    let conn = endpoint.connect(server_addr, SNI_HOST)?.await?;
    let (send, recv) = conn.open_bi().await?;
    let mut writer = FramedWrite::new(
        send,
        LengthDelimitedCodec::builder()
            .length_field_length(LENGTH_DELIMITER_BYTELEN)
            .new_codec(),
    );
    let mut reader = FramedRead::new(
        recv,
        LengthDelimitedCodec::builder()
            .length_field_length(LENGTH_DELIMITER_BYTELEN)
            .new_codec(),
    );

    // No gateway-selection handshake: first bytes on the wire are the WG packet.
    writer.send(Bytes::copy_from_slice(packet)).await?;
    let reply = reader.next().await.ok_or("no framed reply")??;
    Ok(reply.to_vec())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .ok();

    println!("== Spike B: QUIC bridge one-packet round-trip ==");
    let id = make_server_identity()?;
    let good_pin = id.id_pubkey;

    let server_ep = quinn::Endpoint::server(server_config(&id)?, "127.0.0.1:0".parse()?)?;
    let server_addr = server_ep.local_addr()?;
    println!("  bridge listening on {server_addr}, ALPN=hq-29, ed25519 pinned");

    let server_task = tokio::spawn(run_bridge(server_ep));

    // Positive: correct pin, 2-byte-framed WG-like packet echoes back verbatim.
    let wg_packet: Vec<u8> = (0u16..1200).map(|i| (i % 251) as u8).collect();
    let reply = bridge_roundtrip(server_addr, good_pin, &wg_packet).await?;
    assert_eq!(reply, wg_packet, "framed round-trip mismatch");
    println!("  PASS: {} B WG packet round-tripped over one bi-stream", wg_packet.len());
    server_task.abort();

    // Negative: wrong pin must be rejected during the TLS handshake.
    let id2 = make_server_identity()?;
    let server_ep2 = quinn::Endpoint::server(server_config(&id2)?, "127.0.0.1:0".parse()?)?;
    let addr2 = server_ep2.local_addr()?;
    let server_task2 = tokio::spawn(run_bridge(server_ep2));

    let mut wrong_pin = id2.id_pubkey;
    wrong_pin[0] ^= 0xFF; // deliberately corrupt the pinned key
    match bridge_roundtrip(addr2, wrong_pin, &wg_packet).await {
        Err(e) => println!("  PASS: wrong pin rejected ({e})"),
        Ok(_) => return Err("SECURITY: connection succeeded with wrong pin".into()),
    }
    server_task2.abort();

    println!("== Spike B PASS ==");
    Ok(())
}
