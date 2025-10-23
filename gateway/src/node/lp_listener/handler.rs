// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::handshake::LpGatewayHandshake;
use super::messages::{LpRegistrationRequest, LpRegistrationResponse};
use super::registration::process_registration;
use super::LpHandlerState;
use crate::error::GatewayError;
use nym_lp::{
    keypair::{Keypair, PublicKey},
    LpMessage, LpPacket, LpSession,
};
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::*;

pub struct LpConnectionHandler {
    stream: TcpStream,
    remote_addr: SocketAddr,
    state: LpHandlerState,
}

impl LpConnectionHandler {
    pub fn new(stream: TcpStream, remote_addr: SocketAddr, state: LpHandlerState) -> Self {
        Self {
            stream,
            remote_addr,
            state,
        }
    }

    pub async fn handle(mut self) -> Result<(), GatewayError> {
        debug!("Handling LP connection from {}", self.remote_addr);

        // For LP, we need:
        // 1. Gateway's keypair (from local_identity)
        // 2. Client's public key (will be received during handshake)
        // 3. PSK (pre-shared key) - for now use a placeholder

        // Generate fresh LP keypair (x25519) for this connection
        // Using Keypair::default() which generates a new random x25519 keypair
        // This is secure and simple - each connection gets its own keypair
        let gateway_keypair = Keypair::default();

        // Receive client's public key and salt via ClientHello message
        // The client initiates by sending ClientHello as first packet
        let (client_pubkey, salt) = self.receive_client_hello().await?;

        // Derive PSK using ECDH + Blake3 KDF (nym-109)
        // Both client and gateway derive the same PSK from their respective keys
        let psk = nym_lp::derive_psk(
            gateway_keypair.private_key(),
            &client_pubkey,
            &salt,
        );
        tracing::trace!("Derived PSK from LP keys and ClientHello salt");

        // Create LP handshake as responder
        let handshake = LpGatewayHandshake::new_responder(
            &gateway_keypair,
            &client_pubkey,
            &psk,
        )?;

        // Complete the LP handshake
        let session = handshake.complete(&mut self.stream).await?;

        info!("LP handshake completed for {} (session {})",
              self.remote_addr, session.id());

        // After handshake, receive registration request
        let request = self.receive_registration_request(&session).await?;

        debug!("LP registration request from {}: mode={:?}",
               self.remote_addr, request.mode);

        // Process registration (verify credentials, add peer, etc.)
        let response = process_registration(request, &self.state).await;

        // Send response
        if let Err(e) = self.send_registration_response(&session, response.clone()).await {
            warn!("Failed to send LP response to {}: {}", self.remote_addr, e);
            return Err(e);
        }

        if response.success {
            info!("LP registration successful for {} (session {})",
                  self.remote_addr, response.session_id);
        } else {
            warn!("LP registration failed for {}: {:?}",
                  self.remote_addr, response.error);
        }

        Ok(())
    }

    /// Validates that a ClientHello timestamp is within the acceptable time window.
    ///
    /// # Arguments
    /// * `client_timestamp` - Unix timestamp (seconds) from ClientHello salt
    /// * `tolerance_secs` - Maximum acceptable age in seconds
    ///
    /// # Returns
    /// * `Ok(())` if timestamp is valid (within tolerance window)
    /// * `Err(GatewayError)` if timestamp is too old or too far in the future
    ///
    /// # Security
    /// This prevents replay attacks by rejecting stale ClientHello messages.
    /// The tolerance window should be:
    /// - Large enough for clock skew + network latency
    /// - Small enough to limit replay attack window
    fn validate_timestamp(client_timestamp: u64, tolerance_secs: u64) -> Result<(), GatewayError> {
        use std::time::{SystemTime, UNIX_EPOCH};

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time before UNIX epoch")
            .as_secs();

        let age = if now >= client_timestamp {
            now - client_timestamp
        } else {
            // Client timestamp is in the future
            client_timestamp - now
        };

        if age > tolerance_secs {
            let direction = if now >= client_timestamp {
                "old"
            } else {
                "future"
            };
            return Err(GatewayError::LpProtocolError(format!(
                "ClientHello timestamp is too {} (age: {}s, tolerance: {}s)",
                direction, age, tolerance_secs
            )));
        }

        Ok(())
    }

