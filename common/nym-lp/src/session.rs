// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Session management for the Lewes Protocol.
//!
//! This module implements session management functionality, including replay protection
//! and Noise protocol state handling.

use crate::codec::OuterAeadKey;
use crate::message::EncryptedDataPayload;
// noiserm
use crate::noise_protocol::{NoiseError, NoiseProtocol, ReadResult};
use crate::packet::LpHeader;
use crate::peer::{LpLocalPeer, LpRemotePeer};
use crate::psk::derive_subsession_psk;
use crate::psq::PSQHandshakeState;
use crate::replay::ReceivingKeyCounterValidator;
use crate::{LpError, LpMessage, LpPacket};
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_kkt::ciphersuite::{Ciphersuite, HashFunction, HashLength, KEM, SignatureScheme};
use nym_lp_transport::traits::LpTransport;
use parking_lot::Mutex;
use snow::Builder;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// PQ shared secret wrapper with automatic memory zeroization.
/// Ensures K_pq is cleared from memory when dropped.
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct PqSharedSecret([u8; 32]);

impl PqSharedSecret {
    pub fn new(secret: [u8; 32]) -> Self {
        Self(secret)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl std::fmt::Debug for PqSharedSecret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PqSharedSecret")
            .field("secret", &"<redacted>")
            .finish()
    }
}

/// A session in the Lewes Protocol, handling connection state with Noise.
///
/// Sessions manage connection state, including LP replay protection.
/// Each session has a unique receiving index and sending index for connection identification.
#[derive(Debug)]
pub struct LpSession {
    /// Id of the established session
    session_id: u32,

    /// Negotiated protocol version from handshake.
    /// Set during handshake completion from the ClientHello/ServerHello packet header.
    /// Used for future version negotiation and compatibility checks.
    version: u8,

    /// Outer AEAD key for packet encryption (derived from PSK after PSQ handshake).
    outer_aead_key: OuterAeadKey,

    /// Representation of a local Lewes Protocol peer
    /// encapsulating all the known information and keys.
    local_peer: LpLocalPeer,

    /// Representation of a remote Lewes Protocol peer
    /// encapsulating all the known information and keys.
    remote_peer: LpRemotePeer,

    // TODO: ALL BELOW maybe not needed after all?
    /// Raw PQ shared secret (K_pq) from PSQ KEM encapsulation/decapsulation.
    /// Stored after PSQ handshake completes for subsession PSK derivation.
    pq_shared_secret: PqSharedSecret,

    /// Noise protocol state machine
    noise_state: NoiseProtocol,

    /// Counter for outgoing packets
    sending_counter: u64,

    /// Validator for incoming packet counters to prevent replay attacks
    receiving_counter: ReceivingKeyCounterValidator,

    /// Monotonically increasing counter for subsession indices.
    /// Each subsession gets a unique index to ensure unique PSK derivation.
    /// Uses u64 to make overflow practically impossible (~585k years at 1M/sec).
    subsession_counter: u64,

    /// True if this session has been demoted to read-only mode.
    /// Demoted sessions can still receive/decrypt but cannot send/encrypt.
    read_only: bool,

    /// ID of the successor session that replaced this one.
    /// Set when demote() is called.
    successor_session_id: Option<u32>,
}

impl LpSession {
    /// Creates a new session after completed KTT/PSQ exchange
    ///
    /// # Arguments
    ///
    /// * `session_id` - Session identifier
    /// * `version` - Protocol version to attach in all `LpPacket`s
    /// * `outer_aead_key` - Outer AEAD key for packet encryption
    /// * `local_peer` - This side's LP peer's keys
    /// * `remote_peer` - The remote's LP peer's keys
    /// * `pq_shared_secret` - Raw PQ shared secret (K_pq) from PSQ KEM encapsulation/decapsulation.
    /// * `noise_state` - Noise protocol state machine
    pub fn new(
        session_id: u32,
        version: u8,
        outer_aead_key: OuterAeadKey,
        local_peer: LpLocalPeer,
        remote_peer: LpRemotePeer,
        pq_shared_secret: PqSharedSecret,
        noise_state: NoiseProtocol,
    ) -> Self {
        LpSession {
            session_id,
            version,
            outer_aead_key,
            local_peer,
            remote_peer,
            pq_shared_secret,
            noise_state,
            sending_counter: 0,
            receiving_counter: Default::default(),
            subsession_counter: 0,
            read_only: false,
            successor_session_id: None,
        }
    }

