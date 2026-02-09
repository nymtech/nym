// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Session management for the Lewes Protocol.
//!
//! This module implements session management functionality, including replay protection
//! and Noise protocol state handling.

use crate::codec::OuterAeadKey;
use crate::message::{EncryptedDataPayload, HandshakeData};
// noiserm
use crate::noise_protocol::{NoiseError, NoiseProtocol, ReadResult};
use crate::packet::LpHeader;
use crate::peer::{LpLocalPeer, LpRemotePeer};
use crate::psk::{
    derive_subsession_psk, psq_initiator_create_message, psq_responder_process_message,
};
use crate::psq::PSQHandshakeState;
use crate::replay::ReceivingKeyCounterValidator;
use crate::{LpError, LpMessage, LpPacket};
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_kkt::KKT_RESPONSE_AAD;
use nym_kkt::ciphersuite::{
    Ciphersuite, DecapsulationKey, EncapsulationKey, HashFunction, HashLength, KEM, SignatureScheme,
};
use nym_kkt::context::KKTContext;
use nym_kkt::encryption::{
    KKTSessionSecret, decrypt_initial_kkt_frame, decrypt_kkt_frame, encrypt_initial_kkt_frame,
    encrypt_kkt_frame,
};
use nym_kkt::session::{
    anonymous_initiator_process, initiator_ingest_response, responder_ingest_message,
    responder_process,
};
use nym_lp_transport::traits::LpTransport;
use parking_lot::Mutex;
use rand::RngCore;
use snow::Builder;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
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
    //
    //
    //
    //

    // id: u32,
    //
    // /// Flag indicating if this session acts as the Noise protocol initiator.
    // is_initiator: bool,
    //
    // // noiserm
    // /// Noise protocol state machine
    // noise_state: Mutex<NoiseProtocol>,
    //
    // /// KKT (KEM Key Transfer) exchange state
    // kkt_state: Mutex<KKTState>,
    //
    // /// PSQ (Post-Quantum Secure PSK) handshake state
    // psq_state: Mutex<PSQState>,
    //
    // /// PSK handle from responder (ctxt_B) for future re-registration
    // psk_handle: Mutex<Option<Vec<u8>>>,
    //
    // /// Counter for outgoing packets
    // sending_counter: AtomicU64,
    //
    // /// Validator for incoming packet counters to prevent replay attacks
    // receiving_counter: Mutex<ReceivingKeyCounterValidator>,
    //
    // // noiserm
    // /// Safety flag: `true` if real PSK was injected via PSQ, `false` if still using dummy PSK.
    // /// This prevents transport mode operations from running with the insecure dummy `[0u8; 32]` PSK.
    // psk_injected: AtomicBool,
    //
    // /// Representation of a local Lewes Protocol peer
    // /// encapsulating all the known information and keys.
    // local_peer: LpLocalPeer,
    //
    // /// Representation of a remote Lewes Protocol peer
    // /// encapsulating all the known information and keys.
    // remote_peer: LpRemotePeer,
    //
    // /// Salt for PSK derivation
    // salt: [u8; 32],
    //
    // /// Outer AEAD key for packet encryption (derived from PSK after PSQ handshake).
    // /// None before PSK is available, Some after PSK injection.
    // outer_aead_key: Mutex<Option<OuterAeadKey>>,
    //
    // /// Raw PQ shared secret (K_pq) from PSQ KEM encapsulation/decapsulation.
    // /// Stored after PSQ handshake completes for subsession PSK derivation.
    // /// This preserves PQ protection when creating subsessions via KKpsk0.
    // /// Wrapped in PqSharedSecret for automatic memory zeroization on drop.
    // pq_shared_secret: Mutex<Option<PqSharedSecret>>,
    //
    // /// Monotonically increasing counter for subsession indices.
    // /// Each subsession gets a unique index to ensure unique PSK derivation.
    // /// Uses u64 to make overflow practically impossible (~585k years at 1M/sec).
    // subsession_counter: AtomicU64,
    //
    // /// True if this session has been demoted to read-only mode.
    // /// Demoted sessions can still receive/decrypt but cannot send/encrypt.
    // read_only: AtomicBool,
    //
    // /// ID of the successor session that replaced this one.
    // /// Set when demote() is called.
    // successor_session_id: Mutex<Option<u32>>,
    //
    // /// Negotiated protocol version from handshake.
    // /// Set during handshake completion from the ClientHello/ServerHello packet header.
    // /// Used for future version negotiation and compatibility checks.
    // negotiated_version: u8,
}