    /// Receive client's public key and salt via ClientHello message
    async fn receive_client_hello(&mut self) -> Result<(PublicKey, [u8; 32]), GatewayError> {
        // Receive first packet which should be ClientHello
        let packet = self.receive_lp_packet().await?;

        // Verify it's a ClientHello message
        match packet.message() {
            LpMessage::ClientHello(hello_data) => {
                // Validate protocol version (currently only v1)
                if hello_data.protocol_version != 1 {
                    return Err(GatewayError::LpProtocolError(
                        format!("Unsupported protocol version: {}", hello_data.protocol_version)
                    ));
                }

                // Extract and validate timestamp (nym-110: replay protection)
                let timestamp = hello_data.extract_timestamp();
                Self::validate_timestamp(timestamp, self.state.lp_config.timestamp_tolerance_secs)?;

                tracing::debug!(
                    "ClientHello timestamp validated: {} (age: {}s, tolerance: {}s)",
                    timestamp,
                    {
                        use std::time::{SystemTime, UNIX_EPOCH};
                        let now = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .expect("System time before UNIX epoch")
                            .as_secs();
                        if now >= timestamp {
                            now - timestamp
                        } else {
                            timestamp - now
                        }
                    },
                    self.state.lp_config.timestamp_tolerance_secs
                );

                // Convert bytes to PublicKey
                let client_pubkey = PublicKey::from_bytes(&hello_data.client_lp_public_key)
                    .map_err(|e| GatewayError::LpProtocolError(
                        format!("Invalid client public key: {}", e)
                    ))?;

                // Extract salt for PSK derivation
                let salt = hello_data.salt;

                Ok((client_pubkey, salt))
            }
            other => {
                Err(GatewayError::LpProtocolError(
                    format!("Expected ClientHello, got {}", other)
                ))
            }
        }
    }

    /// Receive registration request after handshake
    async fn receive_registration_request(
        &mut self,
        session: &LpSession,
    ) -> Result<LpRegistrationRequest, GatewayError> {
        // Read LP packet containing the registration request
        let packet = self.receive_lp_packet().await?;

        // Verify it's from the correct session
        if packet.header().session_id != session.id() {
            return Err(GatewayError::LpProtocolError(
                format!("Session ID mismatch: expected {}, got {}",
                        session.id(), packet.header().session_id)
            ));
        }

        // Extract registration request from LP message
        match packet.message() {
            LpMessage::EncryptedData(data) => {
                // Deserialize registration request
                bincode::deserialize(&data)
                    .map_err(|e| GatewayError::LpProtocolError(
                        format!("Failed to deserialize registration request: {}", e)
                    ))
            }
            other => {
                Err(GatewayError::LpProtocolError(
                    format!("Expected EncryptedData message, got {:?}", other)
                ))
            }
        }
    }

    /// Send registration response after processing
    async fn send_registration_response(
        &mut self,
        session: &LpSession,
        response: LpRegistrationResponse,
    ) -> Result<(), GatewayError> {
        // Serialize response
        let data = bincode::serialize(&response)
            .map_err(|e| GatewayError::LpProtocolError(
                format!("Failed to serialize response: {}", e)
            ))?;

        // Create LP packet with response
        let packet = session.create_data_packet(data)
            .map_err(|e| GatewayError::LpProtocolError(
                format!("Failed to create data packet: {}", e)
            ))?;

        // Send the packet
        self.send_lp_packet(&packet).await
    }

    /// Receive an LP packet from the stream with proper length-prefixed framing
    async fn receive_lp_packet(&mut self) -> Result<LpPacket, GatewayError> {
        use nym_lp::codec::parse_lp_packet;

        // Read 4-byte length prefix (u32 big-endian)
        let mut len_buf = [0u8; 4];
        self.stream.read_exact(&mut len_buf).await
            .map_err(|e| GatewayError::LpConnectionError(
                format!("Failed to read packet length: {}", e)
            ))?;

        let packet_len = u32::from_be_bytes(len_buf) as usize;

        // Sanity check to prevent huge allocations
        const MAX_PACKET_SIZE: usize = 65536; // 64KB max
        if packet_len > MAX_PACKET_SIZE {
            return Err(GatewayError::LpProtocolError(
                format!("Packet size {} exceeds maximum {}", packet_len, MAX_PACKET_SIZE)
            ));
        }

        // Read the actual packet data
        let mut packet_buf = vec![0u8; packet_len];
        self.stream.read_exact(&mut packet_buf).await
            .map_err(|e| GatewayError::LpConnectionError(
                format!("Failed to read packet data: {}", e)
            ))?;

        parse_lp_packet(&packet_buf)
            .map_err(|e| GatewayError::LpProtocolError(
                format!("Failed to parse LP packet: {}", e)
            ))
    }

