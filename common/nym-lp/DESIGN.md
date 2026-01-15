# LP Protocol Design

## Overview

The Lewes Protocol (LP) provides authenticated, encrypted sessions with replay protection. Key design principles:

1. **Unified packet structure** - Same format for all packet types
2. **Receiver index** - Client-proposed session identifier (replaces computed session_id)
3. **Opportunistic encryption** - Header authentication and payload encryption as soon as PSK is available
4. **WireGuard-inspired simplicity** - Minimal header, clear security model

## Packet Structure

### Unified Format (v2)

All packets share the same outer structure - cleartext fields are always first:

```
┌────────────────┬─────────┬─────────┬──────────┬─────────────────────┬─────────┐
│ receiver_index │ counter │ version │ reserved │ payload             │ trailer │
│ 4B             │ 8B      │ 1B      │ 3B       │ variable            │ 16B     │
└────────────────┴─────────┴─────────┴──────────┴─────────────────────┴─────────┘
│←── 12B outer header ────┤│←── inner (cleartext or encrypted) ──────┤│─ 16B ──┤
```

**Total overhead:** 32 bytes (12B outer + 4B inner prefix + 16B trailer)

Key properties:
- **Outer header** (12 bytes): Always cleartext, used for routing before session lookup
- **Inner content**: Cleartext before PSK, encrypted after PSK
- **No disambiguation needed**: Format is identical for both modes

### Field Descriptions

**Outer Header** (always cleartext, 12 bytes):

| Field | Size | Description |
|-------|------|-------------|
| receiver_index | 4 bytes | Session identifier, proposed by client (routing key) |
| counter | 8 bytes | Monotonic counter, used as AEAD nonce and for replay protection |

**Inner Content** (cleartext or encrypted):

| Field | Size | Description |
|-------|------|-------------|
| version | 1 byte | Protocol version |
| reserved | 3 bytes | Reserved for future use |
| payload | variable | Message type (2B) + content |
| trailer | 16 bytes | Zeros (no PSK) or AEAD Poly1305 tag (with PSK) |

### Wire Format

Length-prefixed over TCP:

```
┌────────────────────┬─────────────────────────────────────────────────────┐
│ length (4B BE u32) │ LpPacket                                            │
└────────────────────┴─────────────────────────────────────────────────────┘
```

## Message Types

| Type | Value | Description |
|------|-------|-------------|
| Busy | 0x0000 | Server congestion signal |
| Handshake | 0x0001 | Noise protocol messages |
| EncryptedData | 0x0002 | Encrypted application data |
| ClientHello | 0x0003 | Initial session setup |
| KKTRequest | 0x0004 | KEM key transfer request |
| KKTResponse | 0x0005 | KEM key transfer response |
| ForwardPacket | 0x0006 | Nested session forwarding |
| Collision | 0x0007 | Receiver index collision |
| Ack | 0x0008 | Gateway confirms receipt of message |

### Planned Message Types (not yet implemented)

| Type | Value | Description |
|------|-------|-------------|
| SubsessionRequest | 0x0009 | Client requests new subsession |
| SubsessionKK1 | 0x000A | KK handshake msg 1 (responder → initiator) |
| SubsessionKK2 | 0x000B | KK handshake msg 2 (initiator → responder) |
| SubsessionReady | 0x000C | Subsession established confirmation |

## Receiver Index

### Assignment

The client generates a random 4-byte receiver_index and includes it in ClientHello. The gateway uses this as the session lookup key. This replaces the previous approach of computing a deterministic session_id from both parties' keys.

### Collision Handling

With 4 bytes (2^32 values), collision probability is negligible:

| Active Sessions | Collision Probability |
|-----------------|----------------------|
| 10,000 | ~0.001% |
| 100,000 | ~0.1% |

If collision detected, gateway rejects ClientHello and client retries with new index.

## Opportunistic Encryption

### Principle

As soon as PSK is derived (after processing Noise msg 1 with PSQ), all subsequent packets use outer AEAD encryption:

- **Header**: Authenticated as associated data (AD)
- **Payload**: Encrypted (message type + content)
- **Trailer**: AEAD tag

### Timeline