// noiserm
/// Generates a fresh salt for PSK derivation.
///
/// Salt format: 8 bytes timestamp (u64 LE) + 24 bytes random nonce
///
/// This ensures each session derives a unique PSK, even with the same key pairs.
/// The timestamp provides temporal uniqueness while the random nonce prevents collisions.
///
/// # Returns
/// A 32-byte array containing fresh salt material
pub fn generate_fresh_salt() -> [u8; 32] {
    let mut salt = [0u8; 32];

    // First 8 bytes: current timestamp as u64 little-endian
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("System time before UNIX epoch")
        .as_secs();
    salt[..8].copy_from_slice(&timestamp.to_le_bytes());

    // Last 24 bytes: random nonce
    rand::thread_rng().fill_bytes(&mut salt[8..]);

    salt
}

impl LpSession {
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

    pub fn new2(
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

    /// Creates a new session and initializes the Noise protocol state.
    ///
    /// PSQ always runs during the handshake to derive the real PSK from X25519 DHKEM.
    /// The Noise protocol is initialized with a dummy PSK that gets replaced during handshake.
    ///
    /// # Arguments
    ///
    /// * `id` - Session identifier
    /// * `is_initiator` - True if this side initiates the Noise handshake.
    /// * `local_peer` - This side's LP peer's keys
    /// * `remote_peer` - The remote's LP peer's keys
    /// * `salt` - Salt for PSK derivation
    /// * `protocol_version` - Protocol version to attach in all `LpPacket`s
    #[deprecated]
    pub fn new(
        id: u32,
        is_initiator: bool,
        local_peer: LpLocalPeer,
        remote_peer: LpRemotePeer,
        salt: &[u8; 32],
        protocol_version: u8,
    ) -> Result<Self, LpError> {
        // noiserm
        // if we're LP responder, we **must** set our kem key
        // georgio: if the initiator is a client, this is ok. but if it's a node then this will block mutual kkt.
        if !is_initiator && local_peer.kem_psq.is_none() {
            return Err(LpError::ResponderWithMissingKEMKey);
        }

        todo!()
        //
        // // XKpsk3 pattern requires remote static key known upfront (XK)
        // // and PSK mixed at position 3. This provides forward secrecy with PSK authentication.
        // let pattern_name = crate::NOISE_PATTERN;
        // let psk_index = crate::NOISE_PSK_INDEX;
        //
        // // noiserm
        // let params = pattern_name.parse()?;
        // let builder = Builder::new(params);
        //
        // let local_key_bytes = local_peer.x25519.private_key().as_bytes();
        // // noiserm
        // let builder = builder.local_private_key(local_key_bytes);
        //
        // let remote_key_bytes = remote_peer.x25519_public.to_bytes();
        // // noiserm
        // let builder = builder.remote_public_key(&remote_key_bytes);
        //
        // // noiserm
        // // Initialize with dummy PSK - real PSK will be injected via set_psk() during handshake
        // // when PSQ runs using X25519 as DHKEM
        // let dummy_psk = [0u8; 32];
        // let builder = builder.psk(psk_index, &dummy_psk);
        //
        // // noiserm
        // let initial_state = if is_initiator {
        //     builder.build_initiator().map_err(LpError::SnowKeyError)?
        // } else {
        //     builder.build_responder().map_err(LpError::SnowKeyError)?
        // };
        //
        // // noiserm
        // let noise_protocol = NoiseProtocol::new(initial_state);
        //
        // // Initialize KKT state - both roles start at NotStarted
        // let kkt_state = KKTState::NotStarted;
        //
        // // Initialize PSQ state based on role
        // // georgio: why PSQState::ResponderWaiting if responder?
        // // georgio: maybe because we can start straight with PSQ?
        // // georgio: either way, doesn't matter so much because a bad PSQ request will be rejected
        // let psq_state = if is_initiator {
        //     PSQState::NotStarted
        // } else {
        //     PSQState::ResponderWaiting
        // };
        //
        // Ok(Self {
        //     id,
        //     is_initiator,
        //     // noiserm
        //     noise_state: Mutex::new(noise_protocol),
        //     kkt_state: Mutex::new(kkt_state),
        //     psq_state: Mutex::new(psq_state),
        //     psk_handle: Mutex::new(None),
        //     sending_counter: AtomicU64::new(0),
        //     receiving_counter: Mutex::new(ReceivingKeyCounterValidator::default()),
        //     // noiserm
        //     psk_injected: AtomicBool::new(false),
        //     local_peer,
        //     remote_peer,
        //     salt: *salt,
        //     outer_aead_key: Mutex::new(None),
        //     pq_shared_secret: Mutex::new(None),
        //     subsession_counter: AtomicU64::new(0),
        //     read_only: AtomicBool::new(false),
        //     successor_session_id: Mutex::new(None),
        //     negotiated_version: protocol_version,
        // })
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

    /// Returns the PQ shared secret (K_pq) if available.
    ///
    /// This is the raw KEM output from PSQ before Blake3 KDF combination.
    /// Used for deriving subsession PSKs to maintain PQ protection.
    pub fn pq_shared_secret(&self) -> Option<[u8; 32]> {
        todo!()
        // self.pq_shared_secret.lock().as_ref().map(|s| *s.as_bytes())
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

    /// Test-only method to set KKT state to Completed with a mock KEM key.
    /// This allows tests to bypass KKT exchange and directly test PSQ handshake.
    #[cfg(test)]
    pub(crate) fn set_kkt_completed_for_test(&self, remote_x25519_pub: &x25519::PublicKey) {
        todo!()
        // // Convert remote X25519 public key to EncapsulationKey for testing
        // let remote_kem_bytes = remote_x25519_pub.as_bytes();
        // let libcrux_public_key =
        //     libcrux_kem::PublicKey::decode(libcrux_kem::Algorithm::X25519, remote_kem_bytes)
        //         .expect("Test KEM key conversion failed");
        // let kem_pk = EncapsulationKey::X25519(libcrux_public_key);
        //
        // let mut kkt_state = self.kkt_state.lock();
        // *kkt_state = KKTState::Completed {
        //     kem_pk: Box::new(kem_pk),
        // };
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
        let pq_secret = self
            .pq_shared_secret()
            .ok_or_else(|| LpError::Internal("PQ shared secret not available".into()))?;

        // Derive subsession PSK from parent's PQ shared secret
        let subsession_psk = derive_subsession_psk(&pq_secret, subsession_index);

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
            pq_shared_secret: PqSharedSecret::new(pq_secret),
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

        todo!()
        //
        // // Extract the noise state (now in transport mode)
        // let noise_state = self.noise_state.into_inner();
        //
        // // Generate fresh salt for the new session
        // let salt = generate_fresh_salt();
        //
        // // Derive outer AEAD key from the subsession PSK
        // let outer_key = OuterAeadKey::from_psk(&self.subsession_psk);
        //
        // Ok(LpSession {
        //     id: receiver_index,
        //     is_initiator: self.is_initiator,
        //     // noiserm
        //     noise_state: Mutex::new(noise_state),
        //     // KKT: subsession inherits from parent, mark as processed
        //     kkt_state: Mutex::new(KKTState::ResponderProcessed),
        //     // PSQ: subsession uses PSK derived from parent's PQ secret
        //     psq_state: Mutex::new(PSQState::Completed {
        //         psk: self.subsession_psk,
        //     }),
        //     psk_handle: Mutex::new(None), // Subsession doesn't have its own handle
        //     sending_counter: AtomicU64::new(0),
        //     receiving_counter: Mutex::new(ReceivingKeyCounterValidator::new(0)),
        //     // noiserm
        //     psk_injected: AtomicBool::new(true), // PSK was in KKpsk0
        //     local_peer: self.local_peer,
        //     remote_peer: self.remote_peer,
        //     salt,
        //     outer_aead_key: Mutex::new(Some(outer_key)),
        //     pq_shared_secret: Mutex::new(Some(self.pq_shared_secret)),
        //     subsession_counter: AtomicU64::new(0),
        //     read_only: AtomicBool::new(false),
        //     successor_session_id: Mutex::new(None),
        //     // Inherit parent's protocol version
        //     negotiated_version: self.negotiated_version,
        // })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::version;
    use crate::peer::mock_peers;
    use crate::{replay::ReplayError, sessions_for_tests};
    use nym_crypto::asymmetric::ed25519;
    use rand::thread_rng;

    // Helper function to generate keypairs for tests
    fn generate_x25519_keypair() -> x25519::KeyPair {
        x25519::KeyPair::new(&mut thread_rng())
    }

    // Helper function to create a session with real keys for handshake tests
    fn create_handshake_test_session(receiver_index: u32, is_initiator: bool) -> LpSession {
        let (keys_1, keys_2) = mock_peers();

        // Create Ed25519 keypairs that correspond to initiator/responder roles
        // Initiator uses [1u8], Responder uses [2u8]
        let (local, remote) = if is_initiator {
            (keys_1, keys_2.as_remote())
        } else {
            (keys_2, keys_1.as_remote())
        };

        let salt = [0u8; 32]; // Test salt

        // PSQ will derive the PSK during handshake using X25519 as DHKEM
        let mut session = LpSession::new(
            receiver_index,
            is_initiator,
            local,
            remote.clone(),
            &salt,
            version::CURRENT,
        )
        .expect("Test session creation failed");

        // Initialize KKT state to Completed for tests (bypasses KKT exchange)
        // This simulates having already received the remote party's KEM key via KKT
        session.set_kkt_completed_for_test(&remote.x25519_public);

        session
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
        let mut session = create_handshake_test_session(12345u32, true);

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
        // --- Setup Handshake ---
        todo!()
        //
        // let initiator_session = create_handshake_test_session(12345u32, true);
        // let responder_session = create_handshake_test_session(12345u32, false);
        //
        // // Drive handshake to completion
        // let i_msg = initiator_session
        //     .prepare_handshake_message()
        //     .unwrap()
        //     .unwrap();
        // responder_session.process_handshake_message(&i_msg).unwrap();
        // let r_msg = responder_session
        //     .prepare_handshake_message()
        //     .unwrap()
        //     .unwrap();
        // initiator_session.process_handshake_message(&r_msg).unwrap();
        // let i_msg = initiator_session
        //     .prepare_handshake_message()
        //     .unwrap()
        //     .unwrap();
        // responder_session.process_handshake_message(&i_msg).unwrap();
        //
        // assert!(initiator_session.is_handshake_complete());
        //
        // // Encryption works before demotion
        // let plaintext = b"Hello before demotion";
        // assert!(initiator_session.encrypt_data(plaintext).is_ok());
        //
        // // Demote the session
        // initiator_session.demote(99999);
        //
        // // Encryption fails after demotion
        // let result = initiator_session.encrypt_data(plaintext);
        // assert!(result.is_err());
        // match result.unwrap_err() {
        //     NoiseError::SessionReadOnly => {
        //         // Expected
        //     }
        //     e => panic!("Expected SessionReadOnly error, got: {:?}", e),
        // }
    }

    #[test]
    fn test_decrypt_works_after_demotion() {
        // --- Setup Handshake ---
        todo!()
        // let initiator_session = create_handshake_test_session(12345u32, true);
        // let responder_session = create_handshake_test_session(12345u32, false);
        //
        // // Drive handshake to completion
        // let i_msg = initiator_session
        //     .prepare_handshake_message()
        //     .unwrap()
        //     .unwrap();
        // responder_session.process_handshake_message(&i_msg).unwrap();
        // let r_msg = responder_session
        //     .prepare_handshake_message()
        //     .unwrap()
        //     .unwrap();
        // initiator_session.process_handshake_message(&r_msg).unwrap();
        // let i_msg = initiator_session
        //     .prepare_handshake_message()
        //     .unwrap()
        //     .unwrap();
        // responder_session.process_handshake_message(&i_msg).unwrap();
        //
        // assert!(initiator_session.is_handshake_complete());
        // assert!(responder_session.is_handshake_complete());
        //
        // // Responder encrypts a message
        // let plaintext = b"Message to demoted initiator";
        // let ciphertext = responder_session
        //     .encrypt_data(plaintext)
        //     .expect("Encryption failed");
        //
        // // Demote the initiator session
        // initiator_session.demote(99999);
        // assert!(initiator_session.is_read_only());
        //
        // // Decryption still works on demoted session (drain in-flight)
        // let decrypted = initiator_session
        //     .decrypt_data(&ciphertext)
        //     .expect("Decryption should work on demoted session");
        // assert_eq!(decrypted, plaintext);
    }
}