    /// Send an LP packet over the stream with proper length-prefixed framing
    async fn send_lp_packet(&mut self, packet: &LpPacket) -> Result<(), GatewayError> {
        use nym_lp::codec::serialize_lp_packet;
        use bytes::BytesMut;

        // Serialize the packet first
        let mut packet_buf = BytesMut::new();
        serialize_lp_packet(packet, &mut packet_buf)
            .map_err(|e| GatewayError::LpProtocolError(
                format!("Failed to serialize packet: {}", e)
            ))?;

        // Send 4-byte length prefix (u32 big-endian)
        let len = packet_buf.len() as u32;
        self.stream.write_all(&len.to_be_bytes()).await
            .map_err(|e| GatewayError::LpConnectionError(
                format!("Failed to send packet length: {}", e)
            ))?;

        // Send the actual packet data
        self.stream.write_all(&packet_buf).await
            .map_err(|e| GatewayError::LpConnectionError(
                format!("Failed to send packet data: {}", e)
            ))?;

        self.stream.flush().await
            .map_err(|e| GatewayError::LpConnectionError(
                format!("Failed to flush stream: {}", e)
            ))?;

        Ok(())
    }
}

// Extension trait for LpSession to create packets
// This would ideally be part of nym-lp
trait LpSessionExt {
    fn create_data_packet(&self, data: Vec<u8>) -> Result<LpPacket, nym_lp::LpError>;
}

impl LpSessionExt for LpSession {
    fn create_data_packet(&self, data: Vec<u8>) -> Result<LpPacket, nym_lp::LpError> {
        use nym_lp::packet::LpHeader;

        let header = LpHeader {
            protocol_version: 1,
            session_id: self.id(),
            counter: 0, // TODO: Use actual counter from session
        };

        let message = LpMessage::EncryptedData(data);

        Ok(LpPacket::new(header, message))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;
    use nym_lp::keypair::Keypair;
    use nym_lp::message::{ClientHelloData, LpMessage};
    use nym_lp::packet::{LpHeader, LpPacket};
    use nym_lp::codec::{serialize_lp_packet, parse_lp_packet};
    use std::sync::Arc;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use crate::node::ActiveClientsStore;
    use crate::node::lp_listener::LpConfig;

    // ==================== Test Helpers ====================

    /// Create a minimal test state for handler tests
    async fn create_minimal_test_state() -> LpHandlerState {
        use nym_crypto::asymmetric::ed25519;
        use rand::rngs::OsRng;

        // Create in-memory storage for testing
        let storage = nym_gateway_storage::GatewayStorage::init(":memory:", 100)
            .await
            .expect("Failed to create test storage");

        // Create mock ecash manager for testing
        let ecash_verifier = nym_credential_verification::ecash::MockEcashManager::new(
            Box::new(storage.clone())
        );

        LpHandlerState {
            lp_config: LpConfig {
                enabled: true,
                timestamp_tolerance_secs: 30,
                ..Default::default()
            },
            ecash_verifier: Arc::new(ecash_verifier) as Arc<dyn nym_credential_verification::ecash::traits::EcashManager + Send + Sync>,
            storage,
            local_identity: Arc::new(ed25519::KeyPair::new(&mut OsRng)),
            metrics: nym_node_metrics::NymNodeMetrics::default(),
            active_clients_store: ActiveClientsStore::new(),
            wg_peer_controller: None,
            wireguard_data: None,
        }
    }

    /// Helper to write an LP packet to a stream with proper framing
    async fn write_lp_packet_to_stream<W: AsyncWriteExt + Unpin>(
        stream: &mut W,
        packet: &LpPacket,
    ) -> Result<(), std::io::Error> {
        let mut packet_buf = BytesMut::new();
        serialize_lp_packet(packet, &mut packet_buf)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

        // Write length prefix
        let len = packet_buf.len() as u32;
        stream.write_all(&len.to_be_bytes()).await?;

        // Write packet data
        stream.write_all(&packet_buf).await?;
        stream.flush().await?;

        Ok(())
    }

    /// Helper to read an LP packet from a stream with proper framing
    async fn read_lp_packet_from_stream<R: AsyncReadExt + Unpin>(
        stream: &mut R,
    ) -> Result<LpPacket, std::io::Error> {
        // Read length prefix
        let mut len_buf = [0u8; 4];
        stream.read_exact(&mut len_buf).await?;
        let packet_len = u32::from_be_bytes(len_buf) as usize;

        // Read packet data
        let mut packet_buf = vec![0u8; packet_len];
        stream.read_exact(&mut packet_buf).await?;

        // Parse packet
        parse_lp_packet(&packet_buf)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
    }

    // ==================== Existing Tests ====================

    #[test]
    fn test_validate_timestamp_current() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Current timestamp should always pass
        assert!(LpConnectionHandler::validate_timestamp(now, 30).is_ok());
    }

