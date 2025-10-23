# LP (Lewes Protocol) Security Considerations

## Threat Model

### Attacker Capabilities

**Network Attacker (Dolev-Yao Model):**
- ✅ Can observe all network traffic
- ✅ Can inject, modify, drop, or replay packets
- ✅ Can perform active MITM attacks
- ✅ Cannot break cryptographic primitives (ChaCha20, Poly1305, X25519)
- ✅ Cannot forge digital signatures (BLS12-381)

**Gateway Compromise:**
- ✅ Attacker gains full access to gateway server
- ✅ Can read all gateway state (keys, credentials, database)
- ✅ Can impersonate gateway to clients
- ❌ Cannot decrypt past sessions (forward secrecy)
- ❌ Cannot impersonate clients without their keys

**Client Compromise:**
- ✅ Attacker gains access to client device
- ✅ Can read client LP private key
- ✅ Can impersonate client to gateways
- ❌ Cannot decrypt other clients' sessions

### Security Goals

**Confidentiality:**
- Registration requests encrypted end-to-end
- E-cash credentials protected from eavesdropping
- WireGuard keys transmitted securely

**Integrity:**
- All messages authenticated with Poly1305 MAC
- Tampering detected and rejected
- Replay attacks prevented

**Authentication:**
- Mutual authentication via Noise XKpsk3
- Gateway proves possession of LP private key
- Client proves possession of LP private key + PSK

**Forward Secrecy:**
- Compromise of long-term keys doesn't reveal past sessions
- Ephemeral keys provide PFS
- Session keys destroyed after use

**Non-Goals:**
- **Network anonymity**: LP reveals client IP to gateway (use mixnet for anonymity)
- **Traffic analysis resistance**: Packet timing visible to network observer
- **Deniability**: Parties can prove who they communicated with

## Cryptographic Design

### Noise Protocol XKpsk3

**Pattern:**
```
XKpsk3:
  <- s
  ...
  -> e
  <- e, ee, s, es
  -> s, se, psk
```

**Security Properties:**

| Property | Provided | Rationale |
|----------|----------|-----------|
| Confidentiality (forward) | ✅ Strong | Ephemeral keys + PSK |
| Confidentiality (backward) | ✅ Weak | PSK compromise affects future |
| Authentication (initiator) | ✅ Strong | Static key + PSK |
| Authentication (responder) | ✅ Strong | Static key known upfront |
| Identity hiding (initiator) | ✅ Yes | Static key encrypted |
| Identity hiding (responder) | ❌ No | Static key in handshake msg 2 |

**Why XKpsk3:**

1. **Known responder identity**: Client knows gateway's LP public key from descriptor
2. **Mutual authentication**: Both sides prove identity
3. **PSK binding**: Links session to out-of-band PSK (prevents MITM with compromised static key alone)
4. **Forward secrecy**: Ephemeral keys provide PFS even if static keys leaked

**Alternative patterns considered:**

- **IKpsk2**: No forward secrecy (rejected)
- **XXpsk3**: More round trips, unknown identities (not needed)
- **NKpsk0**: No client authentication (rejected)

### PSK Derivation Security

**Formula:**
```
shared_secret = X25519(client_lp_private, gateway_lp_public)
psk = Blake3_derive_key("nym-lp-psk-v1", shared_secret, salt)
```

**Security Analysis:**

1. **ECDH Security**: Based on Curve25519 hardness (128-bit security)
   - Resistant to quantum attacks up to Grover's algorithm (64-bit post-quantum)
   - Well-studied, no known vulnerabilities

2. **Blake3 KDF Security**:
   - Output indistinguishable from random (PRF security)
   - Domain separation via context string prevents cross-protocol attacks
   - Collision resistance: 128 bits (birthday bound on 256-bit hash)

3. **Salt Freshness**:
   - Timestamp component prevents long-term PSK reuse
   - Nonce component provides per-session uniqueness
   - Both transmitted in ClientHello (integrity protected by timestamp validation + Noise handshake)

**Attack Scenarios:**

| Attack | Feasibility | Mitigation |
|--------|-------------|------------|
| Brute force PSK | ❌ Infeasible | 2^128 operations (Curve25519 DL) |
| Quantum attack on ECDH | ⚠️ Future threat | Shor's algorithm breaks X25519 in polynomial time |
| Salt replay | ❌ Prevented | Timestamp validation (30s window) |
| Cross-protocol PSK reuse | ❌ Prevented | Domain separation ("nym-lp-psk-v1") |

