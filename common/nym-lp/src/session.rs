// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Session management for the Lewes Protocol.
//!
//! This module implements session management functionality, including replay protection
//! and Noise protocol state handling.

use crate::codec::OuterAeadKey;
use crate::keypair::{PrivateKey, PublicKey};
use crate::message::{EncryptedDataPayload, HandshakeData};
use crate::noise_protocol::{NoiseError, NoiseProtocol, ReadResult};
use crate::packet::LpHeader;
use crate::psk::{
    derive_subsession_psk, psq_initiator_create_message, psq_responder_process_message,
};
use crate::replay::ReceivingKeyCounterValidator;
use crate::{LpError, LpMessage, LpPacket};
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_kkt::ciphersuite::{DecapsulationKey, EncapsulationKey};
use nym_kkt::encryption::KKTSessionSecret;
use nym_kkt::kkt::decrypt_kkt_response_frame;
use parking_lot::Mutex;
use snow::Builder;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
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

/// KKT (KEM Key Transfer) exchange state.
///
/// Tracks the KKT protocol for obtaining the responder's KEM public key
/// before PSQ can begin. This allows post-quantum KEM algorithms to be
/// used even when keys are not pre-published.
///
/// # State Transitions
///
/// **Initiator path:**
/// ```text
/// NotStarted → InitiatorWaiting → Completed
/// ```
///
/// **Responder path:**
/// ```text
/// NotStarted → ResponderProcessed
/// ```
pub enum KKTState {
    /// KKT exchange not started.
    NotStarted,

    /// Initiator sent KKT request and is waiting for responder's KEM key.
    InitiatorWaiting {
        /// KKT context for verifying the response
        context: nym_kkt::context::KKTContext,
        session_secret: KKTSessionSecret,
    },

    /// KKT exchange completed (initiator received and validated KEM key).
    Completed {
        /// Responder's KEM public key for PSQ encapsulation
        kem_pk: Box<EncapsulationKey<'static>>,
    },

    /// Responder processed a KKT request and sent response.
    /// Responder uses their own KEM keypair, not the one from KKT.
    ResponderProcessed,
}

impl std::fmt::Debug for KKTState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotStarted => write!(f, "KKTState::NotStarted"),
            Self::InitiatorWaiting { context, .. } => f
                .debug_struct("KKTState::InitiatorWaiting")
                .field("context", context)
                .finish(),
            Self::Completed { .. } => write!(f, "KKTState::Completed {{ kem_pk: <opaque> }}"),
            Self::ResponderProcessed => write!(f, "KKTState::ResponderProcessed"),
        }
    }
}

/// PSQ (Post-Quantum Secure PSK) handshake state.
///
/// Tracks the PSQ protocol state machine through the session lifecycle.
///
/// # State Transitions
///
/// **Initiator path:**
/// ```text
/// NotStarted → InitiatorWaiting → Completed
/// ```
///
/// **Responder path:**
/// ```text
/// NotStarted → ResponderWaiting → Completed
/// ```
#[derive(Debug)]
pub enum PSQState {
    /// PSQ handshake not yet started.
    NotStarted,

    /// Initiator has sent PSQ ciphertext and is waiting for confirmation.
    /// PSK is already derived but we don't encrypt outgoing packets yet
    /// because the responder may not have processed our message yet.
    InitiatorWaiting {
        /// The derived PSK, stored until we transition to Completed
        psk: [u8; 32],
    },

    /// Responder is ready to receive and decapsulate PSQ ciphertext.
    ResponderWaiting,

    /// PSQ handshake completed successfully.
    /// The PSK has been derived and registered with the Noise protocol.
    Completed {
        /// The derived post-quantum PSK
        psk: [u8; 32],
    },
}

/// A session in the Lewes Protocol, handling connection state with Noise.
///
/// Sessions manage connection state, including LP replay protection and Noise cryptography.
/// Each session has a unique receiving index and sending index for connection identification.
///
/// ## PSK Injection Lifecycle
///
/// 1. Session created with dummy PSK `[0u8; 32]` in Noise HandshakeState
/// 2. During handshake, PSQ runs and derives real post-quantum PSK
/// 3. Real PSK injected via `set_psk()` - `psk_injected` flag set to `true`
/// 4. Handshake completes, transport mode available
/// 5. Transport operations (`encrypt_data`/`decrypt_data`) check `psk_injected` flag for safety
#[derive(Debug)]
pub struct LpSession {
    id: u32,

    /// Flag indicating if this session acts as the Noise protocol initiator.
    is_initiator: bool,

    /// Noise protocol state machine
    noise_state: Mutex<NoiseProtocol>,

    /// KKT (KEM Key Transfer) exchange state
    kkt_state: Mutex<KKTState>,

    /// PSQ (Post-Quantum Secure PSK) handshake state
    psq_state: Mutex<PSQState>,

    /// PSK handle from responder (ctxt_B) for future re-registration
    psk_handle: Mutex<Option<Vec<u8>>>,

    /// Counter for outgoing packets
    sending_counter: AtomicU64,

    /// Validator for incoming packet counters to prevent replay attacks
    receiving_counter: Mutex<ReceivingKeyCounterValidator>,

    /// Safety flag: `true` if real PSK was injected via PSQ, `false` if still using dummy PSK.
    /// This prevents transport mode operations from running with the insecure dummy `[0u8; 32]` PSK.
    psk_injected: AtomicBool,

    // PSQ-related keys stored for handshake
    /// Local Ed25519 keys for PSQ authentication
    local_ed25519: Arc<ed25519::KeyPair>,

    /// Remote Ed25519 public key for PSQ authentication
    remote_ed25519_public: ed25519::PublicKey,

    /// Local x25519 keys (Noise static key)
    local_x25519: Arc<x25519::KeyPair>,

    /// Remote X25519 public key (Noise static key)
    remote_x25519_public: PublicKey,

    /// Salt for PSK derivation
    salt: [u8; 32],

    /// Outer AEAD key for packet encryption (derived from PSK after PSQ handshake).
    /// None before PSK is available, Some after PSK injection.
    outer_aead_key: Mutex<Option<OuterAeadKey>>,

    /// Raw PQ shared secret (K_pq) from PSQ KEM encapsulation/decapsulation.
    /// Stored after PSQ handshake completes for subsession PSK derivation.
    /// This preserves PQ protection when creating subsessions via KKpsk0.
    /// Wrapped in PqSharedSecret for automatic memory zeroization on drop.
    pq_shared_secret: Mutex<Option<PqSharedSecret>>,

    /// Monotonically increasing counter for subsession indices.
    /// Each subsession gets a unique index to ensure unique PSK derivation.
    /// Uses u64 to make overflow practically impossible (~585k years at 1M/sec).
    subsession_counter: AtomicU64,

    /// True if this session has been demoted to read-only mode.
    /// Demoted sessions can still receive/decrypt but cannot send/encrypt.
    read_only: AtomicBool,

    /// ID of the successor session that replaced this one.
    /// Set when demote() is called.
    successor_session_id: Mutex<Option<u32>>,