    #[test]
    fn test_validate_timestamp_within_tolerance() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // 10 seconds old, tolerance 30s -> should pass
        let old_timestamp = now - 10;
        assert!(LpConnectionHandler::validate_timestamp(old_timestamp, 30).is_ok());

        // 10 seconds in future, tolerance 30s -> should pass
        let future_timestamp = now + 10;
        assert!(LpConnectionHandler::validate_timestamp(future_timestamp, 30).is_ok());
    }

    #[test]
    fn test_validate_timestamp_too_old() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // 60 seconds old, tolerance 30s -> should fail
        let old_timestamp = now - 60;
        let result = LpConnectionHandler::validate_timestamp(old_timestamp, 30);
        assert!(result.is_err());
        assert!(format!("{:?}", result).contains("too old"));
    }

    #[test]
    fn test_validate_timestamp_too_far_future() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // 60 seconds in future, tolerance 30s -> should fail
        let future_timestamp = now + 60;
        let result = LpConnectionHandler::validate_timestamp(future_timestamp, 30);
        assert!(result.is_err());
        assert!(format!("{:?}", result).contains("too future"));
    }

    #[test]
    fn test_validate_timestamp_boundary() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Exactly at tolerance boundary -> should pass
        let boundary_timestamp = now - 30;
        assert!(LpConnectionHandler::validate_timestamp(boundary_timestamp, 30).is_ok());

        // Just beyond boundary -> should fail
        let beyond_timestamp = now - 31;
        assert!(LpConnectionHandler::validate_timestamp(beyond_timestamp, 30).is_err());
    }

    // ==================== Packet I/O Tests ====================

    #[tokio::test]
    async fn test_receive_lp_packet_valid() {
        use tokio::net::{TcpListener, TcpStream};

        // Bind to localhost
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        // Spawn server task
        let server_task = tokio::spawn(async move {
            let (stream, remote_addr) = listener.accept().await.unwrap();
            let state = create_minimal_test_state().await;
            let mut handler = LpConnectionHandler::new(stream, remote_addr, state);
            handler.receive_lp_packet().await
        });

        // Connect as client
        let mut client_stream = TcpStream::connect(addr).await.unwrap();

        // Send a valid packet from client side
        let packet = LpPacket::new(
            LpHeader {
                protocol_version: 1,
                session_id: 42,
                counter: 0,
            },
            LpMessage::Busy,
        );
        write_lp_packet_to_stream(&mut client_stream, &packet).await.unwrap();

        // Handler should receive and parse it correctly
        let received = server_task.await.unwrap().unwrap();
        assert_eq!(received.header().protocol_version, 1);
        assert_eq!(received.header().session_id, 42);
        assert_eq!(received.header().counter, 0);
    }

    #[tokio::test]
    async fn test_receive_lp_packet_exceeds_max_size() {
        use tokio::net::{TcpListener, TcpStream};

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_task = tokio::spawn(async move {
            let (stream, remote_addr) = listener.accept().await.unwrap();
            let state = create_minimal_test_state().await;
            let mut handler = LpConnectionHandler::new(stream, remote_addr, state);
            handler.receive_lp_packet().await
        });

        let mut client_stream = TcpStream::connect(addr).await.unwrap();

        // Send a packet size that exceeds MAX_PACKET_SIZE (64KB)
        let oversized_len: u32 = 70000; // > 65536
        client_stream.write_all(&oversized_len.to_be_bytes()).await.unwrap();
        client_stream.flush().await.unwrap();

        // Handler should reject it
        let result = server_task.await.unwrap();
        assert!(result.is_err());
        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(err_msg.contains("exceeds maximum"));
    }

    #[tokio::test]
    async fn test_send_lp_packet_valid() {
        use tokio::net::{TcpListener, TcpStream};

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_task = tokio::spawn(async move {
            let (stream, remote_addr) = listener.accept().await.unwrap();
            let state = create_minimal_test_state().await;
            let mut handler = LpConnectionHandler::new(stream, remote_addr, state);

            let packet = LpPacket::new(
                LpHeader {
                    protocol_version: 1,
                    session_id: 99,
                    counter: 5,
                },
                LpMessage::Busy,
            );
            handler.send_lp_packet(&packet).await
        });

        let mut client_stream = TcpStream::connect(addr).await.unwrap();

        // Wait for server to send
        server_task.await.unwrap().unwrap();

        // Client should receive it correctly
        let received = read_lp_packet_from_stream(&mut client_stream).await.unwrap();
        assert_eq!(received.header().session_id, 99);
        assert_eq!(received.header().counter, 5);
    }

    #[tokio::test]
    async fn test_send_receive_handshake_message() {
        use tokio::net::{TcpListener, TcpStream};

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let handshake_data = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let expected_data = handshake_data.clone();

        let server_task = tokio::spawn(async move {
            let (stream, remote_addr) = listener.accept().await.unwrap();
            let state = create_minimal_test_state().await;
            let mut handler = LpConnectionHandler::new(stream, remote_addr, state);

            let packet = LpPacket::new(
                LpHeader {
                    protocol_version: 1,
                    session_id: 100,
                    counter: 10,
                },
                LpMessage::Handshake(handshake_data),
            );
            handler.send_lp_packet(&packet).await
        });

        let mut client_stream = TcpStream::connect(addr).await.unwrap();
        server_task.await.unwrap().unwrap();

        let received = read_lp_packet_from_stream(&mut client_stream).await.unwrap();
        assert_eq!(received.header().session_id, 100);
        assert_eq!(received.header().counter, 10);
        match received.message() {
            LpMessage::Handshake(data) => assert_eq!(data, &expected_data),
            _ => panic!("Expected Handshake message"),
        }
    }

    #[tokio::test]
    async fn test_send_receive_encrypted_data_message() {
        use tokio::net::{TcpListener, TcpStream};

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let encrypted_payload = vec![42u8; 256];
        let expected_payload = encrypted_payload.clone();

        let server_task = tokio::spawn(async move {
            let (stream, remote_addr) = listener.accept().await.unwrap();
            let state = create_minimal_test_state().await;
            let mut handler = LpConnectionHandler::new(stream, remote_addr, state);

            let packet = LpPacket::new(
                LpHeader {
                    protocol_version: 1,
                    session_id: 200,
                    counter: 20,
                },
                LpMessage::EncryptedData(encrypted_payload),
            );
            handler.send_lp_packet(&packet).await
        });

        let mut client_stream = TcpStream::connect(addr).await.unwrap();
        server_task.await.unwrap().unwrap();

        let received = read_lp_packet_from_stream(&mut client_stream).await.unwrap();
        assert_eq!(received.header().session_id, 200);
        assert_eq!(received.header().counter, 20);
        match received.message() {
            LpMessage::EncryptedData(data) => assert_eq!(data, &expected_payload),
            _ => panic!("Expected EncryptedData message"),
        }
    }

    #[tokio::test]
    async fn test_send_receive_client_hello_message() {
        use tokio::net::{TcpListener, TcpStream};
        use nym_lp::message::ClientHelloData;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let client_key = [7u8; 32];
        let hello_data = ClientHelloData::new_with_fresh_salt(client_key, 1);
        let expected_salt = hello_data.salt; // Clone salt before moving hello_data

        let server_task = tokio::spawn(async move {
            let (stream, remote_addr) = listener.accept().await.unwrap();
            let state = create_minimal_test_state().await;
            let mut handler = LpConnectionHandler::new(stream, remote_addr, state);

            let packet = LpPacket::new(
                LpHeader {
                    protocol_version: 1,
                    session_id: 300,
                    counter: 30,
                },
                LpMessage::ClientHello(hello_data),
            );
            handler.send_lp_packet(&packet).await
        });

        let mut client_stream = TcpStream::connect(addr).await.unwrap();
        server_task.await.unwrap().unwrap();

        let received = read_lp_packet_from_stream(&mut client_stream).await.unwrap();
        assert_eq!(received.header().session_id, 300);
        assert_eq!(received.header().counter, 30);
        match received.message() {
            LpMessage::ClientHello(data) => {
                assert_eq!(data.client_lp_public_key, client_key);
                assert_eq!(data.protocol_version, 1);
                assert_eq!(data.salt, expected_salt);
            }
            _ => panic!("Expected ClientHello message"),
        }
    }

    // ==================== receive_client_hello Tests ====================

    #[tokio::test]
    async fn test_receive_client_hello_valid() {
        use tokio::net::{TcpListener, TcpStream};

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_task = tokio::spawn(async move {
            let (stream, remote_addr) = listener.accept().await.unwrap();
            let state = create_minimal_test_state().await;
            let mut handler = LpConnectionHandler::new(stream, remote_addr, state);
            handler.receive_client_hello().await
        });

        let mut client_stream = TcpStream::connect(addr).await.unwrap();

        // Create and send valid ClientHello
        let client_keypair = Keypair::default();
        let hello_data = ClientHelloData::new_with_fresh_salt(
            client_keypair.public_key().to_bytes(),
            1, // protocol version
        );
        let packet = LpPacket::new(
            LpHeader {
                protocol_version: 1,
                session_id: 0,
                counter: 0,
            },
            LpMessage::ClientHello(hello_data.clone()),
        );
        write_lp_packet_to_stream(&mut client_stream, &packet).await.unwrap();

        // Handler should receive and parse it
        let result = server_task.await.unwrap();
        assert!(result.is_ok());

        let (pubkey, salt) = result.unwrap();
        assert_eq!(pubkey.as_bytes(), &client_keypair.public_key().to_bytes());
        assert_eq!(salt, hello_data.salt);
    }

    #[tokio::test]
    async fn test_receive_client_hello_timestamp_too_old() {
        use tokio::net::{TcpListener, TcpStream};
        use std::time::{SystemTime, UNIX_EPOCH};

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_task = tokio::spawn(async move {
            let (stream, remote_addr) = listener.accept().await.unwrap();
            let state = create_minimal_test_state().await;
            let mut handler = LpConnectionHandler::new(stream, remote_addr, state);
            handler.receive_client_hello().await
        });

        let mut client_stream = TcpStream::connect(addr).await.unwrap();

        // Create ClientHello with old timestamp
        let client_keypair = Keypair::default();
        let mut hello_data = ClientHelloData::new_with_fresh_salt(
            client_keypair.public_key().to_bytes(),
            1,
        );

        // Manually set timestamp to be very old (100 seconds ago)
        let old_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() - 100;
        hello_data.salt[..8].copy_from_slice(&old_timestamp.to_le_bytes());

        let packet = LpPacket::new(
            LpHeader {
                protocol_version: 1,
                session_id: 0,
                counter: 0,
            },
            LpMessage::ClientHello(hello_data),
        );
        write_lp_packet_to_stream(&mut client_stream, &packet).await.unwrap();

        // Should fail with timestamp error
        let result = server_task.await.unwrap();
        assert!(result.is_err());
        // Note: Can't use unwrap_err() directly because PublicKey doesn't implement Debug
        // Just check that it failed
        match result {
            Err(e) => {
                let err_msg = format!("{}", e);
                assert!(err_msg.contains("too old"), "Expected 'too old' in error, got: {}", err_msg);
            }
            Ok(_) => panic!("Expected error but got success"),
        }
    }
}