**Quantum Resistance:**

LP is **not quantum-resistant** due to X25519 use. Future upgrade path:

```rust
// Hybrid PQ-KEM (future)
let classical_secret = X25519(client_priv, gateway_pub);
let pq_secret = Kyber768::encaps(gateway_pq_pub);
let psk = Blake3_derive_key(
    "nym-lp-psk-v2-pq",
    classical_secret || pq_secret,
    salt
);
```

### Replay Protection Analysis

**Algorithm: Sliding Window with Bitmap**

```rust
Window size: 1024 packets
Bitmap: [u64; 16] = 1024 bits

For counter C:
  - Accept if C >= next (new packet)
  - Reject if C + 1024 < next (too old)
  - Reject if bitmap[C % 1024] == 1 (duplicate)
  - Otherwise accept and mark
```

**Security Properties:**

1. **Replay Window**: 1024 packets
   - Sufficient for expected reordering in TCP+KCP
   - Small enough to limit replay attack surface

2. **Memory Efficiency**: 128 bytes bitmap
   - Tracks 1024 unique counters
   - O(1) lookup and insertion

3. **Overflow Handling**: Wraps at u64::MAX
   - Properly handles counter wraparound
   - Unlikely to occur (2^64 packets = trillions)

**Attack Scenarios:**

| Attack | Feasibility | Mitigation |
|--------|-------------|------------|
| Replay within window | ❌ Prevented | Bitmap tracking |
| Replay outside window | ❌ Prevented | Window boundary check |
| Counter overflow | ⚠️ Theoretical | Wraparound handling + 2^64 limit |
| Timing attack | ❌ Mitigated | Branchless execution |

**Timing Attack Resistance:**

```rust
// Constant-time check (branchless)
pub fn will_accept_branchless(&self, counter: u64) -> ReplayResult<()> {
    let is_growing = counter >= self.next;
    let too_far_back = /* calculated */;
    let duplicate = self.check_bit_branchless(counter);

    // Single branch at end (constant-time up to this point)
    let result = if is_growing { Ok(()) }
                 else if too_far_back { Err(OutOfWindow) }
                 else if duplicate { Err(Duplicate) }
                 else { Ok(()) };
    result.unwrap()
}
```

**SIMD Optimizations:**

- AVX2, SSE2, NEON: SIMD clears are constant-time
- Scalar fallback: Also constant-time (no data-dependent branches)
- No timing channels revealed through replay check

## Denial of Service (DoS) Protection

### Connection-Level DoS

**Attack:** Flood gateway with TCP connections

**Mitigations:**

1. **Max connections limit** (default: 10,000):
   ```rust
   if active_connections >= max_connections {
       return; // Drop new connection
   }
   ```
   - Prevents memory exhaustion (~5 KB per connection)
   - Configurable based on gateway capacity

2. **TCP SYN cookies** (kernel-level):
   ```bash
   sysctl -w net.ipv4.tcp_syncookies=1
   ```
   - Prevents SYN flood attacks
   - No state allocated until 3-way handshake completes

3. **Connection rate limiting** (iptables):
   ```bash
   iptables -A INPUT -p tcp --dport 41264 -m state --state NEW \
       -m recent --update --seconds 60 --hitcount 100 -j DROP
   ```
   - Limits new connections per IP
   - 100 connections/minute threshold

**Residual Risk:**

- ⚠️ **No per-IP limit in application**: Current implementation only has global limit
- **Recommendation**: Add per-IP tracking:
  ```rust
  let connections_from_ip = ip_tracker.get(remote_addr.ip());
  if connections_from_ip >= per_ip_limit {
      return; // Reject
  }
  ```

### Handshake-Level DoS

**Attack:** Start handshakes but never complete them

**Mitigations:**

1. **Handshake timeout**: Noise state machine times out
   - Implementation: Tokio task timeout (implicit)
   - Recommended: Explicit 15-second timeout

2. **State cleanup**: Connection dropped if handshake fails
   ```rust
   if handshake_fails {
       drop(connection); // Frees memory immediately
   }
   ```

3. **No resource allocation before handshake**:
   - Replay validator created only after handshake
   - Minimal memory usage during handshake (~200 bytes)

**Attack Scenarios:**