| Packet | PSK Available | Header | Payload | Trailer |
|--------|---------------|--------|---------|---------|
| ClientHello | No | Clear | Clear | Zeros |
| Ack | No | Clear | Clear | Zeros |
| KKTRequest | No | Clear | Clear | Zeros |
| KKTResponse | No | Clear | Clear | Zeros |
| Noise msg 1 | No | Clear | Clear | Zeros |
| | | **PSK derived** | | |
| Noise msg 2 | Yes | Authenticated | Encrypted | Tag |
| Noise msg 3 | Yes | Authenticated | Encrypted | Tag |
| Data | Yes | Authenticated | Encrypted | Tag |

### Encryption Scheme

- **AEAD**: ChaCha20-Poly1305
- **Key**: outer_key = KDF(PSK, "lp-outer-aead") - derived from PSK, not PSK itself
- **Nonce**: counter (8 bytes, zero-padded to 12 bytes)
- **AAD**: receiver_index ‖ counter (12 bytes) - the outer header
- **Encrypted**: version ‖ reserved ‖ message_type ‖ content

Note: PSK is used as-is for Noise (which does internal key derivation). The outer_key derivation avoids key reuse between the two encryption layers.

### Before PSK

```
┌────────────────┬─────────┬─────────┬──────────┬─────────────────────┬─────────┐
│ receiver_index │ counter │ version │ reserved │ payload             │ 00...00 │
│                │         │         │          │ (plaintext)         │         │
└────────────────┴─────────┴─────────┴──────────┴─────────────────────┴─────────┘
│←── 12B outer ──────────┤│←────────────── cleartext inner ──────────┤│─zeros──┤
```

### After PSK

```
┌────────────────┬─────────┬─────────┬──────────┬─────────────────────┬─────────┐
│ receiver_index │ counter │ version │ reserved │ payload             │ tag     │
│                │         │ (enc)   │ (enc)    │ (encrypted)         │         │
└────────────────┴─────────┴─────────┴──────────┴─────────────────────┴─────────┘
│←── 12B outer (AAD) ────┤│←────────── encrypted inner ──────────────┤│─ tag ──┤
```

## Handshake Flow

Each arrow represents a separate TCP connection (packet-per-connection model).

```
Client                                  Gateway
   │                                       │
   │ [hdr][ClientHello][zeros]             │
   │──────────────────────────────────────►│ store state[receiver_index]
   │                                       │
   │ [hdr][Ack][zeros]                     │
   │◄──────────────────────────────────────│ confirm ClientHello
   │                                       │
   │ [hdr][KKTRequest][zeros]              │
   │──────────────────────────────────────►│
   │                                       │
   │ [hdr][KKTResponse][zeros]             │
   │◄──────────────────────────────────────│
   │                                       │
   │ [hdr][Noise1+PSQ][zeros]              │
   │──────────────────────────────────────►│ derive PSK
   │                                       │
   │ [hdr][encrypted Noise2][tag]          │ ← authenticated
   │◄──────────────────────────────────────│
   │                                       │
   │ [hdr][encrypted Noise3][tag]          │ ← authenticated
   │──────────────────────────────────────►│
   │                                       │
   │ ════════ Session Established ═════════│
   │                                       │
   │ [hdr][encrypted Data][tag]            │
   │◄─────────────────────────────────────►│
```

## Data Packet Encryption

Data packets have two encryption layers:

```
Application Data
       │
       ▼
┌─────────────────────┐
│ Noise encrypt       │  Inner layer (forward secrecy, ratcheting)
│ (session keys)      │
└─────────────────────┘
       │
       ▼
┌─────────────────────┐
│ PSK AEAD            │  Outer layer (header auth, payload encryption)
│ (pre-shared key)    │
└─────────────────────┘
       │
       ▼
Wire: [header][encrypted payload][tag]
```

### What Outer AEAD Encrypts

The outer AEAD encrypts: message_type (2B) + message content

This hides the message type from observers after PSK is available.

## Subsessions and Rekeying

Subsessions enable **forward secrecy** through periodic rekeying and **channel multiplexing** for independent encrypted streams.

### Design Principles

| Aspect | Decision | Rationale |
|--------|----------|-----------|
| Key derivation | Noise KK handshake | Clean crypto, both parties already authenticated |
| Initiation channel | Tunneled through parent | Already authenticated, no proof-of-ownership needed |
| Hierarchy | Promotion model (chain) | Simpler than tree, natural for rekeying |
| Old session after promotion | Read-only until TTL | Drains in-flight packets, provides grace period |