    /// Create an instance of `Ciphersuite` using hardcoded defaults.
    /// This is a temporary workaround until values can be properly inferred
    /// from reported version
    pub fn default_ciphersuite() -> Ciphersuite {
        Ciphersuite::new(
            KEM::X25519,
            HashFunction::Blake3,
            SignatureScheme::Ed25519,
            HashLength::Default,
        )
    }

    pub fn psq_handshake_state<S>(
        connection: &'_ mut S,
        ciphersuite: Ciphersuite,
        local_peer: LpLocalPeer,
        remote_peer: Option<LpRemotePeer>,
    ) -> PSQHandshakeState<'_, S>
    where
        S: LpTransport + Unpin,
    {
        PSQHandshakeState::new(connection, ciphersuite, local_peer, remote_peer)
    }

    pub fn id(&self) -> u32 {
        self.session_id
    }

    /// Returns the negotiated protocol version from the handshake.
    ///
    /// Set during `LpSession` creation after sending / receiving `ClientHelloData`
    pub fn negotiated_version(&self) -> u8 {
        self.version
    }

    /// Returns the local X25519 public key.
    ///
    /// This is used for KKT protocol when the responder needs to send their
    /// KEM public key in the KKT response.
    pub fn local_x25519_public(&self) -> x25519::PublicKey {
        *self.local_peer.x25519.public_key()
    }

    /// Returns the remote ed25519 public key.
    pub fn remote_ed25519_public(&self) -> ed25519::PublicKey {
        self.remote_peer.ed25519_public
    }

    /// Returns the remote X25519 public key.
    ///
    /// Used for tie-breaking in simultaneous subsession initiation.
    /// Lower key loses and becomes responder.
    pub fn remote_x25519_public(&self) -> &x25519::PublicKey {
        &self.remote_peer.x25519_public
    }

    /// Returns the outer AEAD key for packet encryption/decryption.
    pub fn outer_aead_key(&self) -> &OuterAeadKey {
        &self.outer_aead_key
    }

    pub fn next_packet(&mut self, message: LpMessage) -> Result<LpPacket, LpError> {
        let counter = self.next_counter();
        let header = LpHeader::new(self.id(), counter, self.version);
        let packet = LpPacket::new(header, message);
        Ok(packet)
    }

    /// Generates the next counter value for outgoing packets.
    pub fn next_counter(&mut self) -> u64 {
        let counter = self.sending_counter;
        self.sending_counter += 1;
        counter
    }

    /// Performs a quick validation check for an incoming packet counter.
    ///
    /// This should be called before performing any expensive operations like
    /// decryption/Noise processing to efficiently filter out potential replay attacks.
    ///
    /// # Arguments
    ///
    /// * `counter` - The counter value to check
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the counter is likely valid
    /// * `Err(LpError::Replay)` if the counter is invalid or a potential replay
    pub fn receiving_counter_quick_check(&self, counter: u64) -> Result<(), LpError> {
        // Branchless implementation uses SIMD when available for constant-time
        // operations, preventing timing attacks. Check before crypto to save CPU cycles.
        self.receiving_counter
            .will_accept_branchless(counter)
            .map_err(LpError::Replay)
    }

    /// Marks a counter as received after successful packet processing.
    ///
    /// This should be called after a packet has been successfully decoded and processed
    /// (including Noise decryption/handshake step) to update the replay protection state.
    ///
    /// # Arguments
    ///
    /// * `counter` - The counter value to mark as received
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the counter was successfully marked
    /// * `Err(LpError::Replay)` if the counter cannot be marked (duplicate, too old, etc.)
    pub fn receiving_counter_mark(&mut self, counter: u64) -> Result<(), LpError> {
        self.receiving_counter
            .mark_did_receive_branchless(counter)
            .map_err(LpError::Replay)
    }

    /// Returns current packet statistics for monitoring.
    ///
    /// # Returns
    ///
    /// A tuple containing:
    /// * The next expected counter value for incoming packets
    /// * The total number of received packets
    pub fn current_packet_cnt(&self) -> (u64, u64) {
        self.receiving_counter.current_packet_cnt()
    }