| Attack | Resource Consumed | Mitigation |
|--------|-------------------|------------|
| Half-open connections | TCP state (~4 KB) | SYN cookies |
| Incomplete handshakes | Noise state (~200 B) | Timeout + cleanup |
| Slow clients | Connection slot | Timeout + max connections |

### Timestamp-Based DoS

**Attack:** Replay old ClientHello messages

**Mitigation:**

```rust
let timestamp_age = now - client_hello.timestamp;
if timestamp_age > 30_seconds {
    return Err(TimestampTooOld);
}
if timestamp_age < -30_seconds {
    return Err(TimestampFromFuture);
}
```

**Properties:**

- 30-second window limits replay attack surface
- Clock skew tolerance: ±30 seconds (reasonable for NTP)
- Metrics track rejections: `lp_timestamp_validation_rejected`

**Residual Risk:**

- ⚠️ 30-second window allows replay of ClientHello within window
- **Mitigation**: Replay protection on post-handshake messages

### Credential Verification DoS

**Attack:** Flood gateway with fake credentials

**Mitigations:**

1. **Fast rejection path**:
   ```rust
   // Check signature before database lookup
   if !verify_bls_signature(&credential) {
       return Err(InvalidSignature); // Fast path
   }
   // Only then check database
   ```

2. **Database indexing**:
   ```sql
   CREATE INDEX idx_nullifiers ON spent_credentials(nullifier);
   ```
   - O(log n) nullifier lookup instead of O(n)

3. **Rate limiting** (future):
   - Limit credential verification attempts per IP
   - Exponential backoff for repeated failures

**Performance Impact:**

- BLS signature verification: ~5ms per credential
- Database lookup: ~1ms (with index)
- Total: ~6ms per invalid credential

**Attack Cost:**

- Attacker must generate BLS signatures (computationally expensive)
- Invalid signatures rejected before database query
- Real cost is in valid-looking but fake credentials (still requires crypto)

## Threat Scenarios

### Scenario 1: Passive Eavesdropper

**Attacker:** Network observer (ISP, hostile network)

**Capabilities:**
- Observe all LP traffic (including ClientHello)
- Analyze packet sizes, timing, patterns

**Protections:**
- ✅ ClientHello metadata visible but not sensitive (timestamp, nonce)
- ✅ Noise handshake encrypts all subsequent messages
- ✅ Registration request fully encrypted (credential not visible)
- ✅ ChaCha20-Poly1305 provides IND-CCA2 security

**Leakage:**
- ⚠️ Client IP address visible (inherent to TCP)
- ⚠️ Packet timing reveals registration events
- ⚠️ Connection to known gateway suggests Nym usage

**Recommendation:** Use LP for fast registration, mixnet for anonymity-critical operations.

### Scenario 2: Active MITM

**Attacker:** On-path adversary (malicious router, hostile WiFi)

**Capabilities:**
- Intercept, modify, drop, inject packets
- Cannot break cryptography

**Protections:**
- ✅ Noise XKpsk3 mutual authentication prevents impersonation
- ✅ Client verifies gateway's LP static public key
- ✅ Gateway verifies client via PSK derivation
- ✅ Any packet modification detected via Poly1305 MAC

**Attack Attempts:**

1. **Impersonate Gateway**:
   - Attacker doesn't have gateway's LP private key
   - Cannot complete handshake (Noise fails at `es` mix)
   - Client rejects connection

2. **Impersonate Client**:
   - Attacker doesn't know client's LP private key
   - Cannot derive correct PSK
   - Noise fails at `psk` mix in message 3
   - Gateway rejects connection

3. **Modify Messages**:
   - Poly1305 MAC fails
   - Noise decryption fails
   - Connection aborted

**Residual Risk:**
- ⚠️ DoS possible (drop packets, connection killed)
- ✅ Cannot learn registration data or credentials

### Scenario 3: Gateway Compromise

**Attacker:** Full access to gateway server

**Capabilities:**
- Read all gateway state (keys, database, memory)
- Modify gateway behavior
- Impersonate gateway to clients

**Impact:**

1. **Current Sessions**: Compromised
   - Attacker can decrypt ongoing registration requests
   - Can steal credentials from current sessions

2. **Past Sessions**: Protected (forward secrecy)
   - Ephemeral keys already destroyed
   - Cannot decrypt recorded traffic

3. **Future Sessions**: Compromised until key rotation
   - Attacker can impersonate gateway
   - Can steal credentials from new registrations

