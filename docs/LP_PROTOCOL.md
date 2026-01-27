# Lewes Protocol (LP) - Technical Specification

## Overview

The Lewes Protocol (LP) is a direct TCP-based registration protocol for Nym gateways. It provides an alternative to mixnet-based registration with different trade-offs: lower latency at the cost of revealing client IP to the gateway.

**Design Goals:**
- **Low latency**: Direct TCP connection vs multi-hop mixnet routing
- **High reliability**: KCP protocol provides ordered, reliable delivery with ARQ
- **Strong security**: Noise XKpsk3 provides mutual authentication and forward secrecy
- **Replay protection**: Bitmap-based counter validation prevents replay attacks
- **Observability**: Prometheus metrics for production monitoring

**Non-Goals:**
- Network-level anonymity (use mixnet registration for that)
- Persistent connections (LP is registration-only, single-use)
- Backward compatibility with legacy protocols

## Architecture

### Protocol Stack

```
┌─────────────────────────────────────────┐
│  Application Layer                      │
│  - Registration Requests                │
│  - E-cash Credential Verification       │
│  - WireGuard Peer Management            │
└─────────────────────────────────────────┘
                  ↓
┌─────────────────────────────────────────┐
│  LP Layer (Lewes Protocol)              │
│  - Noise XKpsk3 Handshake               │
│  - Replay Protection (1024-pkt window)  │
│  - Counter-based Sequencing             │
└─────────────────────────────────────────┘
                  ↓
┌─────────────────────────────────────────┐
│  KCP Layer (Reliability)                │
│  - Ordered Delivery                     │
│  - ARQ with Selective ACK               │
│  - Congestion Control                   │
│  - RTT Estimation                       │
└─────────────────────────────────────────┘
                  ↓
┌─────────────────────────────────────────┐
│  TCP Layer                              │
│  - Connection Establishment             │
│  - Byte Stream Delivery                 │
└─────────────────────────────────────────┘
```

### Why This Layering?

**TCP**: Provides connection-oriented byte stream and handles network-level retransmission.

