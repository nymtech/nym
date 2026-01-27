//! LP+Sphinx+KCP Client
//!
//! Integrates LP transport with Sphinx routing and KCP framing.
//! Supports bidirectional encrypted data channel testing.

#![allow(unused)]

use anyhow::{bail, Context, Result};
use bytes::Bytes;
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
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::{TcpStream, UdpSocket};
use tokio_util::bytes::BytesMut;
use tokio_util::codec::Encoder;
use tracing::{debug, info, trace};

use crate::topology::{GatewayInfo, SpeedtestTopology};
use nym_ip_packet_requests::v8::request::IpPacketRequest;
use nym_sphinx::forwarding::packet::MixPacket;

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
    /// LP registration client (kept alive for data framing)
    lp_client: Option<LpRegistrationClient>,
}

/// Prepared Sphinx packet data ready for sending
struct PreparedPackets {
    /// Message fragments ready for Sphinx wrapping
    fragments: Vec<nym_sphinx::chunking::fragment::Fragment>,
    /// Route through mixnet (Mix1, Mix2, Mix3, Gateway)
    route: Vec<nym_sphinx_types::Node>,
    /// Sphinx destination (gateway's sphinx key)
    destination: Destination,
    /// Zero delays for each hop
    delays: Vec<Delay>,
    /// SURB encryption keys for decrypting replies
    encryption_keys: Vec<SurbEncryptionKey>,
    /// First hop address for sending
    first_hop_addr: NymNodeRoutingAddress,
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
            lp_client: None,
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

        let client_ip = "0.0.0.0".parse()?;

