# dVPN Protocol

This page describes the technical details of dVPN mode's protocol and encryption.

## Protocol Stack

dVPN mode combines WireGuard with additional layer encryption:

```
┌─────────────────────────────────────────────────────────────────┐
│                      Application Data                           │
├─────────────────────────────────────────────────────────────────┤
│                    Layer Encryption                             │
│              (Entry-to-Exit hop encryption)                     │
├─────────────────────────────────────────────────────────────────┤
│                      WireGuard                                  │
│              (Client-to-Entry encryption)                       │
├─────────────────────────────────────────────────────────────────┤
│                       UDP/IP                                    │
└─────────────────────────────────────────────────────────────────┘
```

## WireGuard Integration

The client-to-Entry Gateway connection uses the WireGuard protocol:

- **Key exchange**: Curve25519 (X25519)
- **Symmetric encryption**: ChaCha20-Poly1305
- **Hash function**: BLAKE2s
- **Authentication**: Pre-shared keys derived during connection setup

WireGuard provides:
- Fast handshake and reconnection
- Efficient encryption with low overhead
- Modern cryptographic primitives
- Minimal attack surface

## Layer Encryption

Beyond WireGuard, dVPN mode applies additional layer encryption for the Entry-to-Exit hop:

### Encryption Algorithms

The Nym Network uses the following encryption standards:

| Algorithm | Purpose |
|-----------|---------|
| AES-GCM-SIV-256 | Authenticated encryption with nonce-misuse resistance |
| AES-CTR-128 | Stream cipher for payload encryption |
| ChaCha20-Poly1305 | Authenticated encryption (WireGuard layer) |
| Curve25519 | Key exchange (X25519) and signatures (Ed25519) |

### Key Derivation

Keys for layer encryption are derived through:

1. ECDH key exchange between client and Exit Gateway
2. HKDF-based key derivation
3. Separate keys for each direction (client→exit, exit→client)

## Packet Format

All dVPN packets are padded to a uniform size:

```
┌────────────────────────────────────────────────────────────────┐
│ Header (routing info)                                          │
├────────────────────────────────────────────────────────────────┤
│ Encrypted Payload                                              │
│                                                                │
│ ┌────────────────────────────────────────────────────────────┐ │
│ │ Original packet data                                       │ │
│ ├────────────────────────────────────────────────────────────┤ │
│ │ Padding (to uniform size)                                  │ │
│ └────────────────────────────────────────────────────────────┘ │
├────────────────────────────────────────────────────────────────┤
│ Authentication tag                                             │
└────────────────────────────────────────────────────────────────┘
```

### Packet Padding

- All packets are padded to a fixed size
- Padding prevents packet-size fingerprinting
- Padding is removed at the Exit Gateway before forwarding

## Connection Lifecycle

### 1. Gateway Selection

Client selects Entry and Exit Gateways:
- Entry: Based on latency, location preference, or random selection
- Exit: Based on exit location requirements

### 2. Authentication

```
Client                    Entry Gateway                Nym API
   │                            │                         │
   │─── zk-nym credential ─────▶│                         │
   │                            │─── verify credential ──▶│
   │                            │◀── validity response ───│
   │◀── connection accepted ────│                         │
```

The zk-nym credential:
- Proves payment without revealing identity
- Is re-randomized for each connection
- Cannot be linked to previous usage

### 3. Tunnel Establishment

```
Client                    Entry Gateway              Exit Gateway
   │                            │                         │
   │─── WireGuard handshake ───▶│                         │
   │◀── WireGuard response ─────│                         │
   │                            │─── establish link ─────▶│
   │                            │◀── link confirmed ──────│
   │◀── tunnel ready ───────────│                         │
```

### 4. Data Transfer

```
Client                    Entry Gateway              Exit Gateway          Destination
   │                            │                         │                     │
   │─── encrypted packet ──────▶│                         │                     │
   │                            │─── re-encrypted ───────▶│                     │
   │                            │                         │─── plaintext ──────▶│
   │                            │                         │◀── response ────────│
   │                            │◀── encrypted ───────────│                     │
   │◀── decrypted response ─────│                         │                     │
```

## Security Properties

### Forward Secrecy

- New session keys are derived for each connection
- Compromise of long-term keys doesn't expose past sessions
- WireGuard's key rotation provides additional forward secrecy

### Split-Knowledge Architecture

| Node | Knows | Doesn't Know |
|------|-------|--------------|
| Entry Gateway | Client IP, Entry-Exit link | Destination, Payload content |
| Exit Gateway | Destination, Payload | Client IP |

### Replay Protection

- WireGuard provides replay protection via counters
- zk-nym credentials include replay protection via serial numbers

## Differences from Mixnet Mode

| Aspect | dVPN Mode | Mixnet Mode |
|--------|-----------|-------------|
| Hops | 2 | 5 |
| Packet format | Custom | Sphinx |
| Timing delays | None | Random exponential |
| Cover traffic | None | Continuous |
| Routing | Static per-session | Per-packet |

## Implementation Notes

The dVPN mode is implemented in the NymVPN application and is not available through the SDKs. The underlying protocol shares infrastructure with mixnet mode:

- Same Entry and Exit Gateways
- Same zk-nym credential system
- Same packet size (for traffic indistinguishability)

This ensures that external observers cannot distinguish between dVPN and mixnet users.