    /// Returns the PQ shared secret (K_pq).
    ///
    /// This is the raw KEM output from PSQ before Blake3 KDF combination.
    /// Used for deriving subsession PSKs to maintain PQ protection.
    pub fn pq_shared_secret(&self) -> &PqSharedSecret {
        &self.pq_shared_secret
    }

    /// Gets the next subsession index and increments the counter.
    ///
    /// Each subsession requires a unique index to ensure unique PSK derivation.
    /// The index is monotonically increasing per session.
    pub fn next_subsession_index(&mut self) -> u64 {
        let next = self.subsession_counter;
        self.subsession_counter += 1;
        next
    }

    /// Returns true if this session is in read-only mode.
    ///
    /// Read-only sessions have been demoted after a subsession was promoted.
    /// They can still decrypt incoming messages but cannot encrypt outgoing ones.
    pub fn is_read_only(&self) -> bool {
        self.read_only
    }

    /// Demotes this session to read-only mode after a subsession replaces it.
    ///
    /// After demotion:
    /// - `encrypt_data()` will return `NoiseError::SessionReadOnly`
    /// - `decrypt_data()` still works (to drain in-flight messages)
    /// - Session should be cleaned up after TTL expires
    ///
    /// # Arguments
    /// * `successor_idx` - The receiver index of the session that replaced this one
    pub fn demote(&mut self, successor_idx: u32) {
        self.successor_session_id = Some(successor_idx);
        self.read_only = true;
    }

    /// Returns the successor session ID if this session was demoted.
    pub fn successor_session_id(&self) -> Option<u32> {
        self.successor_session_id
    }

    /// Encrypts application data payload using the established Noise transport session.
    ///
    /// # Arguments
    ///
    /// * `payload` - The application data to encrypt.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<u8>)` containing the encrypted Noise message ciphertext.
    /// * `Err(NoiseError)` if the session is not in transport mode or encryption fails.
    pub fn encrypt_data(&mut self, payload: &[u8]) -> Result<LpMessage, NoiseError> {
        // Check if session is read-only (demoted)
        if self.read_only {
            return Err(NoiseError::SessionReadOnly);
        }

        let payload = self.noise_state.write_message(payload)?;
        Ok(LpMessage::EncryptedData(EncryptedDataPayload(payload)))
    }

    /// Decrypts an incoming Noise message containing application data.
    ///
    /// # Arguments
    ///
    /// * `noise_ciphertext` - The encrypted Noise message received from the peer.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<u8>)` containing the decrypted application data payload.
    /// * `Err(NoiseError)` if the session is not in transport mode, decryption fails, or the message is not data.
    pub fn decrypt_data(&mut self, noise_ciphertext: &LpMessage) -> Result<Vec<u8>, NoiseError> {
        let payload = noise_ciphertext.payload();

        match self.noise_state.read_message(payload)? {
            ReadResult::DecryptedData(data) => Ok(data),
            _ => Err(NoiseError::IncorrectStateError),
        }
    }

    /// Creates a new subsession using Noise KKpsk0 pattern.
    ///
    /// KKpsk0 reuses parent's static X25519 keys (both parties know each other from parent session).
    /// PSK is derived from parent's PQ shared secret, preserving quantum resistance.
    ///
    /// # Arguments
    /// * `subsession_index` - Unique index for this subsession (use `next_subsession_index()`)
    /// * `is_initiator` - True if this side initiates the subsession handshake
    ///
    /// # Returns
    /// `SubsessionHandshake` ready for KK1/KK2 message exchange
    ///
    /// # Errors
    /// * Returns error if parent handshake not complete
    /// * Returns error if PQ shared secret not available
    pub fn create_subsession(
        &self,
        subsession_index: u64,
        is_initiator: bool,
    ) -> Result<SubsessionHandshake, LpError> {
        // Get PQ shared secret
        let pq_secret = self.pq_shared_secret();

        // Derive subsession PSK from parent's PQ shared secret
        let subsession_psk = derive_subsession_psk(pq_secret.as_bytes(), subsession_index);

        // Build KKpsk0 handshake
        // Pattern: Noise_KKpsk0_25519_ChaChaPoly_SHA256
        // Both parties already know each other's static keys from parent session
        let pattern_name = "Noise_KKpsk0_25519_ChaChaPoly_SHA256";
        let params = pattern_name.parse()?;

        let local_key_bytes = self.local_peer.x25519.private_key().to_bytes();
        let remote_key_bytes = self.remote_x25519_public().to_bytes();

        let builder = Builder::new(params)
            .local_private_key(&local_key_bytes)
            .remote_public_key(&remote_key_bytes)
            .psk(0, &subsession_psk); // PSK at position 0 for KKpsk0

        let handshake_state = if is_initiator {
            builder.build_initiator().map_err(LpError::SnowKeyError)?
        } else {
            builder.build_responder().map_err(LpError::SnowKeyError)?
        };

        Ok(SubsessionHandshake {
            index: subsession_index,
            noise_state: Mutex::new(NoiseProtocol::new(handshake_state)),
            is_initiator,
            local_peer: self.local_peer.clone(),
            remote_peer: self.remote_peer.clone(),
            pq_shared_secret: self.pq_shared_secret.clone(),
            subsession_psk,
            negotiated_version: self.version,
        })
    }
}

