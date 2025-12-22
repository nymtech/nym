//! LP+Sphinx+KCP Client
//!
//! Integrates LP transport with Sphinx routing and KCP framing.
//! Supports bidirectional encrypted data channel testing.

use anyhow::{Context, Result, bail};
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_kcp::driver::KcpDriver;
use nym_kcp::session::KcpSession;
use nym_registration_client::LpRegistrationClient;
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::addressing::nodes::NymNodeRoutingAddress;
use nym_sphinx::message::NymMessage;
use nym_sphinx::params::{PacketSize, PacketType, SphinxKeyRotation};
use nym_sphinx::{Delay, Destination, DestinationAddressBytes, NymPacket};
use nym_sphinx_anonymous_replies::requests::{AnonymousSenderTag, RepliableMessage};
use nym_sphinx_anonymous_replies::{ReplySurb, SurbEncryptionKey};
use nym_sphinx_framing::codec::NymCodec;
use nym_sphinx_framing::packet::FramedNymPacket;
use rand_chacha::ChaCha8Rng;
use rand_chacha::rand_core::SeedableRng;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::UdpSocket;
use tokio_util::bytes::BytesMut;
use tokio_util::codec::Encoder;
use tracing::{debug, info, trace};

use crate::topology::{GatewayInfo, SpeedtestTopology};

/// Conv ID for KCP - hash of source and destination addresses
fn compute_conv_id(local: SocketAddr, remote: SocketAddr) -> u32 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    local.hash(&mut hasher);
    remote.hash(&mut hasher);
    hasher.finish() as u32
}

/// Speedtest client for LP+Sphinx+KCP testing
pub struct SpeedtestClient {
    /// Client's Ed25519 identity keypair
    identity_keypair: Arc<ed25519::KeyPair>,
    /// Client's x25519 encryption keypair (for SURBs)
    encryption_keypair: Arc<x25519::KeyPair>,
    /// Target gateway
    gateway: GatewayInfo,
    /// Network topology for routing
    topology: Arc<SpeedtestTopology>,
    /// UDP socket for data plane
    socket: Option<UdpSocket>,
    /// KCP driver for reliable delivery
    kcp_driver: Option<KcpDriver>,
    /// RNG for packet building
    rng: ChaCha8Rng,
}

impl SpeedtestClient {
    /// Create a new speedtest client
    pub fn new(gateway: GatewayInfo, topology: Arc<SpeedtestTopology>) -> Self {
        let identity_keypair = Arc::new(ed25519::KeyPair::new(&mut rand::rngs::OsRng));
        let encryption_keypair = Arc::new(x25519::KeyPair::new(&mut rand::rngs::OsRng));
        let rng = ChaCha8Rng::from_entropy();

        Self {
            identity_keypair,
            encryption_keypair,
            gateway,
            topology,
            socket: None,
            kcp_driver: None,
            rng,
        }
    }

    /// Get this client's Recipient address for receiving replies
    fn recipient(&self) -> Recipient {
        Recipient::new(
            *self.identity_keypair.public_key(),
            *self.encryption_keypair.public_key(),
            self.gateway.identity,
        )
    }

    /// Test LP control plane connectivity (TCP handshake)
    ///
    /// Returns handshake duration on success.
    pub async fn test_lp_handshake(&self) -> Result<Duration> {
        info!(
            "Testing LP handshake with gateway at {}",
            self.gateway.lp_address
        );

        let client_ip = "0.0.0.0".parse().unwrap();

        let mut lp_client = LpRegistrationClient::new_with_default_psk(
            self.identity_keypair.clone(),
            self.gateway.identity,
            self.gateway.lp_address,
            client_ip,
        );

        let start = Instant::now();
        lp_client
            .perform_handshake()
            .await
            .context("LP handshake failed")?;
        let duration = start.elapsed();

        info!("LP handshake successful in {:?}", duration);
        lp_client.close();

        Ok(duration)
    }

    /// Initialize UDP socket and KCP for data plane
    pub async fn init_data_channel(&mut self) -> Result<()> {
        let socket = UdpSocket::bind("0.0.0.0:0")
            .await
            .context("failed to bind UDP socket")?;

        let local_addr = socket.local_addr()?;
        let conv_id = compute_conv_id(local_addr, self.gateway.mix_host);

        debug!(
            "UDP socket bound to {}, conv_id={}",
            local_addr, conv_id
        );

        let session = KcpSession::new(conv_id);
        let driver = KcpDriver::new(session);

        self.socket = Some(socket);
        self.kcp_driver = Some(driver);

        Ok(())
    }