    /// Negotiated protocol version from handshake.
    /// Set during handshake completion from the ClientHello/ServerHello packet header.
    /// Used for future version negotiation and compatibility checks.
    negotiated_version: std::sync::atomic::AtomicU8,
}

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
    use rand::RngCore;
    use std::time::{SystemTime, UNIX_EPOCH};

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
    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn noise_state(&self) -> &Mutex<NoiseProtocol> {
        &self.noise_state
    }

    /// Returns true if this session was created as the initiator.
    pub fn is_initiator(&self) -> bool {
        self.is_initiator
    }

    /// Returns the negotiated protocol version from the handshake.
    ///
    /// Defaults to 1 (current LP version). Set during handshake via
    /// `set_negotiated_version()` when ClientHello/ServerHello is processed.
    pub fn negotiated_version(&self) -> u8 {
        self.negotiated_version
            .load(std::sync::atomic::Ordering::Acquire)
    }

    /// Sets the negotiated protocol version from handshake packet header.
    ///
    /// Should be called during handshake when processing ClientHello (responder)
    /// or ServerHello (initiator) to record the agreed protocol version.
    pub fn set_negotiated_version(&self, version: u8) {
        self.negotiated_version
            .store(version, std::sync::atomic::Ordering::Release);
    }

    /// Returns the local X25519 public key.
    ///
    /// This is used for KKT protocol when the responder needs to send their
    /// KEM public key in the KKT response.
    pub fn local_x25519_public(&self) -> PublicKey {
        *self.local_x25519.public_key()
    }

    /// Returns the remote X25519 public key.
    ///
    /// Used for tie-breaking in simultaneous subsession initiation.
    /// Lower key loses and becomes responder.
    pub fn remote_x25519_public(&self) -> &PublicKey {
        &self.remote_x25519_public
    }

    /// Returns the outer AEAD key for packet encryption/decryption.
    ///
    /// Returns `None` before PSK is derived (during initial handshake),
    /// `Some(&OuterAeadKey)` after PSK injection via PSQ.
    ///
    /// Callers should use `None` for packet encryption/decryption during
    /// the handshake phase, and use the returned key for transport phase.
    ///
    /// Note: For sending packets during handshake, use `outer_aead_key_for_sending()`
    /// which checks PSQ state to avoid encrypting before the responder can decrypt.
    pub fn outer_aead_key(&self) -> Option<OuterAeadKey> {
        self.outer_aead_key.lock().clone()
    }

    /// Returns the outer AEAD key only if it's safe to use for sending.
    ///
    /// This method gates the key based on PSQ handshake state:
    /// - Returns `None` if PSQ is NotStarted, InitiatorWaiting, or ResponderWaiting
    /// - Returns `Some(key)` only if PSQ is Completed
    ///
    /// # Why This Matters
    ///
    /// The first Noise handshake message (containing PSQ payload from initiator)
    /// must be sent in cleartext because the responder hasn't derived the PSK yet.
    /// Only after the responder processes the PSQ and both sides have the PSK
    /// can outer encryption be used for sending.
    ///
    /// For receiving, use `outer_aead_key()` which returns the key as soon as
    /// it's derived (needed because the peer may start encrypting before we've
    /// finished our send).
    // This fixes a bug where the initiator encrypted the first Noise
    // message with outer AEAD, but the responder couldn't decrypt because it
    // hadn't processed the PSQ yet to derive the same PSK.
    pub fn outer_aead_key_for_sending(&self) -> Option<OuterAeadKey> {
        let psq_state = self.psq_state.lock();
        match &*psq_state {
            PSQState::Completed { .. } => self.outer_aead_key.lock().clone(),
            _ => None,
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
    /// * `local_ed25519_keypair` - This side's Ed25519 keypair for PSQ authentication
    /// * `local_x25519_key` - This side's X25519 private key for Noise protocol and DHKEM
    /// * `remote_ed25519_key` - Peer's Ed25519 public key for PSQ authentication
    /// * `remote_x25519_key` - Peer's X25519 public key for Noise protocol and DHKEM
    /// * `salt` - Salt for PSK derivation
    pub fn new(
        id: u32,
        is_initiator: bool,
        local_ed25519_keypair: Arc<ed25519::KeyPair>,
        local_x25519_key: &PrivateKey,
        remote_ed25519_key: &ed25519::PublicKey,
        remote_x25519_key: &PublicKey,
        salt: &[u8; 32],
    ) -> Result<Self, LpError> {
        // XKpsk3 pattern requires remote static key known upfront (XK)
        // and PSK mixed at position 3. This provides forward secrecy with PSK authentication.
        let pattern_name = "Noise_XKpsk3_25519_ChaChaPoly_SHA256";
        let psk_index = 3;

        let params = pattern_name.parse()?;
        let builder = Builder::new(params);

        let local_key_bytes = local_x25519_key.to_bytes();
        let builder = builder.local_private_key(&local_key_bytes);

        let remote_key_bytes = remote_x25519_key.to_bytes();
        let builder = builder.remote_public_key(&remote_key_bytes);

        // Initialize with dummy PSK - real PSK will be injected via set_psk() during handshake
        // when PSQ runs using X25519 as DHKEM
        let dummy_psk = [0u8; 32];
        let builder = builder.psk(psk_index, &dummy_psk);

        let initial_state = if is_initiator {
            builder.build_initiator().map_err(LpError::SnowKeyError)?
        } else {
            builder.build_responder().map_err(LpError::SnowKeyError)?
        };

        let noise_protocol = NoiseProtocol::new(initial_state);

        // Initialize KKT state - both roles start at NotStarted
        let kkt_state = KKTState::NotStarted;

        // Initialize PSQ state based on role
        let psq_state = if is_initiator {
            PSQState::NotStarted
        } else {
            PSQState::ResponderWaiting
        };

        Ok(Self {
            id,
            is_initiator,
            noise_state: Mutex::new(noise_protocol),
            kkt_state: Mutex::new(kkt_state),
            psq_state: Mutex::new(psq_state),
            psk_handle: Mutex::new(None),
            sending_counter: AtomicU64::new(0),
            receiving_counter: Mutex::new(ReceivingKeyCounterValidator::default()),
            psk_injected: AtomicBool::new(false),
            local_ed25519: local_ed25519_keypair.clone(),
            remote_ed25519_public: *remote_ed25519_key,
            local_x25519: Arc::new(
                x25519::PrivateKey::from_bytes(&local_x25519_key.to_bytes())
                    .expect("Valid x25519 private key")
                    .into(),
            ),
            remote_x25519_public: remote_x25519_key.clone(),
            salt: *salt,
            outer_aead_key: Mutex::new(None),
            pq_shared_secret: Mutex::new(None),
            subsession_counter: AtomicU64::new(0),
            read_only: AtomicBool::new(false),
            successor_session_id: Mutex::new(None),
            negotiated_version: std::sync::atomic::AtomicU8::new(1), // Default to version 1
        })
    }

    pub fn next_packet(&self, message: LpMessage) -> Result<LpPacket, LpError> {
        let counter = self.next_counter();
        let header = LpHeader::new(self.id(), counter);
        let packet = LpPacket::new(header, message);
        Ok(packet)
    }

    /// Generates the next counter value for outgoing packets.
    pub fn next_counter(&self) -> u64 {
        self.sending_counter.fetch_add(1, Ordering::Relaxed)
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
        let counter_validator = self.receiving_counter.lock();
        counter_validator
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
    pub fn receiving_counter_mark(&self, counter: u64) -> Result<(), LpError> {
        let mut counter_validator = self.receiving_counter.lock();
        counter_validator
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
        let counter_validator = self.receiving_counter.lock();
        counter_validator.current_packet_cnt()
    }

    /// Returns the stored PSK handle (ctxt_B) if available.
    ///
    /// The PSK handle is received from the responder during handshake and can be
    /// used for future PSK re-registration without running KEM encapsulation again.
    ///
    /// # Returns
    ///
    /// * `Some(Vec<u8>)` - The encrypted PSK handle from the responder
    /// * `None` - PSK handle not yet received or session is initiator before handshake completion
    pub fn get_psk_handle(&self) -> Option<Vec<u8>> {
        self.psk_handle.lock().clone()
    }

    /// Prepares a KKT (KEM Key Transfer) request message.
    ///
    /// This should be called by the initiator before starting the Noise handshake
    /// to obtain the responder's KEM public key. The KKT protocol authenticates
    /// the exchange using Ed25519 signatures.
    ///
    /// **Protocol Flow:**
    /// 1. Initiator creates KKT request with Ed25519 signature
    /// 2. Responder validates signature and responds with KEM public key + signature
    /// 3. Initiator validates response and stores KEM key for PSQ
    ///
    /// # Returns
    ///
    /// * `Some(Ok(LpMessage::KKTRequest))` - KKT request ready to send
    /// * `Some(Err(LpError))` - Error creating KKT request
    /// * `None` - KKT not applicable (responder, or already completed)
    pub fn prepare_kkt_request(&self) -> Option<Result<LpMessage, LpError>> {
        use nym_kkt::{
            ciphersuite::{Ciphersuite, HashFunction, KEM, SignatureScheme},
            kkt::request_kem_key,
        };

        let mut kkt_state = self.kkt_state.lock();

        // Only initiator creates KKT requests, and only when not started
        if !self.is_initiator || !matches!(*kkt_state, KKTState::NotStarted) {
            return None;
        }

        // Use X25519 as KEM for now (can extend to ML-KEM-768 later)
        let ciphersuite = match Ciphersuite::resolve_ciphersuite(
            KEM::X25519,
            HashFunction::Blake3,
            SignatureScheme::Ed25519,
            None,
        ) {
            Ok(cs) => cs,
            Err(e) => {
                return Some(Err(LpError::Internal(format!(
                    "KKT ciphersuite error: {:?}",
                    e
                ))));
            }
        };

        let mut rng = rand09::rng();
        match request_kem_key(
            &mut rng,
            ciphersuite,
            self.local_ed25519.private_key(),
            &self.remote_x25519_public,
        ) {
            Ok((session_secret, context, request_bytes)) => {
                // Store context for response validation
                *kkt_state = KKTState::InitiatorWaiting {
                    context,
                    session_secret,
                };

                // Serialize KKT frame to bytes
                Some(Ok(LpMessage::KKTRequest(crate::message::KKTRequestData(
                    request_bytes,
                ))))
            }
            Err(e) => Some(Err(LpError::Internal(format!(
                "KKT request creation failed: {:?}",
                e
            )))),
        }
    }

    /// Processes a KKT response from the responder.
    ///
    /// Validates the responder's signature and stores the authenticated KEM public key
    /// for use in PSQ encapsulation.
    ///
    /// # Arguments
    ///
    /// * `response_bytes` - Raw KKT response message from responder
    /// * `expected_key_hash` - Optional expected hash of responder's KEM key.
    ///   - `Some(hash)`: Full KKT validation (signature + hash) - use when directory service available
    ///   - `None`: Signature-only validation (hash computed from received key) - temporary mode
    ///
    /// # Returns
    ///
    /// * `Ok(())` - KKT exchange completed, KEM key stored
    /// * `Err(LpError)` - Signature verification failed, hash mismatch, or invalid state
    ///
    /// # Note
    ///
    /// When None is passed, the function computes the hash from the received key and validates against
    /// that (effectively signature-only mode). This allows easy upgrade: just pass Some(directory_hash)
    /// when directory service becomes available. The full KKT protocol with hash pinning provides
    /// protection against key substitution attacks.
    pub fn process_kkt_response(
        &self,
        response_bytes: &[u8],
        expected_key_hash: Option<&[u8]>,
    ) -> Result<(), LpError> {
        use nym_kkt::key_utils::hash_encapsulation_key;
        use nym_kkt::kkt::validate_kem_response;

        let mut kkt_state = self.kkt_state.lock();

        // Extract context from waiting state
        let (mut context, session_secret) = match &*kkt_state {
            KKTState::InitiatorWaiting {
                context,
                session_secret,
            } => (*context, *session_secret),
            _ => {
                return Err(LpError::Internal(
                    "KKT response received in invalid state".to_string(),
                ));
            }
        };

        // Determine hash to validate against
        let hash_for_validation: Vec<u8>;
        let hash_ref = match expected_key_hash {
            Some(hash) => hash,
            None => {
                // Signature-only mode: extract key from response and compute its hash
                // This effectively bypasses hash validation while keeping signature validation
                let (frame, _) = decrypt_kkt_response_frame(&session_secret, response_bytes)
                    .map_err(|e| {
                        LpError::Internal(format!("Failed to decrypt KKT response: {:?}", e))
                    })?;

                hash_for_validation = hash_encapsulation_key(
                    &context.ciphersuite().hash_function(),
                    context.ciphersuite().hash_len(),
                    frame.body_ref(),
                );
                &hash_for_validation
            }
        };

        // Validate response and extract KEM key
        let kem_pk = validate_kem_response(
            &mut context,
            &session_secret,
            &self.remote_ed25519_public,
            hash_ref,
            response_bytes,
        )
        .map_err(|e| LpError::Internal(format!("KKT response validation failed: {:?}", e)))?;

        // Store the authenticated KEM key
        *kkt_state = KKTState::Completed {
            kem_pk: Box::new(kem_pk),
        };

        Ok(())
    }

    /// Processes a KKT request from the initiator and prepares a signed response.
    ///
    /// Validates the initiator's signature and creates a response containing this
    /// responder's KEM public key, signed with Ed25519.
    ///
    /// # Arguments
    ///
    /// * `request_bytes` - Raw KKT request message from initiator
    /// * `responder_kem_pk` - This responder's KEM public key to send
    ///
    /// # Returns
    ///
    /// * `Ok(LpMessage::KKTResponse)` - Signed KKT response ready to send
    /// * `Err(LpError)` - Signature verification failed or invalid request
    pub fn process_kkt_request(
        &self,
        request_bytes: &[u8],
        responder_kem_pk: &EncapsulationKey,
    ) -> Result<LpMessage, LpError> {
        use nym_kkt::kkt::handle_kem_request;

        let mut rng = rand09::rng();

        let mut kkt_state = self.kkt_state.lock();

        // Handle request and create signed response
        let response_bytes = handle_kem_request(
            &mut rng,
            request_bytes,
            Some(&self.remote_ed25519_public), // Verify initiator signature
            &self.local_ed25519.private_key(), // Sign response
            &self.local_x25519.private_key(),
            responder_kem_pk,
        )
        .map_err(|e| LpError::Internal(format!("KKT request handling failed: {:?}", e)))?;

        // Mark KKT as processed
        // Responder doesn't store the kem_pk since they already have their own KEM keypair
        *kkt_state = KKTState::ResponderProcessed;

        Ok(LpMessage::KKTResponse(crate::message::KKTResponseData(
            response_bytes,
        )))
    }

    /// Prepares the next handshake message to be sent, if any.
    ///
    /// This should be called by the driver/IO layer to check if the Noise protocol
    /// state machine requires a message to be sent to the peer.
    ///
    /// For initiators, PSQ always runs on the first message:
    /// 1. Converts X25519 keys to DHKEM format
    /// 2. Generates PSQ payload and derives PSK
    /// 3. Injects PSK into Noise HandshakeState
    /// 4. Embeds PSQ payload in first handshake message as: [u16 len][psq_payload][noise_msg]
    ///
    /// # Returns
    ///
    /// * `None` if no message needs to be sent currently (e.g., waiting for peer, or handshake complete).
    /// * `Some(LpError)` if there's an error within the Noise protocol or PSQ.
    pub fn prepare_handshake_message(&self) -> Option<Result<LpMessage, LpError>> {
        let mut noise_state = self.noise_state.lock();

        // PSQ always runs for initiator on first message
        let mut psq_state = self.psq_state.lock();

        if self.is_initiator && matches!(*psq_state, PSQState::NotStarted) {
            // Extract KEM public key from completed KKT exchange
            // PSQ requires the authenticated KEM key obtained via KKT protocol
            let kkt_state = self.kkt_state.lock();
            let remote_kem = match &*kkt_state {
                KKTState::Completed { kem_pk } => kem_pk,
                _ => {
                    return Some(Err(LpError::KKTError(
                        "PSQ handshake requires completed KKT exchange".to_string(),
                    )));
                }
            };

            // Generate PSQ payload and PSK using KKT-authenticated KEM key
            let session_context = self.id.to_le_bytes();

            let psq_result = match psq_initiator_create_message(
                &self.local_x25519.private_key(),
                &self.remote_x25519_public,
                remote_kem,
                &self.local_ed25519.private_key(),
                &self.local_ed25519.public_key(),
                &self.salt,
                &session_context,
            ) {
                Ok(result) => result,
                Err(e) => {
                    tracing::error!("PSQ handshake preparation failed, aborting: {:?}", e);
                    return Some(Err(e));
                }
            };
            let psk = psq_result.psk;
            let psq_payload = psq_result.payload;

            // Store PQ shared secret for subsession PSK derivation
            *self.pq_shared_secret.lock() = Some(PqSharedSecret::new(psq_result.pq_shared_secret));

            // Inject PSK into Noise HandshakeState
            if let Err(e) = noise_state.set_psk(3, &psk) {
                return Some(Err(LpError::NoiseError(e)));
            }
            // Mark PSK as injected for safety checks in transport mode
            self.psk_injected.store(true, Ordering::Release);

            // Derive and store outer AEAD key from PSK
            {
                let mut outer_key = self.outer_aead_key.lock();
                *outer_key = Some(OuterAeadKey::from_psk(&psk));
            }

            // Get the Noise handshake message
            let noise_msg = match noise_state.get_bytes_to_send() {
                Some(Ok(msg)) => msg,
                Some(Err(e)) => return Some(Err(LpError::NoiseError(e))),
                None => return None, // Should not happen if is_my_turn, but handle gracefully
            };

            // Combine: [u16 psq_len][psq_payload][noise_msg]
            let psq_len = psq_payload.len() as u16;
            let mut combined = Vec::with_capacity(2 + psq_payload.len() + noise_msg.len());
            combined.extend_from_slice(&psq_len.to_le_bytes());
            combined.extend_from_slice(&psq_payload);
            combined.extend_from_slice(&noise_msg);

            // PSK is derived but we stay in InitiatorWaiting until we receive msg 2.
            // This ensures we send msg 1 in cleartext (responder can't decrypt yet).
            *psq_state = PSQState::InitiatorWaiting { psk };

            return Some(Ok(LpMessage::Handshake(HandshakeData(combined))));
        }

        // Normal flow (no PSQ, or PSQ already completed)
        drop(psq_state); // Release lock

        if let Some(message) = noise_state.get_bytes_to_send() {
            match message {
                Ok(noise_msg) => {
                    // Check if we have a PSK handle (ctxt_B) to embed (responder message 2 only)
                    // Only the responder should embed the handle, never the initiator
                    if !self.is_initiator {
                        let mut psk_handle_guard = self.psk_handle.lock();
                        if let Some(handle_bytes) = psk_handle_guard.take() {
                            // Embed PSK handle in message: [u16 handle_len][handle_bytes][noise_msg]
                            let handle_len = handle_bytes.len() as u16;
                            let mut combined =
                                Vec::with_capacity(2 + handle_bytes.len() + noise_msg.len());
                            combined.extend_from_slice(&handle_len.to_le_bytes());
                            combined.extend_from_slice(&handle_bytes);
                            combined.extend_from_slice(&noise_msg);

                            tracing::debug!(
                                "Embedding PSK handle ({} bytes) in handshake message 2",
                                handle_bytes.len()
                            );

                            return Some(Ok(LpMessage::Handshake(HandshakeData(combined))));
                        }
                    }
                    // No PSK handle to embed, send noise message as-is
                    Some(Ok(LpMessage::Handshake(HandshakeData(noise_msg))))
                }
                Err(e) => Some(Err(LpError::NoiseError(e))),
            }
        } else {
            None
        }
    }

    /// Processes a received handshake message from the peer.
    ///
    /// This should be called by the driver/IO layer after receiving a potential
    /// handshake message payload from an LP packet.
    ///
    /// For responders, PSQ always runs on the first message:
    /// 1. Extracts PSQ payload from the first handshake message: [u16 len][psq_payload][noise_msg]
    /// 2. Converts X25519 keys to DHKEM format
    /// 3. Decapsulates PSK from PSQ payload
    /// 4. Injects PSK into Noise HandshakeState
    /// 5. Processes the remaining Noise handshake message
    ///
    /// # Arguments
    ///
    /// * `message` - The LP message received from the peer, expected to be a Handshake message.
    ///
    /// # Returns
    ///
    /// * `Ok(ReadResult)` detailing the outcome (e.g., handshake complete, no-op).
    /// * `Err(LpError)` if the message is invalid or causes a Noise/PSQ protocol error.
    pub fn process_handshake_message(&self, message: &LpMessage) -> Result<ReadResult, LpError> {
        let mut noise_state = self.noise_state.lock();
        let mut psq_state = self.psq_state.lock();

        match message {
            LpMessage::Handshake(HandshakeData(payload)) => {
                // PSQ always runs for responder on first message
                if !self.is_initiator && matches!(*psq_state, PSQState::ResponderWaiting) {
                    // Extract PSQ payload: [u16 psq_len][psq_payload][noise_msg]
                    if payload.len() < 2 {
                        return Err(LpError::NoiseError(NoiseError::Other(
                            "Payload too short for PSQ extraction".to_string(),
                        )));
                    }

                    let psq_len = u16::from_le_bytes([payload[0], payload[1]]) as usize;

                    if payload.len() < 2 + psq_len {
                        return Err(LpError::NoiseError(NoiseError::Other(
                            "Payload length mismatch for PSQ extraction".to_string(),
                        )));
                    }

                    let psq_payload = &payload[2..2 + psq_len];
                    let noise_payload = &payload[2 + psq_len..];

                    // Convert X25519 local keys to DecapsulationKey/EncapsulationKey (DHKEM)
                    let local_private_bytes = &self.local_x25519.private_key().to_bytes();
                    let libcrux_private_key = libcrux_kem::PrivateKey::decode(
                        libcrux_kem::Algorithm::X25519,
                        local_private_bytes,
                    )
                    .map_err(|e| {
                        LpError::KKTError(format!(
                            "Failed to convert X25519 private key to libcrux PrivateKey: {:?}",
                            e
                        ))
                    })?;
                    let dec_key = DecapsulationKey::X25519(libcrux_private_key);

                    let local_public_key = self.local_x25519_public();
                    let local_public_bytes = local_public_key.as_bytes();
                    let libcrux_public_key = libcrux_kem::PublicKey::decode(
                        libcrux_kem::Algorithm::X25519,
                        local_public_bytes,
                    )
                    .map_err(|e| {
                        LpError::KKTError(format!(
                            "Failed to convert X25519 public key to libcrux PublicKey: {:?}",
                            e
                        ))
                    })?;
                    let enc_key = EncapsulationKey::X25519(libcrux_public_key);

                    // Decapsulate PSK from PSQ payload using X25519 as DHKEM
                    let session_context = self.id.to_le_bytes();

                    let psq_result = match psq_responder_process_message(
                        &self.local_x25519.private_key(),
                        &self.remote_x25519_public,
                        (&dec_key, &enc_key),
                        &self.remote_ed25519_public,
                        psq_payload,
                        &self.salt,
                        &session_context,
                    ) {
                        Ok(result) => result,
                        Err(e) => {
                            tracing::error!("PSQ handshake processing failed, aborting: {:?}", e);
                            return Err(e);
                        }
                    };
                    let psk = psq_result.psk;

                    // Store PQ shared secret for subsession PSK derivation
                    *self.pq_shared_secret.lock() =
                        Some(PqSharedSecret::new(psq_result.pq_shared_secret));

                    // Store the PSK handle (ctxt_B) for transmission in next message
                    {
                        let mut psk_handle = self.psk_handle.lock();
                        *psk_handle = Some(psq_result.psk_handle);
                    }

                    // Inject PSK into Noise HandshakeState
                    noise_state.set_psk(3, &psk)?;
                    // Mark PSK as injected for safety checks in transport mode
                    self.psk_injected.store(true, Ordering::Release);

                    // Derive and store outer AEAD key from PSK
                    {
                        let mut outer_key = self.outer_aead_key.lock();
                        *outer_key = Some(OuterAeadKey::from_psk(&psk));
                    }

                    // Update PSQ state to Completed
                    *psq_state = PSQState::Completed { psk };

                    // Process the Noise handshake message (without PSQ prefix)
                    drop(psq_state); // Release lock before processing
                    return noise_state
                        .read_message(noise_payload)
                        .map_err(LpError::NoiseError);
                }

                // Check if initiator should extract PSK handle from message 2
                if let PSQState::InitiatorWaiting { psk } = *psq_state
                    && self.is_initiator
                {
                    // Extract PSK handle: [u16 handle_len][handle_bytes][noise_msg]
                    if payload.len() >= 2 {
                        let handle_len = u16::from_le_bytes([payload[0], payload[1]]) as usize;

                        if handle_len > 0 && payload.len() >= 2 + handle_len {
                            // Extract and store the PSK handle
                            let handle_bytes = &payload[2..2 + handle_len];
                            let noise_payload = &payload[2 + handle_len..];

                            tracing::debug!(
                                "Extracted PSK handle ({} bytes) from message 2",
                                handle_len
                            );

                            {
                                let mut psk_handle = self.psk_handle.lock();
                                *psk_handle = Some(handle_bytes.to_vec());
                            }

                            // Transition to Completed - we've received confirmation from responder
                            *psq_state = PSQState::Completed { psk };
                            drop(psq_state);

                            // Process only the Noise message part
                            return noise_state
                                .read_message(noise_payload)
                                .map_err(LpError::NoiseError);
                        }
                    }
                    // If no valid handle found, fall through to normal processing
                }

                // The sans-io NoiseProtocol::read_message expects only the payload.
                noise_state
                    .read_message(payload)
                    .map_err(LpError::NoiseError)
            }
            _ => Err(LpError::NoiseError(NoiseError::IncorrectStateError)),
        }
    }

    /// Checks if the Noise handshake phase is complete.
    pub fn is_handshake_complete(&self) -> bool {
        self.noise_state.lock().is_handshake_finished()
    }

    /// Returns the PQ shared secret (K_pq) if available.
    ///
    /// This is the raw KEM output from PSQ before Blake3 KDF combination.
    /// Used for deriving subsession PSKs to maintain PQ protection.
    pub fn pq_shared_secret(&self) -> Option<[u8; 32]> {
        self.pq_shared_secret.lock().as_ref().map(|s| *s.as_bytes())
    }

    /// Gets the next subsession index and increments the counter.
    ///
    /// Each subsession requires a unique index to ensure unique PSK derivation.
    /// The index is monotonically increasing per session.
    pub fn next_subsession_index(&self) -> u64 {
        self.subsession_counter.fetch_add(1, Ordering::Relaxed)
    }

    /// Returns true if this session is in read-only mode.
    ///
    /// Read-only sessions have been demoted after a subsession was promoted.
    /// They can still decrypt incoming messages but cannot encrypt outgoing ones.
    pub fn is_read_only(&self) -> bool {
        self.read_only.load(Ordering::Acquire)
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
    pub fn demote(&self, successor_idx: u32) {
        *self.successor_session_id.lock() = Some(successor_idx);
        self.read_only.store(true, Ordering::Release);
    }

    /// Returns the successor session ID if this session was demoted.
    pub fn successor_session_id(&self) -> Option<u32> {
        *self.successor_session_id.lock()
    }

    /// Encrypts application data payload using the established Noise transport session.
    ///
    /// This should only be called after the handshake is complete (`is_handshake_complete` returns true).
    ///
    /// # Arguments
    ///
    /// * `payload` - The application data to encrypt.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<u8>)` containing the encrypted Noise message ciphertext.
    /// * `Err(NoiseError)` if the session is not in transport mode or encryption fails.
    pub fn encrypt_data(&self, payload: &[u8]) -> Result<LpMessage, NoiseError> {
        // Check if session is read-only (demoted)
        if self.read_only.load(Ordering::Acquire) {
            return Err(NoiseError::SessionReadOnly);
        }

        let mut noise_state = self.noise_state.lock();
        // Safety: Prevent transport mode with dummy PSK
        if !self.psk_injected.load(Ordering::Acquire) {
            return Err(NoiseError::PskNotInjected);
        }
        // Explicitly check if handshake is finished before trying to write
        if !noise_state.is_handshake_finished() {
            return Err(NoiseError::IncorrectStateError);
        }
        let payload = noise_state.write_message(payload)?;
        Ok(LpMessage::EncryptedData(EncryptedDataPayload(payload)))
    }

    /// Decrypts an incoming Noise message containing application data.
    ///
    /// This should only be called after the handshake is complete (`is_handshake_complete` returns true)
    /// and when an `LPMessage::EncryptedData` is received.
    ///
    /// # Arguments
    ///
    /// * `noise_ciphertext` - The encrypted Noise message received from the peer.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<u8>)` containing the decrypted application data payload.
    /// * `Err(NoiseError)` if the session is not in transport mode, decryption fails, or the message is not data.
    pub fn decrypt_data(&self, noise_ciphertext: &LpMessage) -> Result<Vec<u8>, NoiseError> {
        let mut noise_state = self.noise_state.lock();
        // Safety: Prevent transport mode with dummy PSK
        if !self.psk_injected.load(Ordering::Acquire) {
            return Err(NoiseError::PskNotInjected);
        }
        // Explicitly check if handshake is finished before trying to read
        if !noise_state.is_handshake_finished() {
            return Err(NoiseError::IncorrectStateError);
        }

        let payload = noise_ciphertext.payload();

        match noise_state.read_message(payload)? {
            ReadResult::DecryptedData(data) => Ok(data),
            _ => Err(NoiseError::IncorrectStateError),
        }
    }

    /// Test-only method to set KKT state to Completed with a mock KEM key.
    /// This allows tests to bypass KKT exchange and directly test PSQ handshake.
    #[cfg(test)]
    pub(crate) fn set_kkt_completed_for_test(&self, remote_x25519_pub: &PublicKey) {
        // Convert remote X25519 public key to EncapsulationKey for testing
        let remote_kem_bytes = remote_x25519_pub.as_bytes();
        let libcrux_public_key =
            libcrux_kem::PublicKey::decode(libcrux_kem::Algorithm::X25519, remote_kem_bytes)
                .expect("Test KEM key conversion failed");
        let kem_pk = EncapsulationKey::X25519(libcrux_public_key);

        let mut kkt_state = self.kkt_state.lock();
        *kkt_state = KKTState::Completed {
            kem_pk: Box::new(kem_pk),
        };
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
        // Verify parent handshake is complete
        if !self.is_handshake_complete() {
            return Err(LpError::Internal("Parent handshake not complete".into()));
        }

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

        let local_key_bytes = self.local_x25519.private_key().to_bytes();
        let remote_key_bytes = self.remote_x25519_public.to_bytes();

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
            // Copy key material from parent for into_session() conversion
            local_ed25519: self.local_ed25519.clone(),
            remote_ed25519_public: self.remote_ed25519_public,
            remote_x25519_public: self.remote_x25519_public.clone(),
            pq_shared_secret: PqSharedSecret::new(pq_secret),
            subsession_psk,
            local_x25519: self.local_x25519.clone(),
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
    /// Local Ed25519 keys (for PSQ auth if needed)
    local_ed25519: Arc<ed25519::KeyPair>,

    /// Local x25519 keys (Noise static key)
    local_x25519: Arc<x25519::KeyPair>,

    /// Remote Ed25519 public key
    remote_ed25519_public: ed25519::PublicKey,

    /// Remote X25519 public key (Noise static key)
    remote_x25519_public: PublicKey,
    /// PQ shared secret inherited from parent (for creating further subsessions)
    pq_shared_secret: PqSharedSecret,
    /// Subsession PSK (for deriving outer AEAD key)
    subsession_psk: [u8; 32],
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

        // Generate fresh salt for the new session
        let salt = generate_fresh_salt();

        // Derive outer AEAD key from the subsession PSK
        let outer_key = OuterAeadKey::from_psk(&self.subsession_psk);

        Ok(LpSession {
            id: receiver_index,
            is_initiator: self.is_initiator,
            noise_state: Mutex::new(noise_state),
            // KKT: subsession inherits from parent, mark as processed
            kkt_state: Mutex::new(KKTState::ResponderProcessed),
            // PSQ: subsession uses PSK derived from parent's PQ secret
            psq_state: Mutex::new(PSQState::Completed {
                psk: self.subsession_psk,
            }),
            psk_handle: Mutex::new(None), // Subsession doesn't have its own handle
            sending_counter: AtomicU64::new(0),
            receiving_counter: Mutex::new(ReceivingKeyCounterValidator::new(0)),
            psk_injected: AtomicBool::new(true), // PSK was in KKpsk0
            local_ed25519: self.local_ed25519,
            remote_ed25519_public: self.remote_ed25519_public,
            local_x25519: self.local_x25519,
            remote_x25519_public: self.remote_x25519_public,
            salt,
            outer_aead_key: Mutex::new(Some(outer_key)),
            pq_shared_secret: Mutex::new(Some(self.pq_shared_secret)),
            subsession_counter: AtomicU64::new(0),
            read_only: AtomicBool::new(false),
            successor_session_id: Mutex::new(None),
            // Inherit parent's protocol version
            negotiated_version: std::sync::atomic::AtomicU8::new(1),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keypair::KeyPair;
    use crate::{replay::ReplayError, sessions_for_tests};
    use rand::thread_rng;

    // Helper function to generate keypairs for tests
    fn generate_keypair() -> KeyPair {
        KeyPair::new(&mut thread_rng())
    }

    // Helper function to create a session with real keys for handshake tests
    fn create_handshake_test_session(
        receiver_index: u32,
        is_initiator: bool,
        local_keys: &KeyPair,
        remote_pub_key: &crate::keypair::PublicKey,
    ) -> LpSession {
        use nym_crypto::asymmetric::ed25519;

        // Create Ed25519 keypairs that correspond to initiator/responder roles
        // Initiator uses [1u8], Responder uses [2u8]
        let (local_ed25519_seed, remote_ed25519_seed) = if is_initiator {
            ([1u8; 32], [2u8; 32])
        } else {
            ([2u8; 32], [1u8; 32])
        };

        let local_ed25519 = ed25519::KeyPair::from_secret(local_ed25519_seed, 0);
        let remote_ed25519 = ed25519::KeyPair::from_secret(remote_ed25519_seed, 1);

        let salt = [0u8; 32]; // Test salt

        // PSQ will derive the PSK during handshake using X25519 as DHKEM
        let session = LpSession::new(
            receiver_index,
            is_initiator,
            Arc::new(local_ed25519),
            local_keys.private_key(),
            remote_ed25519.public_key(),
            remote_pub_key,
            &salt,
        )
        .expect("Test session creation failed");

        // Initialize KKT state to Completed for tests (bypasses KKT exchange)
        // This simulates having already received the remote party's KEM key via KKT
        session.set_kkt_completed_for_test(remote_pub_key);

        session
    }

    #[test]
    fn test_session_creation() {
        let session = sessions_for_tests().0;

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
        let session = sessions_for_tests().1;

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
        let session = sessions_for_tests().1;

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
        let session = sessions_for_tests().1;

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

    #[test]
    fn test_prepare_handshake_message_initial_state() {
        let initiator_keys = generate_keypair();
        let responder_keys = generate_keypair();
        let receiver_index = 12345u32;

        let initiator_session = create_handshake_test_session(
            receiver_index,
            true,
            &initiator_keys,
            responder_keys.public_key(),
        );
        let responder_session = create_handshake_test_session(
            receiver_index,
            false,
            &responder_keys,
            initiator_keys.public_key(), // Responder also needs initiator's key for XK
        );

        // Initiator should have a message to send immediately (-> e)
        let initiator_msg_result = initiator_session.prepare_handshake_message();
        assert!(initiator_msg_result.is_some());
        let initiator_msg = initiator_msg_result
            .unwrap()
            .expect("Initiator msg prep failed");
        assert!(!initiator_msg.is_empty());

        // Responder should have nothing to send initially (waits for <- e)
        let responder_msg_result = responder_session.prepare_handshake_message();
        assert!(responder_msg_result.is_none());
    }

    #[test]
    fn test_process_handshake_message_first_step() {
        let initiator_keys = generate_keypair();
        let responder_keys = generate_keypair();
        let receiver_index = 12345u32;

        let initiator_session = create_handshake_test_session(
            receiver_index,
            true,
            &initiator_keys,
            responder_keys.public_key(),
        );
        let responder_session = create_handshake_test_session(
            receiver_index,
            false,
            &responder_keys,
            initiator_keys.public_key(),
        );

        // 1. Initiator prepares the first message (-> e)
        let initiator_msg_result = initiator_session.prepare_handshake_message();
        let initiator_msg = initiator_msg_result
            .unwrap()
            .expect("Initiator msg prep failed");

        // 2. Responder processes the message (<- e)
        let process_result = responder_session.process_handshake_message(&initiator_msg);

        // Check the result of processing
        match process_result {
            Ok(ReadResult::NoOp) => {
                // Expected for XK first message, responder doesn't decrypt data yet
            }
            Ok(other) => panic!("Unexpected process result: {:?}", other),
            Err(e) => panic!("Responder processing failed: {:?}", e),
        }

        // 3. After processing, responder should now have a message to send (-> e, es)
        let responder_response_result = responder_session.prepare_handshake_message();
        assert!(responder_response_result.is_some());
        let responder_response = responder_response_result
            .unwrap()
            .expect("Responder response prep failed");
        assert!(!responder_response.is_empty());
    }

    #[test]
    fn test_handshake_driver_simulation() {
        let initiator_keys = generate_keypair();
        let responder_keys = generate_keypair();

        let initiator_session = create_handshake_test_session(
            12345u32,
            true,
            &initiator_keys,
            responder_keys.public_key(),
        );
        let responder_session = create_handshake_test_session(
            12345u32,
            false,
            &responder_keys,
            initiator_keys.public_key(),
        );

        let mut responder_to_initiator_msg = None;
        let mut rounds = 0;
        const MAX_ROUNDS: usize = 10; // Safety break for the loop

        // Start by priming the initiator message
        let mut initiator_to_responder_msg =
            initiator_session.prepare_handshake_message().unwrap().ok();
        assert!(
            initiator_to_responder_msg.is_some(),
            "Initiator did not produce initial message"
        );

        while rounds < MAX_ROUNDS {
            rounds += 1;

            // === Initiator -> Responder ===
            if let Some(msg) = initiator_to_responder_msg.take() {
                // Process message
                match responder_session.process_handshake_message(&msg) {
                    Ok(_) => {}
                    Err(e) => panic!("Responder processing failed: {:?}", e),
                }

                // Check if responder needs to send a reply
                responder_to_initiator_msg = responder_session
                    .prepare_handshake_message()
                    .transpose()
                    .unwrap();
            }

            // Check completion after potentially processing responder's message below
            if initiator_session.is_handshake_complete()
                && responder_session.is_handshake_complete()
            {
                break;
            }

            // === Responder -> Initiator ===
            if let Some(msg) = responder_to_initiator_msg.take() {
                // Process message
                match initiator_session.process_handshake_message(&msg) {
                    Ok(_) => {}
                    Err(e) => panic!("Initiator processing failed: {:?}", e),
                }

                // Check if initiator needs to send a reply (should be last message in XK)
                initiator_to_responder_msg = initiator_session
                    .prepare_handshake_message()
                    .transpose()
                    .unwrap();
            }

            // Check completion again after potentially processing initiator's message above
            if initiator_session.is_handshake_complete()
                && responder_session.is_handshake_complete()
            {
                break;
            }
        }

        assert!(
            rounds < MAX_ROUNDS,
            "Handshake did not complete within max rounds"
        );
        assert!(
            initiator_session.is_handshake_complete(),
            "Initiator handshake did not complete"
        );
        assert!(
            responder_session.is_handshake_complete(),
            "Responder handshake did not complete"
        );

        println!("Handshake completed in {} rounds.", rounds);
    }

    #[test]
    fn test_encrypt_decrypt_after_handshake() {
        // --- Setup Handshake ---
        let initiator_keys = generate_keypair();
        let responder_keys = generate_keypair();

        let initiator_session = create_handshake_test_session(
            12345u32,
            true,
            &initiator_keys,
            responder_keys.public_key(),
        );
        let responder_session = create_handshake_test_session(
            12345u32,
            false,
            &responder_keys,
            initiator_keys.public_key(),
        );

        // Drive handshake to completion (simplified loop from previous test)
        let mut i_msg = initiator_session
            .prepare_handshake_message()
            .unwrap()
            .unwrap();
        responder_session.process_handshake_message(&i_msg).unwrap();
        let r_msg = responder_session
            .prepare_handshake_message()
            .unwrap()
            .unwrap();
        initiator_session.process_handshake_message(&r_msg).unwrap();
        i_msg = initiator_session
            .prepare_handshake_message()
            .unwrap()
            .unwrap();
        responder_session.process_handshake_message(&i_msg).unwrap();

        assert!(initiator_session.is_handshake_complete());
        assert!(responder_session.is_handshake_complete());

        // --- Test Encryption/Decryption ---
        let plaintext = b"Hello, Lewes Protocol!";

        // Initiator encrypts
        let ciphertext = initiator_session
            .encrypt_data(plaintext)
            .expect("Initiator encryption failed");
        assert_ne!(ciphertext.payload(), plaintext); // Ensure it's actually encrypted

        // Responder decrypts
        let decrypted = responder_session
            .decrypt_data(&ciphertext)
            .expect("Responder decryption failed");
        assert_eq!(decrypted, plaintext);

        // --- Test other direction ---
        let plaintext2 = b"Response from responder.";

        // Responder encrypts
        let ciphertext2 = responder_session
            .encrypt_data(plaintext2)
            .expect("Responder encryption failed");
        assert_ne!(ciphertext2.payload(), plaintext2);

        // Initiator decrypts
        let decrypted2 = initiator_session
            .decrypt_data(&ciphertext2)
            .expect("Initiator decryption failed");
        assert_eq!(decrypted2, plaintext2);
    }

    #[test]
    fn test_encrypt_decrypt_before_handshake() {
        let initiator_keys = generate_keypair();
        let responder_keys = generate_keypair();

        let initiator_session = create_handshake_test_session(
            12345u32,
            true,
            &initiator_keys,
            responder_keys.public_key(),
        );

        assert!(!initiator_session.is_handshake_complete());

        // Attempt to encrypt before handshake
        let plaintext = b"This should fail";
        let result = initiator_session.encrypt_data(plaintext);
        assert!(result.is_err());
        match result.unwrap_err() {
            NoiseError::PskNotInjected => {} // Expected - PSK check comes before handshake check
            e => panic!("Expected PskNotInjected, got {:?}", e),
        }

        // Attempt to decrypt before handshake (using dummy ciphertext)
        let dummy_ciphertext = vec![0u8; 32];
        let result_decrypt = initiator_session.decrypt_data(&LpMessage::EncryptedData(
            EncryptedDataPayload(dummy_ciphertext),
        ));
        assert!(result_decrypt.is_err());
        match result_decrypt.unwrap_err() {
            NoiseError::PskNotInjected => {} // Expected - PSK check comes before handshake check
            e => panic!("Expected PskNotInjected, got {:?}", e),
        }
    }

    /*
    // These tests remain commented as they rely on the old mock crypto functions
    #[test]
    fn test_mock_crypto() {
        let session = create_test_session(true);
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
        let session = create_test_session(true);
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

    // ====================================================================
    // PSQ Handshake Integration Tests
    // ====================================================================

    /// Test that PSQ runs during handshake and derives a PSK
    #[test]
    fn test_psq_handshake_runs_with_psk_injection() {
        let initiator_keys = generate_keypair();
        let responder_keys = generate_keypair();

        let initiator_session = create_handshake_test_session(
            12345u32,
            true,
            &initiator_keys,
            responder_keys.public_key(),
        );
        let responder_session = create_handshake_test_session(
            12345u32,
            false,
            &responder_keys,
            initiator_keys.public_key(),
        );

        // Drive the handshake
        let mut i_msg = initiator_session
            .prepare_handshake_message()
            .expect("Initiator should have message")
            .expect("Message prep should succeed");

        // The first message should contain PSQ payload embedded
        // Verify message is not empty and has reasonable size
        assert!(!i_msg.is_empty(), "Initiator message should not be empty");
        assert!(
            i_msg.len() > 100,
            "Message should contain PSQ payload (actual: {})",
            i_msg.len()
        );

        // Responder processes message (which includes PSQ decapsulation)
        responder_session
            .process_handshake_message(&i_msg)
            .expect("Responder should process first message");

        // Continue handshake
        let r_msg = responder_session
            .prepare_handshake_message()
            .expect("Responder should have message")
            .expect("Responder message prep should succeed");

        initiator_session
            .process_handshake_message(&r_msg)
            .expect("Initiator should process responder message");

        i_msg = initiator_session
            .prepare_handshake_message()
            .expect("Initiator should have final message")
            .expect("Final message prep should succeed");

        responder_session
            .process_handshake_message(&i_msg)
            .expect("Responder should process final message");

        // Verify handshake completed
        assert!(initiator_session.is_handshake_complete());
        assert!(responder_session.is_handshake_complete());

        // Verify encryption works (implicitly tests PSK was correctly injected)
        let plaintext = b"PSQ test message";
        let encrypted = initiator_session
            .encrypt_data(plaintext)
            .expect("Encryption should work after handshake");

        let decrypted = responder_session
            .decrypt_data(&encrypted)
            .expect("Decryption should work with PSQ-derived PSK");

        assert_eq!(decrypted, plaintext);
    }

    /// Test that X25519 keys are correctly converted to KEM format
    #[test]
    fn test_x25519_to_kem_conversion() {
        use nym_kkt::ciphersuite::EncapsulationKey;

        let initiator_keys = generate_keypair();
        let responder_keys = generate_keypair();

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

    /// Test that PSQ actually derives a different PSK (not using dummy)
    #[test]
    fn test_psq_derived_psk_differs_from_dummy() {
        let initiator_keys = generate_keypair();
        let responder_keys = generate_keypair();

        // Create sessions - they start with dummy PSK [0u8; 32]
        let initiator_session = create_handshake_test_session(
            12345u32,
            true,
            &initiator_keys,
            responder_keys.public_key(),
        );
        let responder_session = create_handshake_test_session(
            12345u32,
            false,
            &responder_keys,
            initiator_keys.public_key(),
        );

        // Prepare first message (initiator runs PSQ and injects PSK)
        let i_msg = initiator_session
            .prepare_handshake_message()
            .expect("Initiator should have message")
            .expect("Message prep should succeed");

        // Verify message is not empty (PSQ runs successfully)
        assert!(
            !i_msg.is_empty(),
            "First message should contain PSQ payload"
        );

        // Complete handshake
        responder_session
            .process_handshake_message(&i_msg)
            .expect("Responder should process message");

        let r_msg = responder_session
            .prepare_handshake_message()
            .unwrap()
            .unwrap();

        initiator_session.process_handshake_message(&r_msg).unwrap();

        let final_msg = initiator_session
            .prepare_handshake_message()
            .unwrap()
            .unwrap();

        responder_session
            .process_handshake_message(&final_msg)
            .unwrap();

        // Test that encryption produces non-trivial ciphertext
        // (would fail if using dummy PSK incorrectly)
        let plaintext = b"test";
        let encrypted = initiator_session.encrypt_data(plaintext).unwrap();

        // Decrypt should work
        let decrypted = responder_session.decrypt_data(&encrypted).unwrap();
        assert_eq!(decrypted, plaintext);

        // Verify ciphertext is not just plaintext (basic encryption sanity)
        if let LpMessage::EncryptedData(payload) = encrypted {
            assert_ne!(
                &payload.0[..plaintext.len()],
                plaintext,
                "Ciphertext should differ from plaintext"
            );
        } else {
            panic!("Expected EncryptedData message");
        }
    }

    /// Test full end-to-end handshake with PSQ integration
    #[test]
    fn test_handshake_with_psq_end_to_end() {
        let initiator_keys = generate_keypair();
        let responder_keys = generate_keypair();

        let initiator_session = create_handshake_test_session(
            12345u32,
            true,
            &initiator_keys,
            responder_keys.public_key(),
        );
        let responder_session = create_handshake_test_session(
            12345u32,
            false,
            &responder_keys,
            initiator_keys.public_key(),
        );

        // Verify initial state
        assert!(!initiator_session.is_handshake_complete());
        assert!(!responder_session.is_handshake_complete());
        assert!(initiator_session.is_initiator());
        assert!(!responder_session.is_initiator());

        // Round 1: Initiator -> Responder (contains PSQ encapsulation)
        let msg1 = initiator_session
            .prepare_handshake_message()
            .expect("Initiator should prepare message")
            .expect("Message should succeed");

        assert!(!msg1.is_empty());
        assert!(!initiator_session.is_handshake_complete());

        responder_session
            .process_handshake_message(&msg1)
            .expect("Responder should process PSQ message");

        assert!(!responder_session.is_handshake_complete());

        // Round 2: Responder -> Initiator
        let msg2 = responder_session
            .prepare_handshake_message()
            .expect("Responder should prepare message")
            .expect("Message should succeed");

        initiator_session
            .process_handshake_message(&msg2)
            .expect("Initiator should process message");

        // Round 3: Initiator -> Responder (final)
        let msg3 = initiator_session
            .prepare_handshake_message()
            .expect("Initiator should prepare final message")
            .expect("Message should succeed");

        responder_session
            .process_handshake_message(&msg3)
            .expect("Responder should process final message");

        // Verify both sides completed
        assert!(initiator_session.is_handshake_complete());
        assert!(responder_session.is_handshake_complete());

        // Test bidirectional encrypted communication
        let msg_i_to_r = b"Hello from initiator";
        let encrypted_i = initiator_session
            .encrypt_data(msg_i_to_r)
            .expect("Initiator encryption");
        let decrypted_i = responder_session
            .decrypt_data(&encrypted_i)
            .expect("Responder decryption");
        assert_eq!(decrypted_i, msg_i_to_r);

        let msg_r_to_i = b"Hello from responder";
        let encrypted_r = responder_session
            .encrypt_data(msg_r_to_i)
            .expect("Responder encryption");
        let decrypted_r = initiator_session
            .decrypt_data(&encrypted_r)
            .expect("Initiator decryption");
        assert_eq!(decrypted_r, msg_r_to_i);

        // Successfully completed end-to-end test with PSQ
    }

    /// Test that Ed25519 keys are used in PSQ authentication
    #[test]
    fn test_psq_handshake_uses_ed25519_authentication() {
        let initiator_keys = generate_keypair();
        let responder_keys = generate_keypair();

        // Create sessions with explicit Ed25519 keys
        let initiator_session = create_handshake_test_session(
            12345u32,
            true,
            &initiator_keys,
            responder_keys.public_key(),
        );
        let responder_session = create_handshake_test_session(
            12345u32,
            false,
            &responder_keys,
            initiator_keys.public_key(),
        );

        // Verify sessions store Ed25519 keys
        // (Internal verification - keys are used in PSQ calls)
        assert_eq!(initiator_session.id(), responder_session.id());

        // Complete handshake
        let msg1 = initiator_session
            .prepare_handshake_message()
            .unwrap()
            .unwrap();
        responder_session.process_handshake_message(&msg1).unwrap();

        let msg2 = responder_session
            .prepare_handshake_message()
            .unwrap()
            .unwrap();
        initiator_session.process_handshake_message(&msg2).unwrap();

        let msg3 = initiator_session
            .prepare_handshake_message()
            .unwrap()
            .unwrap();
        responder_session.process_handshake_message(&msg3).unwrap();

        // If Ed25519 authentication failed, handshake would not complete
        assert!(initiator_session.is_handshake_complete());
        assert!(responder_session.is_handshake_complete());

        // Verify encrypted communication works (proof of successful PSQ with auth)
        let test_data = b"Authentication test";
        let encrypted = initiator_session.encrypt_data(test_data).unwrap();
        let decrypted = responder_session.decrypt_data(&encrypted).unwrap();
        assert_eq!(decrypted, test_data);
    }

    #[test]
    fn test_psq_deserialization_failure() {
        // Test that corrupted PSQ payload causes clean abort
        let responder_keys = generate_keypair();
        let initiator_keys = generate_keypair();

        let responder_session = create_handshake_test_session(
            12345u32,
            false,
            &responder_keys,
            initiator_keys.public_key(),
        );

        // Create a handshake message with corrupted PSQ payload
        let corrupted_psq_data = vec![0xFF; 128]; // Random garbage
        let bad_message = LpMessage::Handshake(HandshakeData(corrupted_psq_data));

        // Attempt to process corrupted message - should fail
        let result = responder_session.process_handshake_message(&bad_message);

        // Should return error (PSQ deserialization will fail)
        assert!(result.is_err(), "Expected error for corrupted PSQ payload");

        // Verify session state is unchanged
        // PSQ state should still be ResponderWaiting (not modified)
        // Noise PSK should still be dummy [0u8; 32]
        assert!(!responder_session.is_handshake_complete());
    }

    #[test]
    fn test_handshake_abort_on_psq_failure() {
        // Test that Ed25519 auth failure causes handshake abort
        let initiator_keys = generate_keypair();
        let responder_keys = generate_keypair();

        // Create sessions with MISMATCHED Ed25519 keys
        // This simulates authentication failure
        let initiator_ed25519 = ed25519::KeyPair::from_secret([1u8; 32], 0);
        let wrong_ed25519 = ed25519::KeyPair::from_secret([99u8; 32], 99); // Different key!

        let receiver_index: u32 = 55555;
        let salt = [0u8; 32];

        let initiator_session = LpSession::new(
            receiver_index,
            true,
            Arc::new(initiator_ed25519),
            initiator_keys.private_key(),
            wrong_ed25519.public_key(), // Responder expects THIS key
            responder_keys.public_key(),
            &salt,
        )
        .unwrap();
        // Initialize KKT state for test
        initiator_session.set_kkt_completed_for_test(responder_keys.public_key());

        let responder_ed25519 = ed25519::KeyPair::from_secret([2u8; 32], 1);

        let responder_session = LpSession::new(
            receiver_index,
            false,
            Arc::new(responder_ed25519),
            responder_keys.private_key(),
            wrong_ed25519.public_key(), // Expects WRONG key (not initiator's)
            initiator_keys.public_key(),
            &salt,
        )
        .unwrap();
        // Initialize KKT state for test
        responder_session.set_kkt_completed_for_test(initiator_keys.public_key());

        // Initiator prepares message (should succeed - signing works)
        let msg1 = initiator_session
            .prepare_handshake_message()
            .expect("Initiator should prepare message")
            .expect("Initiator should have message");

        // Responder processes message - should FAIL (signature verification fails)
        let result = responder_session.process_handshake_message(&msg1);

        // Should return CredError due to Ed25519 signature mismatch
        assert!(
            result.is_err(),
            "Expected error for Ed25519 authentication failure"
        );

        // Verify handshake aborted cleanly
        assert!(!initiator_session.is_handshake_complete());
        assert!(!responder_session.is_handshake_complete());
    }

    #[test]
    fn test_psq_invalid_signature() {
        // Test Ed25519 signature validation specifically
        // Setup with matching X25519 keys but mismatched Ed25519 keys
        let initiator_keys = generate_keypair();
        let responder_keys = generate_keypair();

        // Initiator uses Ed25519 key [1u8]
        let initiator_ed25519 = ed25519::KeyPair::from_secret([1u8; 32], 0);

        // Responder expects Ed25519 key [99u8] (wrong!)
        let wrong_ed25519_keypair = ed25519::KeyPair::from_secret([99u8; 32], 99);
        let wrong_ed25519_public = wrong_ed25519_keypair.public_key();

        let receiver_index: u32 = 66666;
        let salt = [0u8; 32];

        let initiator_session = LpSession::new(
            receiver_index,
            true,
            Arc::new(initiator_ed25519),
            initiator_keys.private_key(),
            wrong_ed25519_public, // This doesn't matter for initiator
            responder_keys.public_key(),
            &salt,
        )
        .unwrap();
        // Initialize KKT state for test
        initiator_session.set_kkt_completed_for_test(responder_keys.public_key());

        let responder_ed25519 = ed25519::KeyPair::from_secret([2u8; 32], 1);

        let responder_session = LpSession::new(
            receiver_index,
            false,
            Arc::new(responder_ed25519),
            responder_keys.private_key(),
            wrong_ed25519_public, // Responder expects WRONG key
            initiator_keys.public_key(),
            &salt,
        )
        .unwrap();
        // Initialize KKT state for test
        responder_session.set_kkt_completed_for_test(initiator_keys.public_key());

        // Initiator creates message with valid signature (signed with [1u8])
        let msg = initiator_session
            .prepare_handshake_message()
            .unwrap()
            .unwrap();

        // Responder tries to verify with wrong public key [99u8]
        // This should fail Ed25519 signature verification
        let result = responder_session.process_handshake_message(&msg);

        assert!(result.is_err(), "Expected signature verification to fail");

        // Verify error is related to PSQ/authentication
        match result.unwrap_err() {
            LpError::Internal(msg) if msg.contains("PSQ") => {
                // Expected - PSQ v1 responder send failed due to CredError
            }
            e => panic!("Unexpected error type: {:?}", e),
        }
    }

    #[test]
    fn test_psq_state_unchanged_on_error() {
        // Verify that PSQ errors leave session in clean state
        let responder_keys = generate_keypair();
        let initiator_keys = generate_keypair();

        let responder_session = create_handshake_test_session(
            12345u32,
            false,
            &responder_keys,
            initiator_keys.public_key(),
        );

        // Capture initial PSQ state (should be ResponderWaiting)
        // (We can't directly access psq_state, but we can verify behavior)

        // Send corrupted data
        let corrupted_message = LpMessage::Handshake(HandshakeData(vec![0xFF; 100]));

        // Process should fail
        let result = responder_session.process_handshake_message(&corrupted_message);
        assert!(result.is_err());

        // After error, session should still be in handshake mode (not complete)
        assert!(!responder_session.is_handshake_complete());

        // Session should still be functional - can process valid messages
        // Create a proper initiator to send valid message
        let initiator_session = create_handshake_test_session(
            12345u32,
            true,
            &initiator_keys,
            responder_keys.public_key(),
        );

        let valid_msg = initiator_session
            .prepare_handshake_message()
            .unwrap()
            .unwrap();

        // After the error, responder should still be able to process valid messages
        let result2 = responder_session.process_handshake_message(&valid_msg);

        // Should succeed (session state was not corrupted by previous error)
        assert!(
            result2.is_ok(),
            "Session should still be functional after PSQ error"
        );
    }

    #[test]
    fn test_transport_fails_without_psk_injection() {
        // This test verifies the safety mechanism that prevents transport mode operations
        // from running with the dummy PSK if PSQ injection fails or is skipped.

        let initiator_keys = generate_keypair();
        let responder_keys = generate_keypair();

        // Create session but don't complete handshake (no PSK injection will occur)
        let session = create_handshake_test_session(
            12345u32,
            true,
            &initiator_keys,
            responder_keys.public_key(),
        );

        // Verify session was created successfully
        assert!(!session.is_handshake_complete());

        // Attempt to encrypt data - should fail with PskNotInjected
        let plaintext = b"test data";
        let encrypt_result = session.encrypt_data(plaintext);

        assert!(
            encrypt_result.is_err(),
            "encrypt_data should fail without PSK injection"
        );
        match encrypt_result.unwrap_err() {
            NoiseError::PskNotInjected => {
                // Expected - this is the safety mechanism working
            }
            e => panic!("Expected PskNotInjected error, got: {:?}", e),
        }

        // Create a dummy encrypted message to test decrypt
        let dummy_ciphertext = LpMessage::EncryptedData(EncryptedDataPayload(vec![0u8; 48]));

        // Attempt to decrypt data - should also fail with PskNotInjected
        let decrypt_result = session.decrypt_data(&dummy_ciphertext);

        assert!(
            decrypt_result.is_err(),
            "decrypt_data should fail without PSK injection"
        );
        match decrypt_result.unwrap_err() {
            NoiseError::PskNotInjected => {
                // Expected - this is the safety mechanism working
            }
            e => panic!("Expected PskNotInjected error, got: {:?}", e),
        }
    }

    #[test]
    fn test_demote_sets_read_only() {
        let initiator_keys = generate_keypair();
        let responder_keys = generate_keypair();

        let session = create_handshake_test_session(
            12345u32,
            true,
            &initiator_keys,
            responder_keys.public_key(),
        );

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
        let initiator_keys = generate_keypair();
        let responder_keys = generate_keypair();

        let initiator_session = create_handshake_test_session(
            12345u32,
            true,
            &initiator_keys,
            responder_keys.public_key(),
        );
        let responder_session = create_handshake_test_session(
            12345u32,
            false,
            &responder_keys,
            initiator_keys.public_key(),
        );

        // Drive handshake to completion
        let i_msg = initiator_session
            .prepare_handshake_message()
            .unwrap()
            .unwrap();
        responder_session.process_handshake_message(&i_msg).unwrap();
        let r_msg = responder_session
            .prepare_handshake_message()
            .unwrap()
            .unwrap();
        initiator_session.process_handshake_message(&r_msg).unwrap();
        let i_msg = initiator_session
            .prepare_handshake_message()
            .unwrap()
            .unwrap();
        responder_session.process_handshake_message(&i_msg).unwrap();

        assert!(initiator_session.is_handshake_complete());

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
        let initiator_keys = generate_keypair();
        let responder_keys = generate_keypair();

        let initiator_session = create_handshake_test_session(
            12345u32,
            true,
            &initiator_keys,
            responder_keys.public_key(),
        );
        let responder_session = create_handshake_test_session(
            12345u32,
            false,
            &responder_keys,
            initiator_keys.public_key(),
        );

        // Drive handshake to completion
        let i_msg = initiator_session
            .prepare_handshake_message()
            .unwrap()
            .unwrap();
        responder_session.process_handshake_message(&i_msg).unwrap();
        let r_msg = responder_session
            .prepare_handshake_message()
            .unwrap()
            .unwrap();
        initiator_session.process_handshake_message(&r_msg).unwrap();
        let i_msg = initiator_session
            .prepare_handshake_message()
            .unwrap()
            .unwrap();
        responder_session.process_handshake_message(&i_msg).unwrap();

        assert!(initiator_session.is_handshake_complete());
        assert!(responder_session.is_handshake_complete());

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
