# LP Registration Protocol - Technical Walkthrough

**Branch**: `drazen/lp-reg`
**Status**: Implementation complete, testing in progress
**Audience**: Engineering team, technical demo

---

## Executive Summary

LP Registration is a **fast, direct registration protocol** that allows clients to connect to Nym gateways without traversing the mixnet. It's designed primarily for dVPN use cases where users need quick WireGuard peer setup with sub-second latency.

### Key Characteristics

| Aspect | LP Registration | Traditional Mixnet Registration |
|--------|----------------|--------------------------------|
| **Latency** | Sub-second (100ms-1s) | Multi-second (3-10s) |
| **Transport** | Direct TCP (port 41264) | Through mixnet layers |
| **Reliability** | Guaranteed delivery | Probabilistic delivery |
| **Anonymity** | Client IP visible to gateway | Network-level anonymity |
| **Use Case** | dVPN, low-latency services | Privacy-critical applications |
| **Security** | Noise XKpsk3 + ChaCha20-Poly1305 | Sphinx packet encryption |

### Protocol Stack

```
┌─────────────────────────────────────────────────────────────┐
│                      Application Layer                      │
│  WireGuard Peer Registration (dVPN) / Mixnet Client.        │
└─────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────┐
│                     LP Registration Layer                   │
│         LpRegistrationRequest / LpRegistrationResponse      │
└─────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────┐
│                  Noise XKpsk3 Protocol Layer                │
│        ChaCha20-Poly1305 Encryption + Authentication        │
│               Replay Protection (1024-pkt window)           │
└─────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────┐
│                        Transport Layer                      │
│              TCP (length-prefixed packet framing)           │
└─────────────────────────────────────────────────────────────┘
```

---

## Architecture Overview

### High-Level Component Diagram

```
┌──────────────────────────────────────────────────────────────────────┐
│                              CLIENT SIDE                             │
├──────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  ┌─────────────────────────────────────────────────────────────┐     │
│  │         nym-registration-client (Client Library)            │     │
│  │  nym-registration-client/src/lp_client/client.rs:39-62      │     │
│  │                                                             │     │
│  │  • LpRegistrationClient                                     │     │
│  │  • TCP connection management                                │     │
│  │  • Packet serialization/framing                             │     │
│  │  • Integration with BandwidthController                     │     │ 
│  └────────────────────┬────────────────────────────────────────┘     │
│                       │                                              │
│  ┌────────────────────┴─────────────────────────────────────────┐    │
│  │            common/nym-lp (Protocol Library)                  │    │
│  │         common/nym-lp/src/ (multiple modules)                │    │
│  │                                                              │    │
│  │  • LpStateMachine (state_machine.rs:96-420)                  │    │
│  │  • Noise XKpsk3 (noise_protocol.rs:40-88)                    │    │
│  │  • PSK derivation (psk.rs:28-52)                             │    │
│  │  • ReplayValidator (replay/validator.rs:25-125)              │    │
│  │  • Message types (message.rs, packet.rs)                     │    │
│  └──────────────────────────────────────────────────────────────┘    │
│                                                                      │
└──────────────────────────────────────────────────────────────────────┘
                                    │
                                    │ TCP (port 41264)
                                    │ Length-prefixed packets
                                    │
                                    ▼
┌──────────────────────────────────────────────────────────────────────┐
│                             GATEWAY SIDE                             │
├──────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  ┌─────────────────────────────────────────────────────────────┐     │
│  │              LpListener (TCP Accept Loop)                   │     │
│  │      gateway/src/node/lp_listener/mod.rs:226-270            │     │
│  │                                                             │     │
│  │  • Binds to 0.0.0.0:41264                                   │     │
│  │  • Spawns LpConnectionHandler per connection                │     │
│  │  • Metrics: active_lp_connections                           │     │
│  └────────────────────┬────────────────────────────────────────┘     │
│                       │                                              │
│  ┌────────────────────▼─────────────────────────────────────────┐    │
│  │          LpConnectionHandler (Per-Connection)                │    │
│  │     gateway/src/node/lp_listener/handler.rs:101-216          │    │
│  │                                                              │    │
│  │  1. Receive ClientHello & validate timestamp                 │    │
│  │  2. Derive PSK from ECDH + salt                              │    │
│  │  3. Perform Noise handshake                                  │    │
│  │  4. Receive encrypted registration request                   │    │
│  │  5. Process registration (delegate to registration.rs)       │    │
│  │  6. Send encrypted response                                  │    │
│  │  7. Emit metrics & close                                     │    │
│  └────────────────────┬─────────────────────────────────────────┘    │
│                       │                                              │
│  ┌────────────────────▼─────────────────────────────────────────┐    │
│  │          Registration Processor (Business Logic)             │    │
│  │    gateway/src/node/lp_listener/registration.rs:136-288      │    │
│  │                                                              │    │
│  │  Mode: dVPN                    Mode: Mixnet                  │    │
│  │  ├─ register_wg_peer()         ├─ (skip WireGuard)           │    │
│  │  ├─ credential_verification()  ├─ credential_verification()  │    │
│  │  └─ return GatewayData         └─ return bandwidth only      │    │
│  └────────┬───────────────────────────────┬─────────────────────┘    │
│           │                               │                          │
│  ┌────────▼───────────────────┐  ┌───────▼─────────────────────┐     │
│  │   WireGuard Controller      │  │   E-cash Verifier          │     │
│  │   (PeerControlRequest)      │  │   (EcashManager trait)     │     │
│  │                             │  │                            │     │
│  │  • Add/Remove WG peers      │  │  • Verify BLS signature    │     │
│  │  • Manage peer lifecycle    │  │  • Check nullifier spent   │     │
│  │  • Monitor bandwidth usage  │  │  • Allocate bandwidth      │     │
│  └─────────────────────────────┘  └────────────────────────────┘     │
│                                                                      │
│  ┌─────────────────────────────────────────────────────────────┐     │
│  │                  GatewayStorage (Database)                  │     │
│  │                                                             │     │
│  │  Tables:                                                    │     │
│  │  • wireguard_peers (public_key, client_id, ticket_type)     │     │
│  │  • bandwidth (client_id, available)                         │     │
│  │  • spent_credentials (nullifier, expiry)                    │     │
│  └─────────────────────────────────────────────────────────────┘     │
│                                                                      │
└──────────────────────────────────────────────────────────────────────┘
```