/// Subsession created via Noise KKpsk0 handshake tunneled through parent session.
///
/// Subsessions provide fresh session keys while inheriting PQ protection from parent's
/// ML-KEM shared secret. After handshake completes, the subsession can be promoted
/// to replace the parent session.
///
/// # Lifecycle
/// 1. Parent calls `create_subsession()` to get `SubsessionHandshake`
/// 2. Initiator calls `prepare_message()` to get KK1
/// 3. KK1 sent through parent session (encrypted tunnel)
/// 4. Responder calls `process_message(kk1)` to process KK1
/// 5. Responder calls `prepare_message()` to get KK2
/// 6. KK2 sent through parent session
/// 7. Initiator calls `process_message(kk2)` to complete handshake
/// 8. Both call `is_complete()` to verify
#[derive(Debug)]
pub struct SubsessionHandshake {
    /// Subsession index (unique per parent session)
    pub index: u64,
    /// Noise KKpsk0 handshake state
    noise_state: Mutex<NoiseProtocol>,
    /// Is this side the initiator?
    is_initiator: bool,

    // Key material inherited from parent session for into_session() conversion
    /// Representation of a local Lewes Protocol peer
    /// encapsulating all the known information and keys.
    local_peer: LpLocalPeer,

    /// Representation of a remote Lewes Protocol peer
    /// encapsulating all the known information and keys.
    remote_peer: LpRemotePeer,

    /// PQ shared secret inherited from parent (for creating further subsessions)
    pq_shared_secret: PqSharedSecret,

    /// Subsession PSK (for deriving outer AEAD key)
    subsession_psk: [u8; 32],

    /// Negotiated protocol version from handshake.
    negotiated_version: u8,
}

impl SubsessionHandshake {
    /// Prepares the next KK handshake message (KK1 or KK2 depending on role/state).
    ///
    /// # Returns
    /// Noise handshake message bytes to send through parent session tunnel.
    pub fn prepare_message(&self) -> Result<Vec<u8>, LpError> {
        let mut noise_state = self.noise_state.lock();
        noise_state
            .get_bytes_to_send()
            .ok_or_else(|| LpError::Internal("Not our turn to send".into()))?
            .map_err(LpError::NoiseError)
    }

    /// Processes a received KK handshake message (KK1 or KK2).
    ///
    /// # Arguments
    /// * `message` - Noise handshake message received through parent session tunnel.
    ///
    /// # Returns
    /// Any payload embedded in the handshake message (usually empty for KK).
    pub fn process_message(&self, message: &[u8]) -> Result<Vec<u8>, LpError> {
        let mut noise_state = self.noise_state.lock();
        let result = noise_state
            .read_message(message)
            .map_err(LpError::NoiseError)?;
        match result {
            ReadResult::HandshakeComplete | ReadResult::NoOp => Ok(vec![]),
            ReadResult::DecryptedData(data) => Ok(data),
        }
    }

    /// Checks if the handshake is complete (ready for transport mode).
    pub fn is_complete(&self) -> bool {
        self.noise_state.lock().is_handshake_finished()
    }

    /// Returns whether this side is the initiator.
    pub fn is_initiator(&self) -> bool {
        self.is_initiator
    }

    /// Returns the subsession index.
    pub fn subsession_index(&self) -> u64 {
        self.index
    }

