# Nym Lewes Protocol

The Lewes Protocol (LP) is a secure network communication protocol implemented in Rust. It provides authenticated, encrypted sessions with replay protection and supports nested session forwarding for privacy-preserving multi-hop connections.

## Architecture Overview

```
┌─────────────────┐     ┌────────────────┐     ┌───────────────┐
│ Transport Layer │◄───►│ LP Session     │◄───►│ LP Codec      │
│ (TCP)           │     │ - State machine│     │ - Serialize   │
└─────────────────┘     │ - Noise crypto │     │ - Deserialize │
                        │ - Replay prot. │     └───────────────┘
                        └────────────────┘
```

## Packet Structure

The protocol uses a length-prefixed packet format over TCP:

```
Wire Format:
┌────────────────────┬─────────────────────────────────────────┐
│ Length (4B BE u32) │ LpPacket                                │
└────────────────────┴─────────────────────────────────────────┘

LpPacket:
┌──────────────────┬───────────────────┬──────────────────┐
│ Header (16B)     │ Message           │ Trailer (16B)    │
├──────────────────┼───────────────────┼──────────────────┤
│ Version (1B)     │ Type (2B LE u16)  │ Reserved         │
│ Reserved (3B)    │ Content (var)     │ (16 bytes)       │
│ SessionID (4B LE)│                   │                  │
│ Counter (8B LE)  │                   │                  │
└──────────────────┴───────────────────┴──────────────────┘
```

- **Header**: Protocol version (1), session identifier, monotonic counter
- **Message**: Type discriminant + variable-length content
- **Trailer**: Reserved for future use (16 bytes)

## Message Types

| Type | Value | Purpose |
|------|-------|---------|
| `Busy` | 0x0000 | Server congestion signal |
| `Handshake` | 0x0001 | Noise protocol handshake messages |
| `EncryptedData` | 0x0002 | Encrypted application data |
| `ClientHello` | 0x0003 | Initial session negotiation |
| `KKTRequest` | 0x0004 | KEM Key Transfer request |
| `KKTResponse` | 0x0005 | KEM Key Transfer response |
| `ForwardPacket` | 0x0006 | Nested session forwarding |

## Session Establishment

### Session ID

Sessions are identified by a deterministic 32-bit ID computed from both parties' X25519 public keys:

```
session_id = make_lp_id(client_x25519_pub, gateway_x25519_pub)
```

The computation is order-independent, allowing both sides to derive the same ID independently.

**BOOTSTRAP_SESSION_ID (0)**: A special session ID used only for the initial `ClientHello` packet, since neither side can compute the final ID until both X25519 keys are known.

### Handshake Flow

```
┌────────┐                              ┌─────────┐
│ Client │                              │ Gateway │
└───┬────┘                              └────┬────┘
    │                                        │
    │ 1. ClientHello (session_id=0)          │
    │   [client_x25519, client_ed25519, salt]│
    │───────────────────────────────────────►│
    │                                        │ (computes session_id)
    │                                        │ (stores state machine)
    │                                        │
    │ 2. KKTRequest (session_id=N)           │
    │   [signed request for KEM key]         │
    │───────────────────────────────────────►│
    │                                        │
    │ 3. KKTResponse                         │
    │   [gateway KEM key + signature]        │
    │◄───────────────────────────────────────│
    │                                        │
    │ 4. Noise Handshake Msg 1               │
    │   [PSQ payload + noise message]        │
    │───────────────────────────────────────►│
    │                                        │ (derives PSK from PSQ)
    │ 5. Noise Handshake Msg 2               │
    │   [PSK handle + noise message]         │
    │◄───────────────────────────────────────│
    │                                        │
    │ 6. Noise Handshake Msg 3               │
    │───────────────────────────────────────►│
    │                                        │
    │ ═══════ Session Established ═══════    │
    │                                        │
    │ 7. EncryptedData                       │
    │   [encrypted application data]         │
    │◄──────────────────────────────────────►│
    │                                        │
```

### ClientHello Data

```rust
struct ClientHelloData {
    client_lp_public_key: [u8; 32],      // X25519 (derived from Ed25519)
    client_ed25519_public_key: [u8; 32], // For authentication
    salt: [u8; 32],                       // timestamp (8B) + nonce (24B)
}
```

## Packet-Per-Connection Model

The gateway processes **exactly one packet per TCP connection**, then closes. State persists between connections via in-memory maps:

```
TCP Connect → Receive Packet → Process → Send Response → TCP Close
```

**State Storage:**
- `handshake_states`: Maps `session_id → LpStateMachine` (during handshake)
- `session_states`: Maps `session_id → LpSession` (after handshake complete)

Both maps use TTL-based cleanup to remove stale entries (default: 5 min handshake, 1 hour session).

### Gateway Packet Routing

```
Packet Received
    │
    ├─► session_id == 0 (BOOTSTRAP)
    │   └─► handle_client_hello()
    │       └─► Create state machine, store in handshake_states
    │
    ├─► session_id in handshake_states
    │   └─► handle_handshake_packet()
    │       └─► Process KKT/Noise, move to session_states when complete
    │
    └─► session_id in session_states
        └─► handle_transport_packet()
            └─► Decrypt, process registration or forwarding
```