### Noise KK Pattern

Subsessions use `Noise_KK_25519_ChaChaPoly_SHA256`:

- **KK** = Both parties already know each other's static keys
- **2 messages** to complete (vs 3 for XKpsk3)
- **No PSK needed** - already authenticated via parent session

### Promotion Model

When a subsession is created, it becomes the new "master" and the old session becomes read-only:

```
Session A (master) → Session B created → A demoted, B is master
                                         A: read-only until TTL
```

This creates a chain (A → B → C) but maintains only one level of nesting conceptually. Each promotion replaces the previous master.

### Protocol Flow

```
Client                              Gateway
  │                                   │
  │═══════ Parent Session (A) ════════│  Transport mode
  │                                   │
  │──[SubsessionRequest{idx=B}]──────►│  Encrypted in parent
  │                                   │  Gateway creates KK responder
  │◄──[SubsessionKK1{idx=B, e}]───────│  KK handshake msg 1
  │──[SubsessionKK2{idx=B, e,ee,se}]─►│  KK handshake msg 2
  │◄──[SubsessionReady{idx=B}]────────│  Subsession established
  │                                   │
  │  Session A: read-only (receive)   │
  │═══════ Session B (new master) ════│  New Transport mode
```

### Session State Transitions

```
Parent Session (A):
  Transport → ReadOnlyTransport (on subsession creation)
  ReadOnlyTransport → (expires via TTL cleanup)

Subsession (B):
  (created) → KKHandshaking → Transport (becomes new master)
```

### Read-Only Session Semantics

After demotion:
- **Can receive**: Decrypt and process incoming packets (drain in-flight)
- **Cannot send**: Encryption blocked, returns error
- **Cleaned up**: Via normal TTL expiration

### Message Formats

```rust
SubsessionRequestData {
    new_receiver_index: u32,  // Client-proposed index for subsession
}

SubsessionKK1Data {
    new_receiver_index: u32,
    kk_message: Vec<u8>,      // Noise KK message 1
}

SubsessionKK2Data {
    new_receiver_index: u32,
    kk_message: Vec<u8>,      // Noise KK message 2
}

SubsessionReadyData {
    new_receiver_index: u32,
}
```

### Counter Independence

- Each session has independent counters
- Subsession starts at counter 0
- No counter coordination needed between parent and subsession

### Failure Handling

| Scenario | Action |
|----------|--------|
| KK handshake fails | Discard attempt, keep using parent |
| Receiver index collision | Retry with new receiver_index |
| Parent session not found | Return error, client reconnects |

### Security Benefits

1. **Forward secrecy**: Compromise of current keys doesn't expose past traffic
2. **Key rotation**: Periodic rekeying limits exposure window
3. **Channel isolation**: Independent streams can't cross-decrypt

## Security Properties

### Always Visible to Observer

Only the outer header (12 bytes) is visible after PSK establishment:

- Receiver index (4 bytes) - opaque, unlinkable to identity
- Counter (8 bytes) - reveals packet ordering
- Packet size

Note: Before PSK, version, reserved, and message type are also visible.

### Protected After PSK

- Outer header integrity (authenticated via AEAD AAD)
- Inner content confidentiality (encrypted):
  - Protocol version
  - Reserved field
  - Message type
  - Payload
- Application data (double encrypted: outer AEAD + inner Noise)

### Cryptographic Guarantees

| Property | Mechanism |
|----------|-----------|
| Confidentiality | ChaCha20 (outer) + Noise ChaCha20 (inner) |
| Integrity | Poly1305 (outer) + Noise Poly1305 (inner) |
| Replay protection | Counter validation (before decryption) |
| Forward secrecy | Noise session keys (inner) + subsession rekeying |
| Header authentication | AEAD associated data |
| Key rotation | Periodic subsession creation (Noise KK) |

## References

- WireGuard Protocol - Inspiration for receiver_index and packet simplicity
- Noise Protocol Framework - Inner encryption layer, KK pattern for subsessions
- RFC 8439 ChaCha20-Poly1305 - AEAD cipher
- Noise Explorer KK - https://noiseexplorer.com/patterns/KK/