    /// Convert completed subsession handshake into a full LpSession.
    ///
    /// This consumes the SubsessionHandshake and creates a new LpSession
    /// that can be used as a replacement for the parent session.
    ///
    /// # Arguments
    /// * `receiver_index` - New receiver index for the promoted session
    ///
    /// # Errors
    /// Returns error if handshake is not complete
    pub fn into_session(self, receiver_index: u32) -> Result<LpSession, LpError> {
        if !self.is_complete() {
            return Err(LpError::Internal(
                "Cannot convert incomplete subsession to session".to_string(),
            ));
        }

        // Extract the noise state (now in transport mode)
        let noise_state = self.noise_state.into_inner();

        // Derive outer AEAD key from the subsession PSK
        let outer_key = OuterAeadKey::from_psk(&self.subsession_psk);

        Ok(LpSession {
            // noiserm
            session_id: receiver_index,
            noise_state,
            sending_counter: 0,
            receiving_counter: ReceivingKeyCounterValidator::new(0),
            local_peer: self.local_peer,
            remote_peer: self.remote_peer,
            outer_aead_key: outer_key,
            pq_shared_secret: self.pq_shared_secret,
            subsession_counter: 0,
            read_only: false,
            successor_session_id: None,
            version: self.negotiated_version,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SessionsMock, replay::ReplayError, sessions_for_tests};
    use rand::thread_rng;

    // Helper function to generate keypairs for tests
    fn generate_x25519_keypair() -> x25519::KeyPair {
        x25519::KeyPair::new(&mut thread_rng())
    }

    #[test]
    fn test_session_creation() {
        let mut session = sessions_for_tests().0;

        // Initial counter should be zero
        let counter = session.next_counter();
        assert_eq!(counter, 0);

        // Counter should increment
        let counter = session.next_counter();
        assert_eq!(counter, 1);
    }

    // NOTE: These tests are obsolete after removing optional KEM parameters.
    // PSQ now always runs using X25519 keys internally converted to KEM format.
    // The new tests at the end of this file (test_psq_*) cover PSQ integration.
    /*
    #[test]
    fn test_session_creation_with_psq_state_initiator() {
        // OLD API - REMOVED
    }

    #[test]
    fn test_session_creation_with_psq_state_responder() {
        // OLD API - REMOVED
    }
    */

    #[test]
    fn test_replay_protection_sequential() {
        let mut session = sessions_for_tests().1;

        // Sequential counters should be accepted
        assert!(session.receiving_counter_quick_check(0).is_ok());
        assert!(session.receiving_counter_mark(0).is_ok());

        assert!(session.receiving_counter_quick_check(1).is_ok());
        assert!(session.receiving_counter_mark(1).is_ok());

        // Duplicates should be rejected
        assert!(session.receiving_counter_quick_check(0).is_err());
        let err = session.receiving_counter_mark(0).unwrap_err();
        match err {
            LpError::Replay(replay_error) => {
                assert!(matches!(replay_error, ReplayError::DuplicateCounter));
            }
            _ => panic!("Expected replay error"),
        }
    }

    #[test]
    fn test_replay_protection_out_of_order() {
        let mut session = sessions_for_tests().1;

        // Receive packets in order
        assert!(session.receiving_counter_mark(0).is_ok());
        assert!(session.receiving_counter_mark(1).is_ok());
        assert!(session.receiving_counter_mark(2).is_ok());

        // Skip ahead
        assert!(session.receiving_counter_mark(10).is_ok());

        // Can still receive out-of-order packets within window
        assert!(session.receiving_counter_quick_check(5).is_ok());
        assert!(session.receiving_counter_mark(5).is_ok());

        // But duplicates are still rejected
        assert!(session.receiving_counter_quick_check(5).is_err());
        assert!(session.receiving_counter_mark(5).is_err());
    }

    #[test]
    fn test_packet_stats() {
        let mut session = sessions_for_tests().1;

        // Initial stats
        let (next, received) = session.current_packet_cnt();
        assert_eq!(next, 0);
        assert_eq!(received, 0);

        // After receiving packets
        assert!(session.receiving_counter_mark(0).is_ok());
        assert!(session.receiving_counter_mark(1).is_ok());

        let (next, received) = session.current_packet_cnt();
        assert_eq!(next, 2);
        assert_eq!(received, 2);
    }

    /*
    // These tests remain commented as they rely on the old mock crypto functions
    #[test]
    fn test_mock_crypto() {
        let mut session = create_test_session(true);
        let data = [1, 2, 3, 4, 5];
        let mut encrypted = [0; 5];
        let mut decrypted = [0; 5];

        // Mock encrypt should copy the data
        // let encrypted_len = session.encrypt_packet(&data, &mut encrypted).unwrap(); // Removed method
        // assert_eq!(encrypted_len, 5);
        // assert_eq!(encrypted, data);

        // Mock decrypt should copy the data
        // let decrypted_len = session.decrypt_packet(&encrypted, &mut decrypted).unwrap(); // Removed method
        // assert_eq!(decrypted_len, 5);
        // assert_eq!(decrypted, data);
    }

    #[test]
    fn test_mock_crypto_buffer_too_small() {
        let mut session = create_test_session(true);
        let data = [1, 2, 3, 4, 5];
        let mut too_small = [0; 3];

        // Should fail with buffer too small
        // let result = session.encrypt_packet(&data, &mut too_small); // Removed method
        // assert!(result.is_err());
        // match result.unwrap_err() {
        //     LpError::InsufficientBufferSize => {} // Error type might change
        //     _ => panic!("Expected InsufficientBufferSize error"),
        // }
    }
    */

    /// Test that X25519 keys are correctly converted to KEM format
    #[test]
    fn test_x25519_to_kem_conversion() {
        use nym_kkt::ciphersuite::EncapsulationKey;

        let initiator_keys = generate_x25519_keypair();
        let responder_keys = generate_x25519_keypair();

        // Verify we can convert X25519 public key to KEM format (as done in session.rs)
        let x25519_public_bytes = responder_keys.public_key().as_bytes();
        let libcrux_public_key =
            libcrux_kem::PublicKey::decode(libcrux_kem::Algorithm::X25519, x25519_public_bytes)
                .expect("X25519 public key should convert to libcrux PublicKey");

        let _kem_key = EncapsulationKey::X25519(libcrux_public_key);

        // Verify we can convert X25519 private key to KEM format
        let x25519_private_bytes = initiator_keys.private_key().to_bytes();
        let _libcrux_private_key =
            libcrux_kem::PrivateKey::decode(libcrux_kem::Algorithm::X25519, &x25519_private_bytes)
                .expect("X25519 private key should convert to libcrux PrivateKey");

        // Successful conversion is sufficient - actual encapsulation is tested in psk.rs
        // (libcrux_kem::PrivateKey is an enum with no len() method, conversion success is enough)
    }

    #[test]
    fn test_demote_sets_read_only() {
        let sessions = SessionsMock::mock_post_handshake(12345);
        let mut session = sessions.initiator;

        // Initially not read-only
        assert!(!session.is_read_only());
        assert!(session.successor_session_id().is_none());

        // Demote the session
        session.demote(99999);

        // Now read-only with successor
        assert!(session.is_read_only());
        assert_eq!(session.successor_session_id(), Some(99999));
    }

    #[test]
    fn test_encrypt_fails_after_demotion() {
        let receiver_index = 12345;
        let sessions = SessionsMock::mock_post_handshake(receiver_index);
        let mut initiator_session = sessions.initiator;

        // Encryption works before demotion
        let plaintext = b"Hello before demotion";
        assert!(initiator_session.encrypt_data(plaintext).is_ok());

        // Demote the session
        initiator_session.demote(99999);

        // Encryption fails after demotion
        let result = initiator_session.encrypt_data(plaintext);
        assert!(result.is_err());
        match result.unwrap_err() {
            NoiseError::SessionReadOnly => {
                // Expected
            }
            e => panic!("Expected SessionReadOnly error, got: {:?}", e),
        }
    }

    #[test]
    fn test_decrypt_works_after_demotion() {
        // --- Setup Handshake ---
        let receiver_index = 12345;
        let sessions = SessionsMock::mock_post_handshake(receiver_index);
        let mut initiator_session = sessions.initiator;
        let mut responder_session = sessions.responder;

        // Responder encrypts a message
        let plaintext = b"Message to demoted initiator";
        let ciphertext = responder_session
            .encrypt_data(plaintext)
            .expect("Encryption failed");

        // Demote the initiator session
        initiator_session.demote(99999);
        assert!(initiator_session.is_read_only());

        // Decryption still works on demoted session (drain in-flight)
        let decrypted = initiator_session
            .decrypt_data(&ciphertext)
            .expect("Decryption should work on demoted session");
        assert_eq!(decrypted, plaintext);
    }
}