    /// Send data via KCP, wrap in Sphinx packet, and send to first hop
    ///
    /// This is a convenience wrapper that calls `send_data_with_surbs` with no SURBs.
    pub async fn send_data(&mut self, payload: &[u8]) -> Result<()> {
        self.send_data_with_surbs(payload, 0).await?;
        Ok(())
    }

    /// Send data with SURBs for bidirectional communication
    ///
    /// Returns the SURB encryption keys needed to decrypt replies.
    /// The `num_surbs` parameter controls how many reply SURBs to attach.
    pub async fn send_data_with_surbs(
        &mut self,
        payload: &[u8],
        num_surbs: usize,
    ) -> Result<Vec<SurbEncryptionKey>> {
        if self.socket.is_none() {
            self.init_data_channel().await?;
        }

        let driver = self.kcp_driver.as_mut().context("KCP not initialized")?;
        let socket = self.socket.as_ref().context("socket not initialized")?;

        // Step 1: Feed payload to KCP for reliable delivery
        driver.send(payload);
        driver.update(10); // Process KCP state machine to produce outgoing packets

        let outgoing = driver.fetch_outgoing();
        if outgoing.is_empty() {
            bail!("KCP produced no outgoing packets");
        }

        // Step 2: Encode KCP packets
        let mut kcp_buf = BytesMut::new();
        for pkt in outgoing {
            pkt.encode(&mut kcp_buf);
        }
        debug!("KCP produced {} bytes", kcp_buf.len());

        // Step 3: Build route and destination
        let route = self
            .topology
            .random_route_to_gateway(&mut self.rng, &self.gateway)?;

        if route.is_empty() {
            bail!("empty route");
        }

        let destination = Destination::new(
            DestinationAddressBytes::from_bytes(*self.gateway.sphinx_key.as_bytes()),
            Default::default(),
        );

        let delays: Vec<Delay> = route.iter().map(|_| Delay::new_from_millis(0)).collect();

        // Step 4: Create SURBs for replies (if requested)
        let mut surbs_with_keys = Vec::with_capacity(num_surbs);
        if num_surbs > 0 {
            let recipient = self.recipient();
            let route_provider = self.topology.route_provider();

            for _ in 0..num_surbs {
                let surb = ReplySurb::construct(
                    &mut self.rng,
                    &recipient,
                    Duration::from_millis(0), // zero delay for speed testing
                    false,                    // use_legacy_surb_format
                    &route_provider,
                    false, // disable_mix_hops
                )
                .context("failed to construct reply SURB")?;
                surbs_with_keys.push(surb.with_key_rotation(SphinxKeyRotation::Unknown));
            }
        }

        // Extract encryption keys for later decryption
        let encryption_keys: Vec<SurbEncryptionKey> = surbs_with_keys
            .iter()
            .map(|s| s.encryption_key().clone())
            .collect();

        // Step 5: Build message (RepliableMessage if SURBs, plain otherwise)
        let nym_message = if num_surbs > 0 {
            let sender_tag = AnonymousSenderTag::new_random(&mut self.rng);
            let repliable_message = RepliableMessage::new_data(
                false, // use_legacy_surb_format
                kcp_buf.to_vec(),
                sender_tag,
                surbs_with_keys,
            );
            NymMessage::new_repliable(repliable_message)
        } else {
            NymMessage::new_plain(kcp_buf.to_vec())
        };

        let nym_message =
            nym_message.pad_to_full_packet_lengths(PacketSize::RegularPacket.plaintext_size());

        // Step 6: Fragment and send
        let fragments = nym_message
            .split_into_fragments(&mut self.rng, PacketSize::RegularPacket.plaintext_size());

        debug!(
            "Message with {} SURBs split into {} fragments",
            num_surbs,
            fragments.len()
        );

        let mut packet_buf = BytesMut::new();
        for fragment in fragments {
            let nym_packet = NymPacket::sphinx_build(
                false, // use_legacy_sphinx_format
                PacketSize::RegularPacket.payload_size(),
                fragment.into_bytes(),
                &route,
                &destination,
                &delays,
            )?;

            let framed = FramedNymPacket::new(
                nym_packet,
                PacketType::Mix,
                SphinxKeyRotation::Unknown,
                false, // use_legacy_packet_encoding
            );
            let mut codec = NymCodec;
            codec.encode(framed, &mut packet_buf)?;
        }

        // Send to first hop
        let first_hop_addr: SocketAddr =
            NymNodeRoutingAddress::try_from(route[0].address)?.into();

        socket.send_to(&packet_buf, first_hop_addr).await?;
        info!(
            "Sent {} bytes (KCP) with {} SURBs ({} packet bytes) to {}",
            kcp_buf.len(),
            num_surbs,
            packet_buf.len(),
            first_hop_addr
        );

        Ok(encryption_keys)
    }