**KCP**: Adds application-level reliability optimized for low latency:
- **Fast retransmit**: Triggered after 2 duplicate ACKs (vs TCP's 3)
- **Selective ACK**: Acknowledges specific packets, not just cumulative
- **Configurable RTO**: Minimum RTO of 100ms (configurable)
- **No Nagle**: Immediate sending for low-latency applications

**LP**: Provides cryptographic security and session management:
- **Noise XKpsk3**: Mutual authentication with pre-shared key
- **Replay protection**: Prevents duplicate packet acceptance
- **Session isolation**: Each session has unique cryptographic state

**Application**: Business logic for registration and credential verification.

## Protocol Flow

### 1. Connection Establishment

```
Client                                    Gateway
  |                                          |
  |--- TCP SYN --------------------------->  |
  |<-- TCP SYN-ACK ------------------------  |
  |--- TCP ACK ----------------------------> |
  |                                          |
```

- **Control Port**: 41264 (default, configurable)
- **Data Port**: 51264 (reserved for future use, not currently used)

### 2. Session Initialization

Client generates session parameters:

```rust
// Client-side session setup
let client_lp_keypair = Keypair::generate(); // X25519 keypair
let gateway_lp_public = gateway.lp_public_key; // From gateway descriptor
let salt = [timestamp (8 bytes) || nonce (24 bytes)]; // 32-byte salt

// Derive PSK using ECDH + Blake3 KDF
let shared_secret = ECDH(client_private, gateway_public);
let psk = Blake3_derive_key(
    context = "nym-lp-psk-v1",
    input = shared_secret,
    salt = salt
);

// Calculate session IDs (deterministic from keys)
let lp_id = hash(client_lp_public || 0xCC || gateway_lp_public) & 0xFFFFFFFF;
let kcp_conv_id = hash(client_lp_public || 0xFF || gateway_lp_public) & 0xFFFFFFFF;
```

**Session ID Properties:**
- **Deterministic**: Same key pair always produces same ID
- **Order-independent**: `ID(A, B) == ID(B, A)` due to sorted hashing
- **Collision-resistant**: Uses full hash, truncated to u32
- **Unique per protocol**: Different delimiters (0xCC for LP, 0xFF for KCP)

### 3. Noise Handshake (XKpsk3 Pattern)

```
Client (Initiator)                        Gateway (Responder)
  |                                          |
  |--- e ----------------------------------> | [1] Client ephemeral
  |                                          |
  |<-- e, ee, s, es ---------------------  | [2] Gateway ephemeral + static
  |                                          |
  |--- s, se, psk ------------------------->  | [3] Client static + PSK mix
  |                                          |
  [Transport mode established]
```

**Message Contents:**

**[1] Initiator → Responder: `e`**
- Payload: Client ephemeral public key (32 bytes)
- Encrypted: No (initial message)

**[2] Responder → Initiator: `e, ee, s, es`**
- `e`: Responder ephemeral public key
- `ee`: Mix ephemeral-ephemeral DH
- `s`: Responder static public key (encrypted)
- `es`: Mix ephemeral-static DH
- Encrypted: Yes (with keys from `ee`)

**[3] Initiator → Responder: `s, se, psk`**
- `s`: Initiator static public key (encrypted)
- `se`: Mix static-ephemeral DH
- `psk`: Mix pre-shared key (at position 3)
- Encrypted: Yes (with keys from `ee`, `es`)

**Security Properties:**
- ✅ **Mutual authentication**: Both sides prove identity via static keys
- ✅ **Forward secrecy**: Ephemeral keys provide PFS
- ✅ **PSK authentication**: Binds session to out-of-band PSK
- ✅ **Identity hiding**: Static keys encrypted after first message

**Handshake Characteristics:**
- **Messages**: 3 (1.5 round trips)
- **Minimum network RTTs**: 1.5
- **Cryptographic operations**: ECDH, ChaCha20-Poly1305, SHA-256

### 4. PSK Derivation Details

**Formula:**
```
shared_secret = X25519(client_private_lp, gateway_public_lp)
psk = Blake3_derive_key(
    context = "nym-lp-psk-v1",
    key_material = shared_secret (32 bytes),
    salt = timestamp || nonce (32 bytes)
)
```

**Implementation** (from `common/nym-lp/src/psk.rs:48`):
```rust
pub fn derive_psk(
    local_private: &PrivateKey,
    remote_public: &PublicKey,
    salt: &[u8; 32],
) -> [u8; 32] {
    let shared_secret = local_private.diffie_hellman(remote_public);
    nym_crypto::kdf::derive_key_blake3(PSK_CONTEXT, shared_secret.as_bytes(), salt)
}
```

**Why This Design:**

1. **Identity-bound**: PSK tied to LP keypairs, not ephemeral
   - Prevents MITM without LP private key
   - Links session to long-term identities

2. **Session-specific via salt**: Different registrations use different PSKs
   - `timestamp`: 8-byte Unix timestamp (milliseconds)
   - `nonce`: 24-byte random value
   - Prevents PSK reuse across sessions

3. **Symmetric derivation**: Both sides derive same PSK
   - Client: `ECDH(client_priv, gateway_pub)`
   - Gateway: `ECDH(gateway_priv, client_pub)`
   - Mathematical property: `ECDH(a, B) == ECDH(b, A)`

4. **Blake3 KDF with domain separation**:
   - Context string prevents cross-protocol attacks
   - Generates uniform 32-byte output suitable for Noise

**Salt Transmission:**
- Included in `ClientHello` message (cleartext)
- Gateway extracts salt before deriving PSK
- Timestamp validation rejects stale salts

### 5. Replay Protection

**Mechanism: Sliding Window with Bitmap** (from `common/nym-lp/src/replay/validator.rs:32`):

```rust
const WORD_SIZE: usize = 64;
const N_WORDS: usize = 16;  // 1024 bits total
const N_BITS: usize = WORD_SIZE * N_WORDS;  // 1024

pub struct ReceivingKeyCounterValidator {
    next: u64,              // Next expected counter
    receive_cnt: u64,       // Total packets received
    bitmap: [u64; 16],      // 1024-bit bitmap
}
```

**Algorithm:**
```
For each incoming packet with counter C:
  1. Quick check (branchless):
     - If C >= next: Accept (growing)
     - If C + 1024 < next: Reject (too old, outside window)
     - If bitmap[C % 1024] is set: Reject (duplicate)
     - Else: Accept (out-of-order within window)

  2. After successful processing, mark:
     - Set bitmap[C % 1024] = 1
     - If C >= next: Update next = C + 1
     - Increment receive_cnt
```

**Performance Optimizations:**

1. **SIMD-accelerated bitmap operations** (from `common/nym-lp/src/replay/simd/`):
   - AVX2 support (x86_64)
   - SSE2 support (x86_64)
   - NEON support (ARM)
   - Scalar fallback (portable)

2. **Branchless execution** (constant-time):
   ```rust
   // No early returns - prevents timing attacks
   let result = if is_growing {
       Some(Ok(()))
   } else if too_far_back {
       Some(Err(ReplayError::OutOfWindow))
   } else if duplicate {
       Some(Err(ReplayError::DuplicateCounter))
   } else {
       Some(Ok(()))
   };
   result.unwrap()
   ```

3. **Overflow-safe arithmetic**:
   ```rust
   let too_far_back = if counter > u64::MAX - 1024 {
       false  // Can't overflow, so not too far back
   } else {
       counter + 1024 < self.next
   };
   ```

**Memory Usage** (verified from `common/nym-lp/src/replay/validator.rs:738`):
```rust
// test_memory_usage()
size = size_of::<u64>() * 2 +        // next + receive_cnt = 16 bytes
       size_of::<u64>() * N_WORDS;   // bitmap = 128 bytes
// Total: 144 bytes
```

### 6. Registration Request

After handshake completes, client sends encrypted registration request:

```rust
pub struct RegistrationRequest {
    pub mode: RegistrationMode,
    pub credential: EcashCredential,
    pub gateway_identity: String,
}

pub enum RegistrationMode {
    Dvpn {
        wg_public_key: [u8; 32],
    },
    Mixnet {
        client_id: String,
        mix_address: Option<String>,
    },
}
```

**Encryption:**
- Encrypted using Noise transport mode
- Includes 16-byte Poly1305 authentication tag
- Replay protection via LP counter

### 7. Credential Verification

Gateway verifies the e-cash credential:

```rust
// Gateway-side verification
pub async fn verify_credential(
    &self,
    credential: &EcashCredential,
) -> Result<VerifiedCredential, CredentialError> {
    // 1. Check credential signature (BLS12-381)
    verify_blinded_signature(&credential.signature)?;

    // 2. Check credential not already spent (nullifier check)
    if self.storage.is_nullifier_spent(&credential.nullifier).await? {
        return Err(CredentialError::AlreadySpent);
    }

    // 3. Extract bandwidth allocation
    let bandwidth_bytes = credential.bandwidth_value;

    // 4. Mark nullifier as spent
    self.storage.mark_nullifier_spent(&credential.nullifier).await?;

    Ok(VerifiedCredential {
        bandwidth_bytes,
        expiry: credential.expiry,
    })
}
```

**For dVPN Mode:**
```rust
let peer_config = WireguardPeerConfig {
    public_key: request.wg_public_key,
    allowed_ips: vec!["10.0.0.0/8"],
    bandwidth_limit: verified.bandwidth_bytes,
};
self.wg_controller.add_peer(peer_config).await?;
```

### 8. Registration Response

```rust
pub enum RegistrationResponse {
    Success {
        bandwidth_allocated: u64,
        expiry: u64,
        gateway_data: GatewayData,
    },
    Error {
        code: ErrorCode,
        message: String,
    },
}

pub enum ErrorCode {
    InvalidCredential = 1,
    CredentialExpired = 2,
    CredentialAlreadyUsed = 3,
    InsufficientBandwidth = 4,
    WireguardPeerRegistrationFailed = 5,
    InternalError = 99,
}
```

## State Machine and Security Protocol

### Protocol Components

The Lewes Protocol combines three cryptographic protocols for secure, post-quantum resistant communication:

1. **KKT (KEM Key Transfer)** - Dynamically fetches responder's KEM public key with Ed25519 authentication
2. **PSQ (Post-Quantum Secure PSK)** - Derives PSK using KEM-based protocol for HNDL resistance
3. **Noise XKpsk3** - Provides encrypted transport with mutual authentication and forward secrecy

### State Machine

The LP state machine orchestrates the complete protocol flow from connection to encrypted transport:

```
┌─────────────────────────────────────────────────────────────────────┐
│                    LEWES PROTOCOL STATE MACHINE                     │
└─────────────────────────────────────────────────────────────────────┘

                    ┌──────────────────┐
                    │ ReadyToHandshake │
                    │                  │
                    │ • Keys loaded    │
                    │ • Session ID set │
                    └────────┬─────────┘
                             │
                    StartHandshake input
                             │
                             ▼
         ┌───────────────────────────────────────┐
         │          KKTExchange                  │
         │                                       │
         │  Initiator:                          │
         │  1. Send KKT request (signed)        │
         │  2. Receive KKT response             │
         │  3. Validate Ed25519 signature       │
         │  4. Extract KEM public key           │
         │                                       │
         │  Responder:                          │
         │  1. Wait for KKT request             │
         │  2. Validate signature               │
         │  3. Send signed KEM key              │
         └───────────────┬───────────────────────┘
                         │
                  KKT Complete
                         │
                         ▼
         ┌───────────────────────────────────────┐
         │           Handshaking                 │
         │                                       │
         │  PSQ Protocol:                       │
         │  1. Initiator encapsulates PSK       │
         │     (embedded in Noise msg 1)        │
         │  2. Responder decapsulates PSK       │
         │     (sends ctxt_B in Noise msg 2)    │
         │  3. Both derive final PSK:           │
         │     KDF(ECDH || KEM_shared)          │
         │                                       │
         │  Noise XKpsk3 Handshake:             │
         │  → msg 1: e, es, ss + PSQ payload    │
         │  ← msg 2: e, ee, se + ctxt_B         │
         │  → msg 3: s, se (handshake complete) │
         └───────────────┬───────────────────────┘
                         │
                  Handshake Complete
                         │
                         ▼
         ┌───────────────────────────────────────┐
         │            Transport                  │
         │                                       │
         │  • Encrypted data transfer            │
         │  • AEAD with ChaCha20-Poly1305       │
         │  • Replay protection (counters)       │
         │  • Bidirectional communication        │
         └───────────────┬───────────────────────┘
                         │
                    Close input
                         │
                         ▼
                   ┌──────────┐
                   │  Closed  │
                   │          │
                   │ • Reason │
                   └──────────┘
```

### Message Sequence

Complete protocol flow from connection to encrypted transport:

```
Initiator                                                    Responder
    │                                                            │
    │ ════════════════ KKT EXCHANGE ════════════════            │
    │                                                            │
    │  KKTRequest (signed with Ed25519)                         │
    ├──────────────────────────────────────────────────────────>│
    │                                                            │ Validate
    │                                                            │ signature
    │                        KKTResponse (signed KEM key + hash) │
    │<──────────────────────────────────────────────────────────┤
    │                                                            │
    │ Validate signature                                         │
    │ Extract kem_pk                                             │
    │                                                            │
    │ ══════════════ PSQ + NOISE HANDSHAKE ══════════════       │
    │                                                            │
    │  Noise msg 1: e, es, ss                                   │
    │  + PSQ InitiatorMsg (KEM encapsulation)                   │
    ├──────────────────────────────────────────────────────────>│
    │                                                            │
    │                                                            │ PSQ: Decapsulate
    │                                                            │ Derive PSK
    │                                                            │ Inject into Noise
    │                                  Noise msg 2: e, ee, se    │
    │                                  + ctxt_B (encrypted PSK)  │
    │<──────────────────────────────────────────────────────────┤
    │                                                            │
    │ Extract ctxt_B                                             │
    │ Store for re-registration                                 │
    │ Inject PSK into Noise                                      │
    │                                                            │
    │  Noise msg 3: s, se                                       │
    ├──────────────────────────────────────────────────────────>│
    │                                                            │
    │ Handshake Complete ✓                                       │ Handshake Complete ✓
    │ Transport mode active                                      │ Transport mode active
    │                                                            │
    │ ═══════════════ TRANSPORT MODE ═══════════════            │
    │                                                            │
    │  EncryptedData (AEAD, counter N)                          │
    ├──────────────────────────────────────────────────────────>│
    │                                                            │
    │                                  EncryptedData (counter M) │
    │<──────────────────────────────────────────────────────────┤
    │                                                            │
    │  (bidirectional encrypted communication)                  │
    │◄──────────────────────────────────────────────────────────►
    │                                                            │
```

### KKT (KEM Key Transfer) Protocol

**Purpose**: Securely obtain responder's KEM public key before PSQ can begin.

**Key Features**:
- Ed25519 signatures for authentication (both request and response signed)
- Optional hash validation for key pinning (future directory service integration)
- Currently signature-only mode (deployable without infrastructure)
- Easy upgrade path to hash-based key pinning

**Initiator Flow**:
```rust
1. Generate KKT request with Ed25519 signature
2. Send KKTRequest to responder
3. Receive KKTResponse with signed KEM key
4. Validate Ed25519 signature
5. (Optional) Validate key hash against directory
6. Store KEM key for PSQ encapsulation
```

**Responder Flow**:
```rust
1. Receive KKTRequest from initiator
2. Validate initiator's Ed25519 signature
3. Generate KKTResponse with:
   - Responder's KEM public key
   - Ed25519 signature over (key || timestamp)
   - Blake3 hash of KEM key
4. Send KKTResponse to initiator
```

### PSQ (Post-Quantum Secure PSK) Protocol

**Purpose**: Derive a post-quantum secure PSK for Noise protocol.

**Security Properties**:
- **HNDL resistance**: PSK derived from KEM-based protocol
- **Forward secrecy**: Ephemeral KEM keypair per session
- **Authentication**: Ed25519 signatures prevent MitM
- **Algorithm agility**: Easy upgrade from X25519 to ML-KEM

**PSK Derivation**:
```
Classical ECDH:
  ecdh_secret = X25519_DH(local_private, remote_public)

KEM Encapsulation (Initiator):
  (kem_shared_secret, ciphertext) = KEM.Encap(responder_kem_pk)

KEM Decapsulation (Responder):
  kem_shared_secret = KEM.Decap(kem_private, ciphertext)

Final PSK:
  combined = ecdh_secret || kem_shared_secret || salt
  psk = Blake3_KDF("nym-lp-psk-psq-v1", combined)
```

**Integration with Noise**:
- PSQ payload embedded in first Noise message (no extra round-trip)
- Responder sends encrypted PSK handle (ctxt_B) in second Noise message
- Both sides inject derived PSK before completing Noise handshake
- Noise validates PSK correctness during handshake

**PSK Handle (ctxt_B)**:
The responder's encrypted PSK handle allows future re-registration without repeating PSQ:
- Encrypted with responder's long-term key
- Can be presented in future sessions
- Enables fast re-registration for returning clients

### Security Guarantees

**Achieved Properties**:
- ✅ **Mutual authentication**: Ed25519 signatures in KKT and PSQ
- ✅ **Forward secrecy**: Ephemeral keys in Noise handshake
- ✅ **Post-quantum PSK**: KEM-based PSK derivation
- ✅ **HNDL resistance**: PSK safe even if private keys compromised later
- ✅ **Replay protection**: Monotonic counters with sliding window
- ✅ **Key confirmation**: Noise handshake validates PSK correctness

**Implementation Status**:
- 🔄 **Key pinning**: Hash validation via directory service (signature-only for now)
- 🔄 **ML-KEM support**: Easy config upgrade from X25519 to ML-KEM-768
- 🔄 **PSK re-use**: ctxt_B handle stored for future re-registration

### Algorithm Choices

**Current (Testing/Development)**:
- KEM: X25519 (DHKEM) - Classical ECDH, widely tested
- Hash: Blake3 - Fast, secure, parallel
- Signature: Ed25519 - Fast verification, compact
- AEAD: ChaCha20-Poly1305 - Fast, constant-time

**Future (Production)**:
- KEM: ML-KEM-768 - NIST-approved post-quantum KEM
- Hash: Blake3 - No change needed
- Signature: Ed25519 - No change needed (or upgrade to ML-DSA)
- AEAD: ChaCha20-Poly1305 - No change needed

**Migration Path**:
```toml
# Current deployment
[lp.crypto]
kem_algorithm = "x25519"

# Future upgrade (config change only)
[lp.crypto]
kem_algorithm = "ml-kem-768"
```

### Message Types

**KKT Messages**:
```rust
// Message Type 0x0004
struct KKTRequest {
    timestamp: u64,              // Unix timestamp (replay protection)
    initiator_ed25519_pk: [u8; 32], // Initiator's public key
    signature: [u8; 64],         // Ed25519 signature
}

// Message Type 0x0005
struct KKTResponse {
    kem_pk: Vec<u8>,             // Responder's KEM public key
    key_hash: [u8; 32],          // Blake3 hash of KEM key
    timestamp: u64,              // Unix timestamp
    signature: [u8; 64],         // Ed25519 signature
}
```

**PSQ Embedding**:
- PSQ InitiatorMsg embedded in Noise message 1 payload (after 'e, es, ss')
- PSQ ResponderMsg (ctxt_B) embedded in Noise message 2 payload (after 'e, ee, se')
- No additional round-trips beyond standard 3-message Noise handshake

## KCP Protocol Details

### KCP Configuration

From `common/nym-kcp/src/session.rs`:

```rust
pub struct KcpSession {
    conv: u32,           // Conversation ID
    mtu: usize,          // Default: 1400 bytes
    snd_wnd: u16,        // Send window: 128 segments
    rcv_wnd: u16,        // Receive window: 128 segments
    rx_minrto: u32,      // Minimum RTO: 100ms (configurable)
}
```

### KCP Packet Format

```
┌────────────────────────────────────────────────┐
│ Conv ID (4 bytes) - Conversation identifier    │
├────────────────────────────────────────────────┤
│ Cmd (1 byte) - PSH/ACK/WND/ERR                │
├────────────────────────────────────────────────┤
│ Frg (1 byte) - Fragment number (reverse order) │
├────────────────────────────────────────────────┤
│ Wnd (2 bytes) - Receive window size           │
├────────────────────────────────────────────────┤
│ Timestamp (4 bytes) - Send timestamp           │
├────────────────────────────────────────────────┤
│ Sequence Number (4 bytes) - Packet sequence    │
├────────────────────────────────────────────────┤
│ UNA (4 bytes) - Unacknowledged sequence       │
├────────────────────────────────────────────────┤
│ Length (4 bytes) - Data length                 │
├────────────────────────────────────────────────┤
│ Data (variable) - Payload                      │
└────────────────────────────────────────────────┘
```

**Total header**: 24 bytes

### KCP Features

**Reliability Mechanisms:**
- **Sequence Numbers (sn)**: Track packet ordering
- **Fragment Numbers (frg)**: Handle message fragmentation
- **UNA (Unacknowledged)**: Cumulative ACK up to this sequence
- **Selective ACK**: Via individual ACK packets
- **Fast Retransmit**: Triggered by duplicate ACKs (configurable threshold)
- **RTO Calculation**: Smoothed RTT with variance

## LP Packet Format

### LP Header

```
┌────────────────────────────────────────────────┐
│ Protocol Version (1 byte) - Currently: 1       │
├────────────────────────────────────────────────┤
│ Session ID (4 bytes) - LP session identifier   │
├────────────────────────────────────────────────┤
│ Counter (8 bytes) - Replay protection counter  │
└────────────────────────────────────────────────┘
```

**Total header**: 13 bytes

### LP Message Types

```rust
pub enum LpMessage {
    Handshake(Vec<u8>),
    EncryptedData(Vec<u8>),
    ClientHello {
        client_lp_public: [u8; 32],
        salt: [u8; 32],
        timestamp: u64,
    },
    Busy,
}
```

### Complete Packet Structure

```
┌─────────────────────────────────────┐
│  LP Header (13 bytes)               │
│  - Version, Session ID, Counter     │
├─────────────────────────────────────┤
│  LP Message (variable)              │
│  - Type tag (1 byte)                │
│  - Message data                     │
├─────────────────────────────────────┤
│  Trailer (16 bytes)                 │
│  - Reserved for future MAC/tag      │
└─────────────────────────────────────┘
```

## Security Properties

### Threat Model

**Protected Against:**
- ✅ **Passive eavesdropping**: Noise encryption (ChaCha20-Poly1305)
- ✅ **Active MITM**: Mutual authentication via static keys + PSK
- ✅ **Replay attacks**: Counter-based validation with 1024-packet window
- ✅ **Packet injection**: Poly1305 authentication tags
- ✅ **Timestamp replay**: 30-second window for ClientHello timestamps (configurable)
- ✅ **DoS (connection flood)**: Connection limit (default: 10,000, configurable)
- ✅ **Credential reuse**: Nullifier tracking in database

**Not Protected Against:**
- ❌ **Network-level traffic analysis**: LP is not anonymous (use mixnet for that)
- ❌ **Gateway compromise**: Gateway sees client registration data
- ⚠️ **Per-IP DoS**: No per-IP rate limiting (global limit only)

### Cryptographic Primitives

| Component | Algorithm | Key Size | Source |
|-----------|-----------|----------|--------|
| Key Exchange | X25519 | 256 bits | RustCrypto |
| Encryption | ChaCha20 | 256 bits | RustCrypto |
| Authentication | Poly1305 | 256 bits | RustCrypto |
| KDF | Blake3 | 256 bits | nym_crypto |
| Hash (Noise) | SHA-256 | 256 bits | snow crate |
| Signature (E-cash) | BLS12-381 | 381 bits | E-cash contract |

### Forward Secrecy

Noise XKpsk3 provides forward secrecy through ephemeral keys:

1. **Initial handshake**: Uses ephemeral + static keys
2. **Key compromise scenario**:
   - Compromise of **static key**: Past sessions remain secure (ephemeral keys destroyed)
   - Compromise of **PSK**: Attacker needs static key too (two-factor security)
   - Compromise of **both**: Only future sessions affected, not past

3. **Session key lifetime**: Destroyed after single registration completes

### Timing Attack Resistance

**Constant-time operations:**
- ✅ Replay protection check (branchless)
- ✅ Bitmap bit operations (branchless)
- ✅ Noise crypto operations (via snow/RustCrypto)

**Variable-time operations:**
- ⚠️ Credential verification (database lookup time varies)
- ⚠️ WireGuard peer registration (filesystem operations)

## Configuration

### Gateway Configuration

From `gateway/src/node/lp_listener/mod.rs:78`:

```toml
[lp]
# Enable/disable LP listener
enabled = true

# Bind address
bind_address = "0.0.0.0"

# Control port (for LP handshake and registration)
control_port = 41264

# Data port (reserved for future use)
data_port = 51264

# Maximum concurrent connections
max_connections = 10000

# Timestamp validation window (seconds)
# ClientHello messages older than this are rejected
timestamp_tolerance_secs = 30

# Use mock e-cash verifier (TESTING ONLY!)
use_mock_ecash = false
```

### Firewall Rules

**Required inbound rules:**
```bash
# Allow TCP connections to LP control port
iptables -A INPUT -p tcp --dport 41264 -j ACCEPT

# Optional: Rate limiting
iptables -A INPUT -p tcp --dport 41264 -m state --state NEW \
    -m recent --set --name LP_LIMIT
iptables -A INPUT -p tcp --dport 41264 -m state --state NEW \
    -m recent --update --seconds 60 --hitcount 100 --name LP_LIMIT \
    -j DROP
```

## Metrics

From `gateway/src/node/lp_listener/mod.rs:4`:

**Connection Metrics:**
- `active_lp_connections`: Gauge tracking current active LP connections
- `lp_connections_total`: Counter for total LP connections handled
- `lp_connection_duration_seconds`: Histogram of connection durations
- `lp_connections_completed_gracefully`: Counter for successful completions
- `lp_connections_completed_with_error`: Counter for error terminations

**Handshake Metrics:**
- `lp_handshakes_success`: Counter for successful handshakes
- `lp_handshakes_failed`: Counter for failed handshakes
- `lp_handshake_duration_seconds`: Histogram of handshake durations
- `lp_client_hello_failed`: Counter for ClientHello failures

**Registration Metrics:**
- `lp_registration_attempts_total`: Counter for all registration attempts
- `lp_registration_success_total`: Counter for successful registrations
- `lp_registration_failed_total`: Counter for failed registrations
- `lp_registration_duration_seconds`: Histogram of registration durations

**Mode-Specific:**
- `lp_registration_dvpn_attempts/success/failed`: dVPN mode counters
- `lp_registration_mixnet_attempts/success/failed`: Mixnet mode counters

**Credential Metrics:**
- `lp_credential_verification_attempts/success/failed`: Verification counters
- `lp_bandwidth_allocated_bytes_total`: Total bandwidth allocated

**Error Metrics:**
- `lp_errors_handshake`: Handshake errors
- `lp_errors_timestamp_too_old/too_far_future`: Timestamp validation errors
- `lp_errors_wg_peer_registration`: WireGuard peer registration failures

## Error Codes

### Handshake Errors

| Error | Description |
|-------|-------------|
| `NOISE_DECRYPT_ERROR` | Invalid ciphertext or wrong keys |
| `NOISE_PROTOCOL_ERROR` | Unexpected message or state |
| `REPLAY_DUPLICATE` | Counter already seen |
| `REPLAY_OUT_OF_WINDOW` | Counter outside 1024-packet window |
| `TIMESTAMP_TOO_OLD` | ClientHello > configured tolerance |
| `TIMESTAMP_FUTURE` | ClientHello from future |

### Registration Errors

| Code | Name | Description |
|------|------|-------------|
| `CREDENTIAL_INVALID` | Invalid credential | Signature verification failed |
| `CREDENTIAL_EXPIRED` | Credential expired | Past expiry timestamp |
| `CREDENTIAL_SPENT` | Already used | Nullifier already in database |
| `INSUFFICIENT_BANDWIDTH` | Not enough bandwidth | Requested > credential value |
| `WIREGUARD_FAILED` | Peer registration failed | Kernel error adding WireGuard peer |

## Limitations

### Current Limitations

1. **No persistent sessions**: Each registration is independent
2. **Single registration per session**: Connection closes after registration
3. **No streaming**: Protocol is request-response only
4. **No gateway discovery**: Client must know gateway's LP public key beforehand
5. **No version negotiation**: Protocol version fixed at 1
6. **No per-IP rate limiting**: Only global connection limit

### Testing Gaps

1. **No end-to-end integration tests**: Unit tests exist, integration tests pending
2. **No performance benchmarks**: Latency/throughput not measured
3. **No load testing**: Concurrent connection limits not stress-tested
4. **No security audit**: Cryptographic implementation not externally reviewed

## References

### Specifications

- **Noise Protocol Framework**: https://noiseprotocol.org/noise.html
- **XKpsk3 Pattern**: https://noiseexplorer.com/patterns/XKpsk3/
- **KCP Protocol**: https://github.com/skywind3000/kcp
- **Blake3**: https://github.com/BLAKE3-team/BLAKE3-specs

### Implementations

- **snow**: Rust Noise protocol implementation
- **RustCrypto**: Cryptographic primitives (ChaCha20-Poly1305, X25519)
- **tokio**: Async runtime for network I/O

### Security Audits

- [ ] Noise implementation audit (pending)
- [ ] Replay protection audit (pending)
- [ ] E-cash integration audit (pending)
- [ ] Penetration testing (pending)

## Changelog

### Version 1.1 (Post-Quantum PSK with KKT)

**Implemented:**
- KKTExchange state in state machine for pre-handshake KEM key transfer
- PSQ (Post-Quantum Secure PSK) protocol integration
- KKT (KEM Key Transfer) protocol with Ed25519 authentication
- Optional hash validation for KEM key pinning (signature-only mode active)
- PSK handle (ctxt_B) storage for future re-registration
- X25519 DHKEM support (ready for ML-KEM upgrade)
- Comprehensive state machine tests (7 test cases)
- generate_fresh_salt() utility for session creation

**Security Improvements:**
- Post-quantum PSK derivation (KEM-based)
- HNDL (Harvest Now, Decrypt Later) resistance
- Mutual authentication via Ed25519 signatures
- Easy migration path to ML-KEM-768

**Architecture:**
- State flow: ReadyToHandshake → KKTExchange → Handshaking → Transport
- PSQ embedded in Noise handshake (no extra round-trip)
- Automatic KKT on StartHandshake (no manual key distribution)

**Related Issues:**
- nym-4za: Add KKTExchange state to LpStateMachine

### Version 1.0 (Initial Implementation)

**Implemented:**
- Noise XKpsk3 handshake
- KCP reliability layer
- Replay protection (1024-packet window with SIMD)
- PSK derivation (ECDH + Blake3)
- dVPN and Mixnet registration modes
- E-cash credential verification
- WireGuard peer management
- Prometheus metrics
- DoS protection (connection limits, timestamp validation)

**Pending:**
- End-to-end integration tests
- Performance benchmarks
- Security audit
- Client implementation
- Gateway probe support
- Per-IP rate limiting
