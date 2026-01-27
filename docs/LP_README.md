# Lewes Protocol (LP) - Fast Gateway Registration

## What is LP?

The Lewes Protocol (LP) is a direct TCP-based registration protocol for Nym gateways. It provides an alternative to mixnet-based registration with different trade-offs.

**Trade-offs:**
- **Faster**: Direct TCP connection vs multi-hop mixnet routing (fewer hops = lower latency)
- **Less Anonymous**: Client IP visible to gateway (mixnet hides IP)
- **More Reliable**: KCP provides ordered delivery with fast retransmission
- **Secure**: Noise XKpsk3 provides mutual authentication and forward secrecy

**Use LP when:**
- Fast registration is important
- Network anonymity is not required for the registration step
- You want reliable, ordered delivery

**Use mixnet registration when:**
- Network-level anonymity is essential
- IP address hiding is required
- Traffic analysis resistance is critical

## Quick Start

### For Gateway Operators

```bash
# 1. Enable LP in gateway config
cat >> ~/.nym/gateways/<id>/config/config.toml << EOF
[lp]
enabled = true
bind_address = "0.0.0.0"
control_port = 41264
max_connections = 10000
timestamp_tolerance_secs = 30
EOF

# 2. Open firewall
sudo ufw allow 41264/tcp

# 3. Restart gateway
systemctl restart nym-gateway

# 4. Verify LP listener
sudo netstat -tlnp | grep 41264
curl http://localhost:8080/metrics | grep lp_connections_total
```

### For Client Developers

```rust
use nym_registration_client::{RegistrationClient, RegistrationMode};

// Initialize client
let client = RegistrationClient::builder()
    .gateway_identity("gateway-identity-key")
    .gateway_lp_public_key(gateway_lp_pubkey)  // From gateway descriptor
    .gateway_lp_address("gateway-ip:41264")
    .mode(RegistrationMode::Lp)
    .build()?;

// Register with dVPN mode
let result = client.register_lp(
    credential,
    RegistrationMode::Dvpn {
        wg_public_key: client_wg_pubkey,
    }
).await?;

match result {
    LpRegistrationResult::Success { gateway_data, bandwidth_allocated, .. } => {
        // Use gateway_data to configure WireGuard tunnel
    }
    LpRegistrationResult::Error { code, message } => {
        eprintln!("Registration failed: {} (code: {})", message, code);
    }
}
```

## Architecture

```
┌─────────────────────────────────────────┐
│  Application                            │
│  - Registration Request                 │
│  - E-cash Verification                  │
│  - WireGuard Setup                      │
└─────────────────────────────────────────┘
                  ↓
┌─────────────────────────────────────────┐
│  LP Layer                               │
│  - Noise XKpsk3 Handshake               │
│  - Replay Protection (1024 packets)     │
│  - Counter-based Sequencing             │
└─────────────────────────────────────────┘
                  ↓
┌─────────────────────────────────────────┐
│  KCP Layer                              │
│  - Ordered Delivery                     │
│  - Fast Retransmission                  │
│  - Congestion Control                   │
└─────────────────────────────────────────┘
                  ↓
┌─────────────────────────────────────────┐
│  TCP                                    │
│  - Connection-oriented                  │
│  - Byte Stream                          │
└─────────────────────────────────────────┘
```

### Why This Stack?

**TCP**: Reliable connection establishment, handles network-level packet loss.