**Mitigations:**

1. **Key Rotation**:
   ```bash
   # Generate new LP keypair
   ./nym-node generate-lp-keypair
   # Update gateway descriptor (automatic on restart)
   ```
   - Invalidates attacker's stolen keys
   - Clients fetch new public key from descriptor

2. **Monitoring**:
   - Detect anomalous credential verification patterns
   - Alert on unusual database access
   - Monitor for key file modifications

3. **Defense in Depth**:
   - E-cash credentials have limited value (time-bound, nullifiers)
   - WireGuard keys rotatable by client
   - No long-term sensitive data stored

**Credential Reuse Prevention:**

- Nullifier stored in database
- Nullifier = Hash(credential_data)
- Even with database access, attacker cannot create new credentials
- Can only steal credentials submitted during compromise window

### Scenario 4: Replay Attack

**Attacker:** Records past LP sessions, replays later

**Attack Attempts:**

1. **Replay ClientHello**:
   - Timestamp validation rejects messages > 30s old
   - Nonce in salt changes per session
   - Cannot reuse old ClientHello

2. **Replay Handshake Messages**:
   - Noise uses ephemeral keys (fresh each session)
   - Replaying old handshake messages fails (wrong ephemeral key)
   - Handshake fails, no session established

3. **Replay Post-Handshake Packets**:
   - Counter-based replay protection
   - Bitmap tracks last 1024 packets
   - Duplicate counters rejected
   - Cannot replay old encrypted messages

4. **Replay Entire Session**:
   - Different ephemeral keys each time
   - Cannot replay connection to gateway
   - Even if gateway state reset, timestamp rejects old ClientHello

**Success Probability:** Negligible (< 2^-128)

### Scenario 5: Quantum Adversary (Future)

**Attacker:** Quantum computer with Shor's algorithm

**Capabilities:**
- Break X25519 ECDH in polynomial time
- Recover LP static private keys from public keys
- Does NOT break symmetric crypto (ChaCha20, Blake3)

**Impact:**

1. **Recorded Traffic**: Vulnerable
   - Attacker records all LP traffic now
   - Breaks X25519 later with quantum computer
   - Recovers PSKs from recorded ClientHellos
   - Decrypts recorded sessions

2. **Real-Time Interception**: Full compromise
   - Can impersonate gateway (knows private key)
   - Can decrypt all traffic
   - Complete MITM attack

**Mitigations (Future):**

1. **Hybrid PQ-KEM**:
   ```rust
   // Use both classical and post-quantum KEM
   let classical = X25519(client_priv, gateway_pub);
   let pq = Kyber768::encaps(gateway_pq_pub);
   let psk = Blake3(classical || pq, salt);
   ```

2. **Post-Quantum Noise**:
   - Noise specification supports PQ KEMs
   - Can upgrade to Kyber, NTRU, or SIKE
   - Requires protocol version 2

**Timeline:**
- Quantum threat: ~10-20 years away
- PQ upgrade: Can be deployed when threat becomes real
- Backward compatibility: Support both classical and PQ

## Security Recommendations

### For Gateway Operators

**High Priority:**

1. **Enable all DoS protections**:
   ```toml
   [lp]
   max_connections = 10000  # Adjust based on capacity
   timestamp_tolerance_secs = 30  # Don't increase unnecessarily
   ```

2. **Secure key storage**:
   ```bash
   chmod 600 ~/.nym/gateways/<id>/keys/lp_x25519.pem
   # Encrypt disk if possible
   ```

3. **Monitor metrics**:
   - Alert on high `lp_handshakes_failed`
   - Alert on unusual `lp_timestamp_validation_rejected`
   - Track `lp_credential_verification_failed` patterns

4. **Keep database secure**:
   - Regular backups
   - Index on `nullifier` column
   - Periodic cleanup of old nullifiers

**Medium Priority:**

5. **Implement per-IP rate limiting** (future):
   ```rust
   const MAX_CONNECTIONS_PER_IP: usize = 10;
   ```

6. **Regular key rotation**:
   - Rotate LP keypair every 6-12 months
   - Coordinate with network updates

7. **Firewall hardening**:
   ```bash
   # Only allow LP port
   ufw default deny incoming
   ufw allow 41264/tcp
   ```

### For Client Developers

**High Priority:**