        let mut lp_client = LpRegistrationClient::<TcpStream>::new_with_default_psk(
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

    /// Initialize LP session for data plane communication.
    ///
    /// Performs LP handshake with the gateway and stores the cryptographic session
    /// state (encryption keys, counters, session ID) for wrapping UDP data packets.
    /// The TCP control connection is closed immediately after handshake; only the
    /// state machine is retained for `wrap_data()` calls.
    ///
    /// # Data Flow
    /// After calling this method, use `send_data_via_lp()` to:
    /// 1. Build Sphinx packets (as usual)
    /// 2. Wrap them in LP via the stored state machine
    /// 3. Send to gateway's LP data port (UDP:51264)
    ///
    /// # Returns
    /// Handshake duration on success.
    pub async fn init_lp_session(&mut self) -> Result<Duration> {
        info!(
            "Initializing LP session with gateway at {}",
            self.gateway.lp_address
        );

        let client_ip = "0.0.0.0".parse()?;

        let mut lp_client = LpRegistrationClient::<TcpStream>::new_with_default_psk(
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

        // Close TCP connection - we only need the state machine for UDP data
        // (close() only drops stream, preserves state_machine)
        lp_client.close();

        info!(
            "LP session established in {:?}, session_id={}",
            duration,
            lp_client.session_id().unwrap_or(0)
        );

        // Store the client (with state machine) for data plane operations
        self.lp_client = Some(lp_client);

        Ok(duration)
    }

    /// Check if LP session is established
    pub fn has_lp_session(&self) -> bool {
        self.lp_client
            .as_ref()
            .map(|c| c.is_handshake_complete())
            .unwrap_or(false)
    }

    /// Close LP session and cleanup resources.
    ///
    /// This fully destroys the LP session (state machine + TCP stream), unlike
    /// `LpRegistrationClient::close()` which only drops the TCP stream.
    /// After calling this, `init_lp_session()` must be called again to re-establish.
    pub fn close_lp_session(&mut self) {
        if let Some(mut client) = self.lp_client.take() {
            client.close();
            info!("LP session closed");
        }
    }

    /// Initialize UDP socket and KCP for data plane
    pub async fn init_data_channel(&mut self) -> Result<()> {
        let socket = UdpSocket::bind("0.0.0.0:0")
            .await
            .context("failed to bind UDP socket")?;

        let local_addr = socket.local_addr()?;
        let conv_id = compute_conv_id(local_addr, self.gateway.mix_host);

        debug!("UDP socket bound to {local_addr}, conv_id={conv_id}");

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

    /// Prepare Sphinx packet fragments for sending.
    ///
    /// Common logic for both direct and LP-wrapped sending:
    /// - Wraps payload in IpPacketRequest + KCP
    /// - Builds route and destination
    /// - Creates SURBs if requested
    /// - Fragments the message
    ///
    /// Returns prepared data ready for Sphinx wrapping and sending.
    async fn prepare_sphinx_fragments(
        &mut self,
        payload: &[u8],
        num_surbs: usize,
    ) -> Result<PreparedPackets> {
        if self.socket.is_none() {
            self.init_data_channel().await?;
        }

        let driver = self.kcp_driver.as_mut().context("KCP not initialized")?;

        // Step 1: Wrap payload in IpPacketRequest (DataRequest) and feed to KCP
        let data_request = IpPacketRequest::new_data_request(Bytes::copy_from_slice(payload));
        let data_bytes = data_request
            .to_bytes()
            .context("failed to serialize IpPacketRequest")?;
        driver.send(&data_bytes);
        driver.update(10);

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
                    Duration::from_millis(0),
                    false,
                    &route_provider,
                    false,
                )
                .context("failed to construct reply SURB")?;
                surbs_with_keys.push(surb.with_key_rotation(SphinxKeyRotation::Unknown));
            }
        }

        let encryption_keys: Vec<SurbEncryptionKey> = surbs_with_keys
            .iter()
            .map(|s| *s.encryption_key())
            .collect();

        // Step 5: Build message (RepliableMessage if SURBs, plain otherwise)
        let nym_message = if num_surbs > 0 {
            let sender_tag = AnonymousSenderTag::new_random(&mut self.rng);
            let repliable_message =
                RepliableMessage::new_data(false, kcp_buf.to_vec(), sender_tag, surbs_with_keys);
            NymMessage::new_repliable(repliable_message)
        } else {
            NymMessage::new_plain(kcp_buf.to_vec())
        };

        let nym_message =
            nym_message.pad_to_full_packet_lengths(PacketSize::RegularPacket.plaintext_size());

        // Step 6: Fragment
        let fragments = nym_message
            .split_into_fragments(&mut self.rng, PacketSize::RegularPacket.plaintext_size());

        debug!(
            "Message with {} SURBs split into {} fragments",
            num_surbs,
            fragments.len()
        );

        let first_hop_addr = NymNodeRoutingAddress::try_from(route[0].address)
            .context("invalid first hop address")?;

        Ok(PreparedPackets {
            fragments,
            route,
            destination,
            delays,
            encryption_keys,
            first_hop_addr,
        })
    }

    /// Send data with SURBs for bidirectional communication (direct to Mix1).
    ///
    /// Returns the SURB encryption keys needed to decrypt replies.
    /// The `num_surbs` parameter controls how many reply SURBs to attach.
    ///
    /// Note: This sends directly to the first mix node. For LP transport,
    /// use `send_data_via_lp()` instead.
    pub async fn send_data_with_surbs(
        &mut self,
        payload: &[u8],
        num_surbs: usize,
    ) -> Result<Vec<SurbEncryptionKey>> {
        let prepared = self.prepare_sphinx_fragments(payload, num_surbs).await?;
        let socket = self.socket.as_ref().context("socket not initialized")?;

        let mut packet_buf = BytesMut::new();
        for fragment in prepared.fragments {
            let nym_packet = NymPacket::sphinx_build(
                false,
                PacketSize::RegularPacket.payload_size(),
                fragment.into_bytes(),
                &prepared.route,
                &prepared.destination,
                &prepared.delays,
            )?;

            let framed = FramedNymPacket::new(
                nym_packet,
                PacketType::Mix,
                SphinxKeyRotation::Unknown,
                false,
            );
            let mut codec = NymCodec;
            codec.encode(framed, &mut packet_buf)?;
        }

        let first_hop_socket: SocketAddr = prepared.first_hop_addr.into();
        socket.send_to(&packet_buf, first_hop_socket).await?;
        info!(
            "Sent {} packet bytes with {} SURBs to {}",
            packet_buf.len(),
            num_surbs,
            first_hop_socket
        );

        Ok(prepared.encryption_keys)
    }