## Session Forwarding

Forwarding enables a client to establish an independent session with an exit gateway through an entry gateway, providing network-level privacy.

### Architecture

```
┌──────────┐
│  Client  │
└────┬─────┘
     │ Outer LP Session (established, encrypted)
     │
     ▼
┌────────────────┐
│ Entry Gateway  │  Sees: Client IP
│                │  Doesn't see: Exit destination
└────────┬───────┘
         │ Forwards inner packets (TCP)
         │
         ▼
┌────────────────┐
│ Exit Gateway   │  Sees: Entry Gateway IP
│                │  Doesn't see: Client IP
└────────────────┘
```

### ForwardPacket Message

```rust
struct ForwardPacketData {
    target_gateway_identity: [u8; 32], // Exit gateway's Ed25519 key
    target_lp_address: String,          // e.g., "2.2.2.2:41264"
    inner_packet_bytes: Vec<u8>,        // Complete LP packet for exit
}
```

### Forwarding Flow

1. **Client** establishes outer LP session with entry gateway
2. **Client** creates `ClientHello` packet for exit gateway
3. **Client** wraps inner packet in `ForwardPacketData`:
   - Sets `target_gateway_identity` to exit's Ed25519 key
   - Sets `target_lp_address` to exit's LP listener address
   - Serializes complete LP packet as `inner_packet_bytes`
4. **Client** encrypts `ForwardPacketData` using outer session
5. **Client** sends as `EncryptedData` to entry gateway

6. **Entry Gateway** decrypts, sees `ForwardPacketData`
7. **Entry Gateway** connects to exit gateway (new TCP)
8. **Entry Gateway** sends `inner_packet_bytes` directly
9. **Entry Gateway** receives exit's response
10. **Entry Gateway** encrypts response using outer session
11. **Entry Gateway** sends encrypted response to client

12. **Client** decrypts response, processes in inner session state

### NestedLpSession

The `NestedLpSession` struct manages the inner session from the client's perspective:

```rust
struct NestedLpSession {
    exit_identity: [u8; 32],           // Exit gateway Ed25519
    exit_address: String,               // Exit LP address
    client_keypair: Arc<ed25519::KeyPair>,
    exit_public_key: ed25519::PublicKey,
    state_machine: Option<LpStateMachine>,
}
```

**Usage:**
```rust
// Create nested session targeting exit gateway
let nested = NestedLpSession::new(exit_identity, exit_address, keypair, exit_pubkey);

// Perform handshake through outer session
nested.handshake_and_register(&mut outer_client).await?;

// Inner session now established with exit gateway
```

## State Machine States

```
ReadyToHandshake
       │
       ▼
   KKTExchange ◄─── KKTRequest/KKTResponse
       │
       ▼
   Handshaking ◄─── Noise messages + PSQ
       │
       ▼
    Transport ◄─── EncryptedData
       │
       ▼
     Closed
```

## Cryptography

### Key Types
- **Ed25519**: Identity keys, signing
- **X25519**: Key exchange (derived from Ed25519 via RFC 7748)

### Noise Protocol
- Pattern: `Noise_XKpsk3_25519_ChaChaPoly_SHA256`
- Provides: Forward secrecy, mutual authentication, PSK binding

### PSK Derivation (PSQ)
The Pre-Shared Key is derived via Post-Quantum Secure Key Exchange:
1. Client encapsulates using authenticated KEM key from KKT
2. Produces 32-byte PSK + ciphertext
3. Gateway decapsulates to derive same PSK
4. PSK injected into Noise at position 3

### Replay Protection

- **Monotonic counter**: Each packet has incrementing 64-bit counter
- **Sliding window**: Bitmap tracks received counters (1024 packet window)
- **SIMD optimized**: Branchless validation for constant-time operation

```rust
// Validation flow
validator.will_accept_branchless(counter) // Check before decrypt
validator.mark_did_receive_branchless(counter) // Mark after decrypt
```

## Sessions

### LpSession Fields
```rust
struct LpSession {
    id: u32,                    // Session identifier
    is_initiator: bool,         // Client or gateway role
    noise_state: NoiseState,    // Noise transport state
    kkt_state: KktState,        // KKT exchange progress
    psq_state: PsqState,        // PSQ handshake progress
    psk_handle: Option<Vec<u8>>,// PSK handle from responder
    sending_counter: AtomicU64, // Outgoing packet counter
    receiving_counter: Validator, // Replay protection
    psk_injected: AtomicBool,   // Safety: real PSK injected?
}
```

### PSK Safety
Sessions initialize with a dummy PSK. The `psk_injected` flag must be `true` before `encrypt_data()` or `decrypt_data()` will operate, preventing accidental use of the insecure dummy.

## File Structure

```
common/nym-lp/src/
├── lib.rs           # Module exports
├── message.rs       # LpMessage enum, ClientHelloData, ForwardPacketData
├── packet.rs        # LpPacket, LpHeader, BOOTSTRAP_SESSION_ID
├── codec.rs         # Serialization/deserialization
├── session.rs       # LpSession, cryptographic operations
├── state_machine.rs # LpStateMachine, state transitions
├── psk.rs           # PSK derivation utilities
└── error.rs         # Error types
```