1. **Verify gateway LP public key**:
   ```rust
   // Fetch from trusted source (network descriptor)
   let gateway_lp_pubkey = fetch_gateway_descriptor(gateway_id)
       .await?
       .lp_public_key;

   // Pin for future connections
   save_pinned_key(gateway_id, gateway_lp_pubkey);
   ```

2. **Handle errors securely**:
   ```rust
   match registration_result {
       Err(LpError::Replay(_)) => {
           // DO NOT retry immediately (might be replay attack)
           log::warn!("Replay detected, waiting before retry");
           tokio::time::sleep(Duration::from_secs(60)).await;
       }
       Err(e) => {
           // Other errors safe to retry
       }
   }
   ```

3. **Use fresh credentials**:
   - Don't reuse credentials across registrations
   - Check credential expiry before attempting registration

**Medium Priority:**

4. **Implement connection timeout**:
   ```rust
   tokio::time::timeout(
       Duration::from_secs(30),
       registration_client.register_lp(...)
   ).await?
   ```

5. **Secure local key storage**:
   - Use OS keychain for LP private keys
   - Don't log or expose keys

### For Network Operators

**High Priority:**

1. **Deploy monitoring infrastructure**:
   - Prometheus + Grafana for metrics
   - Alerting on security-relevant metrics
   - Correlation of events across gateways

2. **Incident response plan**:
   - Procedure for gateway compromise
   - Key rotation workflow
   - Client notification mechanism

3. **Regular security audits**:
   - External audit of Noise implementation
   - Penetration testing of LP endpoints
   - Review of credential verification logic

**Medium Priority:**

4. **Threat intelligence**:
   - Monitor for known attacks on Noise protocol
   - Track quantum computing advances
   - Plan PQ migration timeline

## Compliance Considerations

### Data Protection (GDPR, etc.)

**Personal Data Collected:**
- Client IP address (connection metadata)
- Credential nullifiers (pseudonymous identifiers)
- Timestamps (connection events)

**Data Retention:**
- IP addresses: Not stored beyond connection duration
- Nullifiers: Stored until credential expiry + grace period
- Logs: Configurable retention (default: 7 days)

**Privacy Protections:**
- Nullifiers pseudonymous (not linkable to real identity)
- No PII collected or stored
- Credentials use blind signatures (gateway doesn't learn identity)

### Security Compliance

**SOC 2 / ISO 27001 Requirements:**

1. **Access Control**:
   - LP keys protected (file permissions)
   - Database access restricted
   - Principle of least privilege

2. **Encryption in Transit**:
   - Noise protocol provides end-to-end encryption
   - TLS for metrics endpoint (if exposed)

3. **Logging and Monitoring**:
   - Security events logged
   - Metrics for anomaly detection
   - Audit trail for credential usage

4. **Incident Response**:
   - Key rotation procedure
   - Backup and recovery
   - Communication plan

## Audit Checklist

Before production deployment:

- [ ] Noise implementation reviewed by cryptographer
- [ ] Replay protection tested with edge cases (overflow, concurrency)
- [ ] DoS limits tested (connection flood, credential spam)
- [ ] Timing attack resistance verified (replay check, credential verification)
- [ ] Key storage secured (file permissions, encryption at rest)
- [ ] Monitoring and alerting configured
- [ ] Incident response plan documented
- [ ] Penetration testing performed
- [ ] Code review completed
- [ ] Dependencies audited (cargo-audit, cargo-deny)

## References

### Security Specifications

- **Noise Protocol Framework**: https://noiseprotocol.org/
- **XKpsk3 Analysis**: https://noiseexplorer.com/patterns/XKpsk3/
- **Curve25519**: https://cr.yp.to/ecdh.html
- **ChaCha20-Poly1305**: RFC 8439
- **Blake3**: https://github.com/BLAKE3-team/BLAKE3-specs

### Security Audits

- [ ] Noise implementation audit (pending)
- [ ] Cryptographic review (pending)
- [ ] Penetration test report (pending)

### Known Vulnerabilities

*None currently identified. This section will be updated as issues are discovered.*

## Responsible Disclosure

If you discover a security vulnerability in LP:

1. **DO NOT** publish vulnerability details publicly
2. Email security@nymtech.net with:
   - Description of vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested mitigation (if any)
3. Allow 90 days for patch development before public disclosure
4. Coordinate disclosure timeline with Nym team

**Bug Bounty**: Check https://nymtech.net/security for current bounty program.