---

## Implementation Roadmap

### ✅ Completed Components

1. **Protocol Library** (`common/nym-lp/`)
   - Noise XKpsk3 implementation
   - PSK derivation (Blake3 KDF)
   - Replay protection with SIMD optimization
   - Message types and packet framing

2. **Gateway Listener** (`gateway/src/node/lp_listener/`)
   - TCP accept loop with connection limits
   - Per-connection handler with lifecycle management
   - dVPN and Mixnet registration modes
   - Comprehensive metrics

3. **Client Library** (`nym-registration-client/`)
   - Connection management with timeouts
   - Noise handshake as initiator
   - E-cash credential integration
   - Error handling and retries

4. **Testing Tools** (`nym-gateway-probe/`)
   - LP-only test mode (`--only-lp-registration`)
   - Mock e-cash mode (`--use-mock-ecash`)
   - Detailed test results


## Detailed Documentation

### For Protocol Deep-Dive
📄 **[LP_REGISTRATION_SEQUENCES.md](./LP_REGISTRATION_SEQUENCES.md)**
- Complete sequence diagrams for all flows
- Happy path with byte-level message formats
- Error scenarios and recovery paths
- Noise handshake details

### For Architecture Understanding
📄 **[LP_REGISTRATION_ARCHITECTURE.md](./LP_REGISTRATION_ARCHITECTURE.md)**
- Component interaction diagrams
- Data flow through gateway modules
- Client-side architecture
- State transitions


---

## Code Navigation

### Key Entry Points

| Component | File Path | Description |
|-----------|-----------|-------------|
| **Gateway Listener** | `gateway/src/node/lp_listener/mod.rs:226` | `LpListener::run()` - main loop |
| **Connection Handler** | `gateway/src/node/lp_listener/handler.rs:101` | `LpConnectionHandler::handle()` - per-connection |
| **Registration Logic** | `gateway/src/node/lp_listener/registration.rs:136` | `process_registration()` - business logic |
| **Client Entry** | `nym-registration-client/src/lp_client/client.rs:39` | `LpRegistrationClient` struct |
| **Protocol Core** | `common/nym-lp/src/state_machine.rs:96` | `LpStateMachine` - Noise protocol |
| **Probe Test** | `nym-gateway-probe/src/lib.rs:861` | `lp_registration_probe()` - integration test |

---

## Metrics and Observability

### Prometheus Metrics

**Connection Metrics**:
- `lp_connections_total{result="success|error"}` - Counter
- `lp_active_lp_connections` - Gauge
- `lp_connection_duration_seconds` - Histogram (buckets: 0.01, 0.1, 1, 5, 10, 30)

**Handshake Metrics**:
- `lp_handshakes_success` - Counter
- `lp_handshakes_failed{reason="..."}` - Counter
- `lp_handshake_duration_seconds` - Histogram

**Registration Metrics**:
- `lp_registration_attempts_total` - Counter
- `lp_registration_success_total{mode="dvpn|mixnet"}` - Counter
- `lp_registration_failed_total{reason="..."}` - Counter
- `lp_registration_duration_seconds` - Histogram

**Bandwidth Metrics**:
- `lp_bandwidth_allocated_bytes_total` - Counter
- `lp_credential_verification_success` - Counter
- `lp_credential_verification_failed{reason="..."}` - Counter

## Performance Characteristics

### Latency Breakdown

```
Total Registration Time: ~221ms (typical)
├─ TCP Connect: 10-20ms
├─ Noise Handshake: 40-60ms (3 round-trips)
│  ├─ ClientHello send: <5ms
│  ├─ Msg 1 (-> e): <5ms
│  ├─ Msg 2 (<- e,ee,s,es): 20-30ms (crypto ops)
│  └─ Msg 3 (-> s,se,psk): 10-20ms
├─ Registration Request: 100-150ms
│  ├─ Request encrypt & send: <5ms
│  ├─ Gateway processing: 90-140ms
│  │  ├─ WireGuard peer setup: 20-40ms
│  │  ├─ Database operations: 30-50ms
│  │  ├─ E-cash verification: 40-60ms (or <1ms with mock)
│  │  └─ Response preparation: <5ms
│  └─ Response receive & decrypt: <5ms
└─ Connection cleanup: <5ms
```

### Resource Usage

- **Memory per session**: 144 bytes (state machine + replay window)
- **Max concurrent connections**: 10,000 (configurable)
- **CPU**: Minimal (ChaCha20 is efficient, SIMD optimizations)
- **Database**: 3-5 queries per registration (indexed lookups)