    /// Receive UDP data with timeout
    pub async fn recv_data(&self, timeout: Duration) -> Result<Option<Vec<u8>>> {
        let socket = self.socket.as_ref().context("socket not initialized")?;
        let mut buf = vec![0u8; 65536];

        match tokio::time::timeout(timeout, socket.recv_from(&mut buf)).await {
            Ok(Ok((len, from))) => {
                trace!("Received {} bytes from {}", len, from);
                Ok(Some(buf[..len].to_vec()))
            }
            Ok(Err(e)) => Err(e.into()),
            Err(_) => Ok(None),
        }
    }

    /// Get gateway info
    pub fn gateway(&self) -> &GatewayInfo {
        &self.gateway
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nym_sphinx_types::{
        Delay as SphinxDelay, Destination, DestinationAddressBytes, Node, NodeAddressBytes,
        PrivateKey, DESTINATION_ADDRESS_LENGTH, IDENTIFIER_LENGTH, NODE_ADDRESS_LENGTH,
    };

    #[test]
    fn test_conv_id() {
        let local: SocketAddr = "127.0.0.1:12345".parse().unwrap();
        let remote: SocketAddr = "192.168.1.1:80".parse().unwrap();

        let id1 = compute_conv_id(local, remote);
        let id2 = compute_conv_id(local, remote);

        assert_eq!(id1, id2);
    }

    fn random_pubkey() -> nym_sphinx_types::PublicKey {
        let private_key = PrivateKey::random();
        (&private_key).into()
    }

    #[test]
    fn test_sphinx_packet_building() {
        // Build a simple 3-hop route
        let node1 = Node::new(
            NodeAddressBytes::from_bytes([5u8; NODE_ADDRESS_LENGTH]),
            random_pubkey(),
        );
        let node2 = Node::new(
            NodeAddressBytes::from_bytes([4u8; NODE_ADDRESS_LENGTH]),
            random_pubkey(),
        );
        let node3 = Node::new(
            NodeAddressBytes::from_bytes([2u8; NODE_ADDRESS_LENGTH]),
            random_pubkey(),
        );

        let route = [node1, node2, node3];
        let destination = Destination::new(
            DestinationAddressBytes::from_bytes([3u8; DESTINATION_ADDRESS_LENGTH]),
            [4u8; IDENTIFIER_LENGTH],
        );
        let delays = vec![
            SphinxDelay::new_from_millis(0),
            SphinxDelay::new_from_millis(0),
            SphinxDelay::new_from_millis(0),
        ];

        let payload = b"test message for sphinx packet";

        // Build the packet using the same API as send_data
        let result = NymPacket::sphinx_build(
            false, // use_legacy_sphinx_format
            PacketSize::RegularPacket.payload_size(),
            payload,
            &route,
            &destination,
            &delays,
        );

        assert!(result.is_ok(), "sphinx_build failed: {:?}", result.err());
        let packet = result.unwrap();
        assert!(packet.len() > 0, "packet should not be empty");

        // Verify we can frame it
        let framed = FramedNymPacket::new(
            packet,
            PacketType::Mix,
            SphinxKeyRotation::Unknown,
            false,
        );

        let mut buf = BytesMut::new();
        let mut codec = NymCodec;
        let encode_result = codec.encode(framed, &mut buf);
        assert!(
            encode_result.is_ok(),
            "framing failed: {:?}",
            encode_result.err()
        );
        assert!(buf.len() > 0, "encoded buffer should not be empty");
    }
}