**KCP**: Application-level reliability optimized for low latency:
- Fast retransmit after 2 duplicate ACKs (vs TCP's 3)
- Selective acknowledgment (better than TCP's cumulative ACK)
- Minimum RTO of 100ms (configurable, vs TCP's typical 200ms+)

**LP**: Cryptographic security:
- **Noise XKpsk3**: Mutual authentication + forward secrecy
- **Replay Protection**: 1024-packet sliding window
- **Session Isolation**: Each registration has unique crypto state

**Application**: Credential verification and peer registration logic.

## Key Features

### Security

**Cryptographic Primitives:**
- **Noise XKpsk3**: Mutual authentication with PSK
- **ChaCha20-Poly1305**: Authenticated encryption
- **X25519**: Key exchange
- **Blake3**: KDF for PSK derivation

**Security Properties:**
- Mutual authentication (both client and gateway prove identity)
- Forward secrecy (past sessions remain secure if keys compromised)
- Replay protection (1024-packet sliding window with SIMD optimization)
- Timestamp validation (30-second window, configurable)

### Observability

**Prometheus metrics** (from `gateway/src/node/lp_listener/mod.rs:4`):
- Connection counts and durations
- Handshake success/failure rates
- Registration outcomes (dVPN vs Mixnet)
- Credential verification results
- Error categorization
- Latency histograms

### DoS Protection

From `gateway/src/node/lp_listener/mod.rs`:
- **Connection limits**: Configurable max concurrent connections (default: 10,000)
- **Timestamp validation**: Rejects messages outside configured window (default: 30s)
- **Replay protection**: Prevents packet replay attacks

## Components

### Core Modules

| Module | Path | Purpose |
|--------|------|---------|
| **nym-lp** | `common/nym-lp/` | Core LP protocol implementation |
| **nym-kcp** | `common/nym-kcp/` | KCP reliability protocol |
| **lp_listener** | `gateway/src/node/lp_listener/` | Gateway-side LP listener |

### Key Files

**Protocol:**
- `common/nym-lp/src/noise_protocol.rs` - Noise state machine
- `common/nym-lp/src/replay/validator.rs` - Replay protection
- `common/nym-lp/src/psk.rs` - PSK derivation
- `common/nym-lp/src/session.rs` - LP session management

**KCP:**
- `common/nym-kcp/src/session.rs` - KCP state machine
- `common/nym-kcp/src/packet.rs` - KCP packet format

**Gateway:**
- `gateway/src/node/lp_listener/mod.rs` - TCP listener
- `gateway/src/node/lp_listener/handler.rs` - Connection handler
- `gateway/src/node/lp_listener/handshake.rs` - Noise handshake
- `gateway/src/node/lp_listener/registration.rs` - Registration logic

## Protocol Flow

### 1. Connection Establishment

```
Client                    Gateway
  |--- TCP SYN ------------> |
  |<-- TCP SYN-ACK --------- |
  |--- TCP ACK ------------> |
```

Port: 41264 (default, configurable)

### 2. Session Setup

```rust
// Client generates session parameters
let salt = [timestamp (8 bytes) || nonce (24 bytes)];
let shared_secret = ECDH(client_lp_private, gateway_lp_public);
let psk = Blake3_derive_key("nym-lp-psk-v1", shared_secret, salt);

// Deterministic session IDs (order-independent)
let lp_id = hash(client_pub || 0xCC || gateway_pub) & 0xFFFFFFFF;
let kcp_conv = hash(client_pub || 0xFF || gateway_pub) & 0xFFFFFFFF;
```

### 3. Noise Handshake (XKpsk3)

```
Client                         Gateway
  |--- e ------------------------>| [1] Client ephemeral
  |<-- e, ee, s, es -------------| [2] Gateway ephemeral + static
  |--- s, se, psk -------------->| [3] Client static + PSK
  [Transport mode established]
```

**Handshake characteristics:**
- 3 messages (1.5 round trips minimum)
- Cryptographic operations: ECDH, ChaCha20-Poly1305, SHA-256

### 4. Registration

```
Client                         Gateway
  |--- RegistrationRequest ------>| (encrypted)
  |                               | [Verify credential]
  |                               | [Register WireGuard peer if dVPN]
  |<-- RegistrationResponse ------| (encrypted)
```

### 5. Connection Close

After successful registration, connection is closed. LP is registration-only.

## Configuration

### Gateway

```toml
# ~/.nym/gateways/<id>/config/config.toml

[lp]
enabled = true
bind_address = "0.0.0.0"
control_port = 41264
data_port = 51264        # Reserved, not currently used
max_connections = 10000
timestamp_tolerance_secs = 30
use_mock_ecash = false   # TESTING ONLY!
```

### Environment Variables

```bash
RUST_LOG=nym_gateway::node::lp_listener=debug
LP_ENABLED=true
LP_CONTROL_PORT=41264
LP_MAX_CONNECTIONS=20000
```

## Monitoring

### Key Metrics

**Connections:**
```promql
nym_gateway_active_lp_connections
rate(nym_gateway_lp_connections_total[5m])
rate(nym_gateway_lp_connections_completed_with_error[5m])
```

**Handshakes:**
```promql
rate(nym_gateway_lp_handshakes_success[5m])
rate(nym_gateway_lp_handshakes_failed[5m])
histogram_quantile(0.95, nym_gateway_lp_handshake_duration_seconds)
```

**Registrations:**
```promql
rate(nym_gateway_lp_registration_success_total[5m])
rate(nym_gateway_lp_registration_dvpn_success[5m])
rate(nym_gateway_lp_registration_mixnet_success[5m])
histogram_quantile(0.95, nym_gateway_lp_registration_duration_seconds)
```

### Recommended Alerts

```yaml
- alert: LPHighRejectionRate
  expr: rate(nym_gateway_lp_connections_completed_with_error[5m]) > 10
  for: 5m

- alert: LPHandshakeFailures
  expr: rate(nym_gateway_lp_handshakes_failed[5m]) / rate(nym_gateway_lp_handshakes_success[5m]) > 0.05
  for: 10m
```

## Testing

### Unit Tests

```bash
# Run all LP tests
cargo test -p nym-lp
cargo test -p nym-kcp

# Specific suites
cargo test -p nym-lp replay
cargo test -p nym-kcp session
```

**Test Coverage** (from code):

| Component | Tests | Focus Areas |
|-----------|-------|-------------|
| Replay Protection | 14 | Edge cases, concurrency, overflow |
| KCP Session | 12 | Out-of-order, retransmit, window |
| PSK Derivation | 5 | Determinism, symmetry, salt |
| LP Session | 10 | Handshake, encrypt/decrypt |

### Missing Tests

- [ ] End-to-end registration flow
- [ ] Network failure scenarios
- [ ] Credential verification integration
- [ ] Load testing (concurrent connections)
- [ ] Performance benchmarks

## Troubleshooting

### Connection Refused

```bash
# Check listener
sudo netstat -tlnp | grep 41264

# Check config
grep "lp.enabled" ~/.nym/gateways/<id>/config/config.toml

# Check firewall
sudo ufw status | grep 41264
```

### Handshake Failures

```bash
# Check logs
journalctl -u nym-gateway | grep "handshake.*failed"

# Common causes:
# - Wrong gateway LP public key
# - Clock skew > 30s (check with: timedatectl)
# - Replay detection (retry with fresh connection)
```

### High Rejection Rate

```bash
# Check metrics
curl http://localhost:8080/metrics | grep lp_connections_completed_with_error

# Check connection limit
curl http://localhost:8080/metrics | grep active_lp_connections
```

See [LP_DEPLOYMENT.md](./LP_DEPLOYMENT.md#troubleshooting) for detailed guide.

## Security

### Threat Model

**Protected Against:**
- ✅ Passive eavesdropping (Noise encryption)
- ✅ Active MITM (mutual authentication)
- ✅ Replay attacks (counter-based validation)
- ✅ Packet injection (Poly1305 MAC)
- ✅ DoS (connection limits, timestamp validation)

**Not Protected Against:**
- ❌ Network-level traffic analysis (IP visible)
- ❌ Gateway compromise (sees registration data)
- ⚠️ Per-IP DoS (global limit only, not per-IP)

**Key Properties:**
- **Forward Secrecy**: Past sessions secure if keys compromised
- **Mutual Authentication**: Both parties prove identity
- **Replay Protection**: 1024-packet sliding window (verified: 144 bytes memory)
- **Constant-Time**: Replay checks are branchless (timing-attack resistant)

See [LP_SECURITY.md](./LP_SECURITY.md) for complete security analysis.

### Known Limitations

1. **No network anonymity**: Client IP visible to gateway
2. **Not quantum-resistant**: X25519 vulnerable to Shor's algorithm
3. **Single-use sessions**: No session resumption
4. **No per-IP rate limiting**: Only global connection limit

## Implementation Status

### Implemented ✅

- Noise XKpsk3 handshake
- KCP reliability layer
- Replay protection (1024-packet window with SIMD)
- PSK derivation (ECDH + Blake3)
- dVPN and Mixnet registration modes
- E-cash credential verification
- WireGuard peer management
- Prometheus metrics
- DoS protection

### Pending ⏳

- End-to-end integration tests
- Performance benchmarks
- External security audit
- Client implementation
- Gateway probe support
- Per-IP rate limiting

## Documentation

- **[LP_PROTOCOL.md](./LP_PROTOCOL.md)**: Complete protocol specification
- **[LP_DEPLOYMENT.md](./LP_DEPLOYMENT.md)**: Deployment and operations guide
- **[LP_SECURITY.md](./LP_SECURITY.md)**: Security analysis and threat model
- **[CODEMAP.md](../CODEMAP.md)**: Repository structure

## Contributing

### Getting Started

1. Read [CODEMAP.md](../CODEMAP.md) for repository structure
2. Review [LP_PROTOCOL.md](./LP_PROTOCOL.md) for protocol details
3. Check [FUNCTION_LEXICON.md](../FUNCTION_LEXICON.md) for API reference

### Areas Needing Work

**High Priority:**
- Integration tests for end-to-end registration
- Performance benchmarks (latency, throughput, concurrent connections)
- Per-IP rate limiting
- Client-side implementation

**Medium Priority:**
- Gateway probe support
- Load testing framework
- Fuzzing for packet parsers

## License

Same as parent Nym repository.

## Support

- **GitHub Issues**: https://github.com/nymtech/nym/issues
- **Discord**: https://discord.gg/nym

---

**Protocol Version**: 1.0
**Status**: Draft (pending security audit and integration tests)