    /// Send data via LP data plane (UDP:51264) with SURBs for bidirectional communication.
    ///
    /// This is the primary method for sending data through the mixnet via the LP transport.
    /// Requires `init_lp_session()` to be called first to establish the LP cryptographic session.
    ///
    /// # Data Flow (see gateway/src/node/lp_listener/data_handler.rs)
    /// ```text
    /// LP Client → UDP:51264 → LP Data Handler → Mixnet Entry
    ///           LP(Sphinx)      decrypt LP      forward Sphinx
    /// ```
    ///
    /// # Why LP instead of direct to Mix1?
    /// - Client may be behind NAT/firewall (can't reach Mix1 directly)
    /// - LP provides authenticated, encrypted session with gateway
    /// - This is the standard client-to-gateway transport for lewes-protocol
    ///
    /// Returns SURB encryption keys needed to decrypt replies.
    pub async fn send_data_via_lp(
        &mut self,
        payload: &[u8],
        num_surbs: usize,
    ) -> Result<Vec<SurbEncryptionKey>> {
        // Check LP session exists first (before borrowing self mutably)
        if !self.has_lp_session() {
            bail!("LP session not initialized - call init_lp_session() first");
        }

        let prepared = self.prepare_sphinx_fragments(payload, num_surbs).await?;

        // Now get mutable references after prepare_sphinx_fragments is done
        let lp_client = self.lp_client.as_mut().unwrap(); // safe: checked above
        let socket = self.socket.as_ref().context("socket not initialized")?;
        let lp_data_address = self.gateway.lp_data_address;

        let mut total_sent = 0usize;
        let fragment_count = prepared.fragments.len();

        for fragment in prepared.fragments {
            let nym_packet = NymPacket::sphinx_build(
                false,
                PacketSize::RegularPacket.payload_size(),
                fragment.into_bytes(),
                &prepared.route,
                &prepared.destination,
                &prepared.delays,
            )?;

            // Wrap in MixPacket v2: packet_type || key_rotation || next_hop || sphinx_data
            let mix_packet = MixPacket::new(
                prepared.first_hop_addr,
                nym_packet,
                PacketType::Mix,
                SphinxKeyRotation::Unknown,
            );

            let mix_bytes = mix_packet
                .into_v2_bytes()
                .context("failed to serialize MixPacket")?;

            // Wrap in LP for UDP data plane
            let lp_packet = lp_client
                .wrap_data(&mix_bytes)
                .context("failed to wrap in LP")?;

            // Send to gateway's LP data port (51264) with timeout
            tokio::time::timeout(
                Duration::from_secs(5),
                socket.send_to(&lp_packet, lp_data_address),
            )
            .await
            .context("UDP send timed out")?
            .context("UDP send failed")?;
            total_sent += lp_packet.len();
        }

        info!(
            "Sent {} bytes via LP ({} fragments, {} SURBs) to {}",
            total_sent, fragment_count, num_surbs, lp_data_address
        );

        Ok(prepared.encryption_keys)
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
        assert!(!packet.is_empty(), "packet should not be empty");

        // Verify we can frame it
        let framed =
            FramedNymPacket::new(packet, PacketType::Mix, SphinxKeyRotation::Unknown, false);

        let mut buf = BytesMut::new();
        let mut codec = NymCodec;
        let encode_result = codec.encode(framed, &mut buf);
        assert!(
            encode_result.is_ok(),
            "framing failed: {:?}",
            encode_result.err()
        );
        assert!(!buf.is_empty(), "encoded buffer should not be empty");
    }
}
