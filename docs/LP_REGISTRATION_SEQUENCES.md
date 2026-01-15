# LP Registration - Detailed Sequence Diagrams

**Technical deep-dive for engineering team**

---

## Table of Contents

- [LP Registration - Detailed Sequence Diagrams](#lp-registration---detailed-sequence-diagrams)
  - [Table of Contents](#table-of-contents)
  - [1. Happy Path: Successful dVPN Registration](#1-happy-path-successful-dvpn-registration)
  - [2. Error Scenario: Timestamp Validation Failure](#2-error-scenario-timestamp-validation-failure)
  - [3. Error Scenario: Credential Rejected](#3-error-scenario-credential-rejected)
  - [4. Noise XKpsk3 Handshake Detail](#4-noise-xkpsk3-handshake-detail)
  - [7. PSK Derivation Flow](#7-psk-derivation-flow)
  - [8. Message Format Specifications](#8-message-format-specifications)
    - [8.1. Packet Framing (Transport Layer)](#81-packet-framing-transport-layer)
    - [8.2. LpPacket Structure](#82-lppacket-structure)
    - [8.3. ClientHello Message](#83-clienthello-message)
    - [8.4. Noise Handshake Messages](#84-noise-handshake-messages)
    - [8.5. LpRegistrationRequest](#85-lpregistrationrequest)
    - [8.6. LpRegistrationResponse](#86-lpregistrationresponse)
    - [8.7. Encrypted Data Format](#87-encrypted-data-format)
  - [Summary](#summary)

---

## 1. Happy Path: Successful dVPN Registration

**Complete flow from TCP connect to WireGuard peer setup**

```
Client                                                  Gateway
(LpRegistrationClient)                                  (LpConnectionHandler)
  |                                                          |
  | [0] Setup Phase                                          |
  |──────────────────────────────────────────────────────────|
  |                                                          |
  | Generate LP keypair (X25519)                             | Load gateway identity (Ed25519)
  | client_lp_keypair = LpKeypair::default()                 | Convert to X25519:
  |   → secret_key: [32 bytes]                               |   gw_lp_keypair = ed25519_to_x25519(gw_identity)
  |   → public_key: [32 bytes]                               |     → secret_key: [32 bytes]
  |                                                          |     → public_key: [32 bytes]
  |                                                          |
  | [1] TCP Connection                                       |
  |──────────────────────────────────────────────────────────|
  |                                                          |
  |-- TCP SYN ──────────────────────────────────────────────>| bind(0.0.0.0:41264)
  |                                                          | accept()
  |<─ TCP SYN-ACK ───────────────────────────────────────────|
  |                                                          |
  |-- TCP ACK ──────────────────────────────────────────────>| spawn(handle_connection)
  |                                                          |   ↓
  |                                                          | inc!(lp_connections_total)
  |                                                          | inc!(active_lp_connections)
  |                                                          |
  | ✓ Connection established                                 |
  | Duration: ~12ms                                          |
  | [client.rs:133-169]                                      | [mod.rs:271-289]
  |                                                          |
  |                                                          |
  | [2] ClientHello (Cleartext PSK Setup)                    |
  |──────────────────────────────────────────────────────────|
  |                                                          |
  | Generate fresh salt:                                     |
  |   salt = random_bytes(32)                                |
  |                                                          |
  | Build ClientHello:                                       |
  | ┌──────────────────────────────────────────────────┐     |
  | │ LpPacket {                                       │     |
  | │   header: LpHeader {                             │     |
  | │     session_id: 0,                               │     |
  | │     sequence_number: 0,                          │     |
  | │     flags: 0,                                    │     |
  | │   },                                             │     |
  | │   message: ClientHello(ClientHelloData {         │     |
  | │     client_public_key: client_lp_keypair.public, │     |
  | │     salt: [32 bytes],                            │     |
  | │     timestamp: unix_timestamp(),                 │     |
  | │     protocol_version: 1,                         │     |
  | │   })                                             │     |
  | │ }                                                │     |
  | └──────────────────────────────────────────────────┘     |
  |                                                          |
  | Serialize (bincode):                                     |
  |   packet_bytes = serialize_lp_packet(client_hello)       |
  |                                                          |
  | Frame (length-prefix):                                   |
  |   frame = [len as u32 BE (4 bytes)] + packet_bytes       |
  |                                                          |
  |-- [4 byte len][ClientHello packet] ────────────────────>| receive_client_hello()
  |                                                          |   ↓
  |                                                          | Read 4 bytes → packet_len
  |                                                          | Validate: packet_len <= 65536
  |                                                          | Read packet_len bytes → packet_buf
  |                                                          | Deserialize → ClientHelloData
  |                                                          |   ↓
  |                                                          | Extract:
  |                                                          |   client_public_key: PublicKey
  |                                                          |   salt: [u8; 32]
  |                                                          |   timestamp: u64
  |                                                          |   ↓
  |                                                          | validate_timestamp(timestamp):
  |                                                          |   now = SystemTime::now()
  |                                                          |   client_time = UNIX_EPOCH + Duration(timestamp)
  |                                                          |   diff = abs(now - client_time)
  |                                                          |   if diff > 30s:
  |                                                          |     inc!(lp_client_hello_failed{reason="timestamp"})
  |                                                          |     return ERROR
  |                                                          |   ↓
  |                                                          | ✓ Timestamp valid (within ±30s)
  |                                                          |
  | Duration: ~8ms                                           | [handler.rs:275-323, 233-261]
  |                                                          |
  |                                                          |
  | [3] PSK Derivation (Both Sides)                          |
  |──────────────────────────────────────────────────────────|
  |                                                          |
  | Client computes PSK:                                     | Gateway computes PSK:
  |   psk = derive_psk(                                      |   psk = derive_psk(
  |     client_lp_keypair.secret,                            |     gw_lp_keypair.secret,
  |     gw_lp_keypair.public,                                |     client_public_key,
  |     salt                                                 |     salt
  |   )                                                      |   )
  |   ↓                                                      |   ↓
  | shared_secret = ECDH(client_secret, gw_public)           | shared_secret = ECDH(gw_secret, client_public)
  |   → [32 bytes]                                           |   → [32 bytes] (same as client!)
  |   ↓                                                      |   ↓
  | hasher = Blake3::new_keyed(PSK_KDF_KEY)                  | hasher = Blake3::new_keyed(PSK_KDF_KEY)
  | hasher.update(b"nym-lp-psk-v1")                          | hasher.update(b"nym-lp-psk-v1")
  | hasher.update(shared_secret)                             | hasher.update(shared_secret)
  | hasher.update(salt)                                      | hasher.update(salt)
  |   ↓                                                      |   ↓
  | psk = hasher.finalize_xof().read(32 bytes)               | psk = hasher.finalize_xof().read(32 bytes)
  |   → [32 bytes PSK]                                       |   → [32 bytes PSK] (same as client!)
  |                                                          |
  | [psk.rs:28-52]                                           | [psk.rs:28-52]
  |                                                          |
  |                                                          |
  | [4] Noise XKpsk3 Handshake (3-way)                       |
  |──────────────────────────────────────────────────────────|
  |                                                          |
  | Create state machine as INITIATOR:                       | Create state machine as RESPONDER:
  |   state_machine = LpStateMachine::new(                   |   state_machine = LpStateMachine::new(
  |     is_initiator: true,                                  |     is_initiator: false,
  |     local_keypair: client_lp_keypair,                    |     local_keypair: gw_lp_keypair,
  |     remote_pubkey: gw_lp_keypair.public,                 |     remote_pubkey: client_public_key,
  |     psk: psk                                             |     psk: psk
  |   )                                                      |   )
  |   ↓                                                      |   ↓
  | noise = NoiseBuilder()                                   | noise = NoiseBuilder()
  |   .pattern("Noise_XKpsk3_25519_ChaChaPoly_BLAKE2s")      |   .pattern("Noise_XKpsk3_25519_ChaChaPoly_BLAKE2s")
  |   .local_private_key(client_secret)                      |   .local_private_key(gw_secret)
  |   .remote_public_key(gw_public)                          |   .remote_public_key(client_public)
  |   .psk(3, psk)  // PSK in 3rd message                    |   .psk(3, psk)
  |   .build_initiator()                                     |   .build_responder()
  |   ↓                                                      |   ↓
  | state = HandshakeInProgress                              | state = WaitingForHandshake
  |                                                          |
  | ────────────────────────────────────────────────────────────────────
  | Handshake Message 1: -> e (ephemeral key exchange)
  | ────────────────────────────────────────────────────────────────────
  |                                                          |
  | action = state_machine.process_input(StartHandshake)     |
  |   ↓                                                      |
  | noise.write_message(&[], &mut msg_buf)                   |
  |   → msg_buf = client_ephemeral_public [32 bytes]         |
  |   ↓                                                      |
  | packet = LpPacket {                                      |
  |   header: LpHeader { session_id: 0, seq: 1 },            |
  |   message: Handshake(msg_buf)                            |
  | }                                                        |
  |                                                          |
  |-- [len][Handshake: e (32 bytes)] ──────────────────────>| receive_packet()
  |                                                          |   ↓
  |                                                          | action = state_machine.process_input(
  |                                                          |   ReceivePacket(packet)
  |                                                          | )
  |                                                          |   ↓
  |                                                          | noise.read_message(&handshake_data, &mut buf)
  |                                                          |   → client_e_pub extracted
  |                                                          |   → No payload expected (buf empty)
  |                                                          |
  | ────────────────────────────────────────────────────────────────────
  | Handshake Message 2: <- e, ee, s, es (respond with gateway identity)
  | ────────────────────────────────────────────────────────────────────
  |                                                          |
  |                                                          | noise.write_message(&[], &mut msg_buf)
  |                                                          |   → e: gw_ephemeral_public [32 bytes]
  |                                                          |   → ee: DH(gw_e_priv, client_e_pub)
  |                                                          |   → s: gw_static_public [32 bytes] (encrypted)
  |                                                          |   → es: DH(gw_e_priv, client_static_pub)
  |                                                          |   ↓
  |                                                          | msg_buf = [gw_e_pub (32)] + [encrypted_gw_static (48)]
  |                                                          |   → Total: 80 bytes
  |                                                          |   ↓
  |                                                          | packet = LpPacket {
  |                                                          |   header: LpHeader { session_id: 0, seq: 1 },
  |                                                          |   message: Handshake(msg_buf)
  |                                                          | }
  |                                                          |
  |<─ [len][Handshake: e,ee,s,es (80 bytes)] ────────────────| send_packet()
  |                                                          |
  | action = state_machine.process_input(                    |
  |   ReceivePacket(packet)                                  |
  | )                                                        |
  |   ↓                                                      |
  | noise.read_message(&handshake_data, &mut buf)            |
  |   → gw_e_pub extracted                                   |
  |   → DH(client_e_priv, gw_e_pub) computed                 |
  |   → gw_static_pub decrypted and authenticated            |
  |   → DH(client_static_priv, gw_e_pub) computed            |
  |   ↓                                                      |
  | ✓ Gateway authenticated                                  |
  |                                                          |
  | ────────────────────────────────────────────────────────────────────
  | Handshake Message 3: -> s, se, psk (final auth + PSK)
  | ────────────────────────────────────────────────────────────────────
  |                                                          |
  | noise.write_message(&[], &mut msg_buf)                   |
  |   → s: client_static_public [32 bytes] (encrypted)       |
  |   → se: DH(client_static_priv, gw_e_pub)                 |
  |   → psk: Mix in pre-shared key                           |
  |   ↓                                                      |
  | msg_buf = [encrypted_client_static (48)]                 |
  |   → Total: 48 bytes                                      |
  |   ↓                                                      |
  | packet = LpPacket {                                      |
  |   header: LpHeader { session_id: 0, seq: 2 },            |
  |   message: Handshake(msg_buf)                            |
  | }                                                        |
  |                                                          |
  |-- [len][Handshake: s,se,psk (48 bytes)] ────────────────>| receive_packet()
  |                                                          |   ↓
  |                                                          | action = state_machine.process_input(
  |                                                          |   ReceivePacket(packet)
  |                                                          | )
  |                                                          |   ↓
  |                                                          | noise.read_message(&handshake_data, &mut buf)
  |                                                          |   → client_static_pub decrypted and authenticated
  |                                                          |   → DH(gw_static_priv, client_e_pub) computed
  |                                                          |   → PSK mixed into key material
  |                                                          |   ↓
  |                                                          | ✓ Client authenticated
  |                                                          | ✓ PSK verified (implicitly)
  |                                                          |
  | ────────────────────────────────────────────────────────────────────
  | Handshake Complete! Derive transport keys
  | ────────────────────────────────────────────────────────────────────
  |                                                          |
  | transport = noise.into_transport_mode()                  | transport = noise.into_transport_mode()
  |   ↓                                                      |   ↓
  | tx_cipher = ChaCha20-Poly1305 (client→gw key)            | rx_cipher = ChaCha20-Poly1305 (client→gw key)
  | rx_cipher = ChaCha20-Poly1305 (gw→client key)            | tx_cipher = ChaCha20-Poly1305 (gw→client key)
  | replay_validator = ReplayValidator::new()                | replay_validator = ReplayValidator::new()
  |   → nonce_high: u64 = 0                                  |   → nonce_high: u64 = 0
  |   → nonce_low: u64 = 0                                   |   → nonce_low: u64 = 0
  |   → seen_bitmap: [u64; 16] = [0; 16]                     |   → seen_bitmap: [u64; 16] = [0; 16]
  |   ↓                                                      |   ↓
  | state = HandshakeComplete                                | state = HandshakeComplete
  |                                                          |
  | ✓ Encrypted channel established                          | ✓ Encrypted channel established
  | Duration: ~45ms (3 round-trips)                          | inc!(lp_handshakes_success)
  | [client.rs:212-325]                                      | [handler.rs:149-175]
  | [state_machine.rs:96-420]                                | [state_machine.rs:96-420]
  |                                                          |
  |                                                          |
  | [5] Send Registration Request (Encrypted)                |
  |──────────────────────────────────────────────────────────|
  |                                                          |
  | Acquire bandwidth credential:                            |
  |   credential = bandwidth_controller                      |
  |     .get_ecash_ticket(                                   |
  |       ticket_type,                                       |
  |       gateway_identity,                                  |
  |       DEFAULT_TICKETS_TO_SPEND                           |
  |     ).await?                                             |
  |   ↓                                                      |
  | CredentialSpendingData {                                 |
  |   nullifier: [32 bytes],                                 |
  |   signature: BLS12-381 signature,                        |
  |   bandwidth_amount: u64,                                 |
  |   expiry: u64                                            |
  | }                                                        |
  |   ↓                                                      |
  | Generate WireGuard keypair:                              |
  |   wg_keypair = wireguard_rs::KeyPair::new(&mut rng)      |
  |   wg_public_key = wg_keypair.public                      |
  |   ↓                                                      |
  | Build request:                                           |
  | ┌──────────────────────────────────────────────────┐     |
  | │ LpRegistrationRequest {                          │     |
  | │   wg_public_key: wg_public_key,                  │     |
  | │   credential: credential,                        │     |
  | │   ticket_type: TicketType::V1MixnetEntry,        │     |
  | │   mode: RegistrationMode::Dvpn,                  │     |
  | │   client_ip: IpAddr::V4(...),                    │     |
  | │   timestamp: unix_timestamp()                    │     |
  | │ }                                                │     |
  | └──────────────────────────────────────────────────┘     |
  |   ↓                                                      |
  | request_bytes = bincode::serialize(&request)?            |
  |   → ~300-500 bytes (depends on credential size)          |
  |   ↓                                                      |
  | action = state_machine.process_input(                    |
  |   SendData(request_bytes)                                |
  | )                                                        |
  |   ↓                                                      |
  | ciphertext = tx_cipher.encrypt(                          |
  |   nonce: seq_num,                                        |
  |   plaintext: request_bytes,                              |
  |   aad: header_bytes                                      |
  | )                                                        |
  |   → ciphertext = request_bytes + [16 byte auth tag]      |
  |   ↓                                                      |
  | packet = LpPacket {                                      |
  |   header: LpHeader { session_id: assigned, seq: 3 },     |
  |   message: EncryptedData(ciphertext)                     |
  | }                                                        |
  |                                                          |
  |-- [len][EncryptedData: encrypted request] ──────────────>| receive_packet()
  |                                                          |   ↓
  |                                                          | action = state_machine.process_input(
  |                                                          |   ReceivePacket(packet)
  |                                                          | )
  |                                                          |   ↓
  |                                                          | Check replay (seq_num against window):
  |                                                          |   replay_validator.validate(seq_num)?
  |                                                          |     → Check if seq_num already seen
  |                                                          |     → Update sliding window bitmap
  |                                                          |     → If duplicate: reject
  |                                                          |   ↓
  |                                                          | plaintext = rx_cipher.decrypt(
  |                                                          |   nonce: seq_num,
  |                                                          |   ciphertext: encrypted_data,
  |                                                          |   aad: header_bytes
  |                                                          | )
  |                                                          |   ↓
  |                                                          | request = bincode::deserialize::<
  |                                                          |   LpRegistrationRequest
  |                                                          | >(&plaintext)?
  |                                                          |
  | Duration: ~5ms                                           | [handler.rs:177-211]
  | [client.rs:433-507]                                      |
  |                                                          |
  |                                                          |
  | [6] Process Registration (Gateway Business Logic)        |
  |──────────────────────────────────────────────────────────|
  |                                                          |
  |                                                          | process_registration(request, state, session_id)
  |                                                          |   ↓
  |                                                          | [6.1] Validate timestamp:
  |                                                          |   if !request.validate_timestamp(30):
  |                                                          |     inc!(lp_registration_failed_timestamp)
  |                                                          |     return ERROR
  |                                                          |   ↓
  |                                                          | ✓ Timestamp valid
  |                                                          |
  |                                                          | [registration.rs:147-151]
  |                                                          |   ↓
  |                                                          | [6.2] Handle dVPN mode:
  |                                                          |   ↓
  |                                                          | ┌──────────────────────────────────────┐
  |                                                          | │ register_wg_peer(                    │
  |                                                          | │   request.wg_public_key,             │
  |                                                          | │   request.client_ip,                 │
  |                                                          | │   request.ticket_type,               │
  |                                                          | │   state                              │
  |                                                          | │ )                                    │
  |                                                          | └───────────────┬──────────────────────┘
  |                                                          |                 ↓
  |                                                          | [6.2.1] Allocate private IPs:
  |                                                          |   random_octet = rng.gen_range(1..255)
  |                                                          |   client_ipv4 = 10.1.0.{random_octet}
  |                                                          |   client_ipv6 = fd00::{random_octet}
  |                                                          |   ↓
  |                                                          | [6.2.2] Create WireGuard peer config:
  |                                                          |   peer = Peer {
  |                                                          |     public_key: request.wg_public_key,
  |                                                          |     allowed_ips: [
  |                                                          |       client_ipv4/32,
  |                                                          |       client_ipv6/128
  |                                                          |     ],
  |                                                          |     persistent_keepalive: Some(25),
  |                                                          |     endpoint: None
  |                                                          |   }
  |                                                          |   ↓
  |                                                          | [6.2.3] CRITICAL ORDER - Store in DB first:
  |                                                          |   client_id = storage.insert_wireguard_peer(
  |                                                          |     &peer,
  |                                                          |     ticket_type
  |                                                          |   ).await?
  |                                                          |   ↓
  |                                                          | SQL: INSERT INTO wireguard_peers
  |                                                          |      (public_key, ticket_type)
  |                                                          |      VALUES (?, ?)
  |                                                          |      RETURNING id
  |                                                          |   → client_id: i64 (auto-increment)
  |                                                          |   ↓
  |                                                          | [6.2.4] Create bandwidth entry:
  |                                                          |   credential_storage_preparation(
  |                                                          |     ecash_verifier,
  |                                                          |     client_id
  |                                                          |   ).await?
  |                                                          |   ↓
  |                                                          | SQL: INSERT INTO bandwidth
  |                                                          |      (client_id, available)
  |                                                          |      VALUES (?, 0)
  |                                                          |   ↓
  |                                                          | [6.2.5] Send to WireGuard controller:
  |                                                          |   (tx, rx) = oneshot::channel()
  |                                                          |   wg_controller.send(
  |                                                          |     PeerControlRequest::AddPeer {
  |                                                          |       peer: peer.clone(),
  |                                                          |       response_tx: tx
  |                                                          |     }
  |                                                          |   ).await?
  |                                                          |   ↓
  |                                                          |   result = rx.await?
  |                                                          |   if result.is_err():
  |                                                          |     // Rollback: remove from DB
  |                                                          |     return ERROR
  |                                                          |   ↓
  |                                                          | ✓ WireGuard peer added successfully
  |                                                          |   ↓
  |                                                          | [6.2.6] Prepare gateway data:
  |                                                          |   gateway_data = GatewayData {
  |                                                          |     public_key: wireguard_data.public_key,
  |                                                          |     endpoint: format!(
  |                                                          |       "{}:{}",
  |                                                          |       wireguard_data.announced_ip,
  |                                                          |       wireguard_data.listen_port
  |                                                          |     ),
  |                                                          |     private_ipv4: client_ipv4,
  |                                                          |     private_ipv6: client_ipv6
  |                                                          |   }
  |                                                          |
  |                                                          | [registration.rs:291-404]
  |                                                          |   ↓
  |                                                          | [6.3] Verify e-cash credential:
  |                                                          |   ↓
  |                                                          | ┌──────────────────────────────────────┐
  |                                                          | │ credential_verification(             │
  |                                                          | │   ecash_verifier,                    │
  |                                                          | │   request.credential,                │
  |                                                          | │   client_id                          │
  |                                                          | │ )                                    │
  |                                                          | └───────────────┬──────────────────────┘
  |                                                          |                 ↓
  |                                                          | [6.3.1] Check if mock mode:
  |                                                          |   if ecash_verifier.is_mock():
  |                                                          |     return Ok(MOCK_BANDWIDTH) // 1GB
  |                                                          |   ↓
  |                                                          | [6.3.2] Real verification:
  |                                                          |   verifier = CredentialVerifier::new(
  |                                                          |     CredentialSpendingRequest(credential),
  |                                                          |     ecash_verifier.clone(),
  |                                                          |     BandwidthStorageManager::new(
  |                                                          |       storage,
  |                                                          |       client_id
  |                                                          |     )
  |                                                          |   )
  |                                                          |   ↓
  |                                                          | [6.3.3] Check nullifier not spent:
  |                                                          | SQL: SELECT COUNT(*) FROM spent_credentials
  |                                                          |      WHERE nullifier = ?
  |                                                          |   if count > 0:
  |                                                          |     inc!(lp_credential_verification_failed{
  |                                                          |       reason="already_spent"
  |                                                          |     })
  |                                                          |     return ERROR
  |                                                          |   ↓
  |                                                          | [6.3.4] Verify BLS signature:
  |                                                          |   blinding_factor = credential.blinding_factor
  |                                                          |   signature = credential.signature
  |                                                          |   message = hash(
  |                                                          |     gateway_identity +
  |                                                          |     bandwidth_amount +
  |                                                          |     expiry
  |                                                          |   )
  |                                                          |   ↓
  |                                                          |   if !bls12_381_verify(
  |                                                          |     public_key: ecash_verifier.public_key(),
  |                                                          |     message: message,
  |                                                          |     signature: signature
  |                                                          |   ):
  |                                                          |     inc!(lp_credential_verification_failed{
  |                                                          |       reason="invalid_signature"
  |                                                          |     })
  |                                                          |     return ERROR
  |                                                          |   ↓
  |                                                          | ✓ Signature valid
  |                                                          |   ↓
  |                                                          | [6.3.5] Mark nullifier spent:
  |                                                          | SQL: INSERT INTO spent_credentials
  |                                                          |      (nullifier, expiry)
  |                                                          |      VALUES (?, ?)
  |                                                          |   ↓
  |                                                          | [6.3.6] Allocate bandwidth:
  |                                                          | SQL: UPDATE bandwidth
  |                                                          |      SET available = available + ?
  |                                                          |      WHERE client_id = ?
  |                                                          |   → allocated_bandwidth = credential.bandwidth_amount
  |                                                          |   ↓
  |                                                          | ✓ Credential verified & bandwidth allocated
  |                                                          |   inc_by!(
  |                                                          |     lp_bandwidth_allocated_bytes_total,
  |                                                          |     allocated_bandwidth
  |                                                          |   )
  |                                                          |
  |                                                          | [registration.rs:87-133]
  |                                                          |   ↓
  |                                                          | [6.4] Build success response:
  |                                                          |   response = LpRegistrationResponse {
  |                                                          |     success: true,
  |                                                          |     error: None,
  |                                                          |     gateway_data: Some(gateway_data),
  |                                                          |     allocated_bandwidth,
  |                                                          |     session_id
  |                                                          |   }
  |                                                          |   ↓
  |                                                          | inc!(lp_registration_success_total)
  |                                                          | inc!(lp_registration_dvpn_success)
  |                                                          |
  | Duration: ~150ms (DB + WG + ecash verify)                | [registration.rs:136-288]
  |                                                          |
  |                                                          |
  | [7] Send Registration Response (Encrypted)               |
  |──────────────────────────────────────────────────────────|
  |                                                          |
  |                                                          | response_bytes = bincode::serialize(&response)?
  |                                                          |   ↓
  |                                                          | action = state_machine.process_input(
  |                                                          |   SendData(response_bytes)
  |                                                          | )
  |                                                          |   ↓
  |                                                          | ciphertext = tx_cipher.encrypt(
  |                                                          |   nonce: seq_num,
  |                                                          |   plaintext: response_bytes,
  |                                                          |   aad: header_bytes
  |                                                          | )
  |                                                          |   ↓
  |                                                          | packet = LpPacket {
  |                                                          |   header: LpHeader { session_id, seq: 4 },
  |                                                          |   message: EncryptedData(ciphertext)
  |                                                          | }
  |                                                          |
  |<─ [len][EncryptedData: encrypted response] ──────────────| send_packet()
  |                                                          |
  | receive_packet()                                         |
  |   ↓                                                      |
  | action = state_machine.process_input(                    |
  |   ReceivePacket(packet)                                  |
  | )                                                        |
  |   ↓                                                      |
  | Check replay: replay_validator.validate(seq_num)?        |
  |   ↓                                                      |
  | plaintext = rx_cipher.decrypt(                           |
  |   nonce: seq_num,                                        |
  |   ciphertext: encrypted_data,                            |
  |   aad: header_bytes                                      |
  | )                                                        |
  |   ↓                                                      |
  | response = bincode::deserialize::<                       |
  |   LpRegistrationResponse                                 |
  | >(&plaintext)?                                           |
  |   ↓                                                      |
  | Validate response:                                       |
  |   if !response.success:                                  |
  |     return Err(RegistrationRejected {                    |
  |       reason: response.error                             |
  |     })                                                   |
  |   ↓                                                      |
  | gateway_data = response.gateway_data                     |
  |   .ok_or(MissingGatewayData)?                            |
  |   ↓                                                      |
  | ✓ Registration complete!                                 |
  |                                                          |
  | [client.rs:615-715]                                      | [handler.rs:177-211]
  |                                                          |
  |                                                          |
  | [8] Connection Cleanup                                   |
  |──────────────────────────────────────────────────────────|
  |                                                          |
  | TCP close (FIN)                                          |
  |-- FIN ──────────────────────────────────────────────────>|
  |<─ ACK ───────────────────────────────────────────────────|
  |<─ FIN ───────────────────────────────────────────────────|
  |-- ACK ──────────────────────────────────────────────────>|
  |                                                          |
  | ✓ Connection closed gracefully                           | dec!(active_lp_connections)
  |                                                          | inc!(lp_connections_completed_gracefully)
  |                                                          | observe!(lp_connection_duration_seconds, duration)
  |                                                          |
  |                                                          |
  | [9] Client Has WireGuard Configuration                   |
  |──────────────────────────────────────────────────────────|
  |                                                          |
  | Client can now configure WireGuard tunnel:               |
  | ┌──────────────────────────────────────────────────┐     |
  | │ [Interface]                                      │     |
  | │ PrivateKey = <client_wg_keypair.private>         │     |
  | │ Address = 10.1.0.42/32, fd00::42/128             │     |
  | │                                                  │     |
  | │ [Peer]                                           │     |
  | │ PublicKey = <gateway_data.public_key>            │     |
  | │ Endpoint = <gateway_data.endpoint>               │     |
  | │ AllowedIPs = 0.0.0.0/0, ::/0                     │     |
  | │ PersistentKeepalive = 25                         │     |
  | └──────────────────────────────────────────────────┘     |
  |                                                          |
  | Total Registration Time: ~221ms                          |
  |   ├─ TCP Connect: 12ms                                   |
  |   ├─ ClientHello: 8ms                                    |
  |   ├─ Noise Handshake: 45ms                               |
  |   ├─ Registration Request: 5ms                           |
  |   ├─ Gateway Processing: 150ms                           |
  |   └─ Response Receive: 8ms                               |
  |                                                          |
  | ✅ SUCCESS                                               |✅ SUCCESS
  |                                                          |

```

**Code References**:
- Client: `nym-registration-client/src/lp_client/client.rs:39-715`
- Gateway Handler: `gateway/src/node/lp_listener/handler.rs:101-478`
- Registration Logic: `gateway/src/node/lp_listener/registration.rs:58-404`
- State Machine: `common/nym-lp/src/state_machine.rs:96-420`
- Noise Protocol: `common/nym-lp/src/noise_protocol.rs:40-88`
- PSK Derivation: `common/nym-lp/src/psk.rs:28-52`
- Replay Protection: `common/nym-lp/src/replay/validator.rs:25-125`

---

## 2. Error Scenario: Timestamp Validation Failure

**Client clock skew exceeds tolerance**

```
Client                                                  Gateway
  |                                                          |
  | [1] TCP Connect                                          |
  |-- TCP SYN ──────────────────────────────────────────────>| accept()
  |<─ TCP SYN-ACK ───────────────────────────────────────────|
  |-- TCP ACK ──────────────────────────────────────────────>|
  |                                                          |
  |                                                          |
  | [2] ClientHello with Bad Timestamp                       |
  |──────────────────────────────────────────────────────────|
  |                                                          |
  | Client system time is WRONG:                             |
  |   client_time = SystemTime::now() // e.g., 2025-01-01    |
  |   ↓                                                      |
  | packet = LpPacket {                                      |
  |   message: ClientHello {                                 |
  |     timestamp: client_time.as_secs(), // 1735689600      |
  |     ...                                                  |
  |   }                                                      |
  | }                                                        |
  |                                                          |
  |-- [len][ClientHello: timestamp=1735689600] ─────────────>| receive_client_hello()
  |                                                          |   ↓
  |                                                          | now = SystemTime::now()
  |                                                          |   → e.g., 1752537600 (2025-11-11)
  |                                                          | client_time = UNIX_EPOCH + Duration(1735689600)
  |                                                          |   ↓
  |                                                          | diff = abs(now - client_time)
  |                                                          |   → abs(1752537600 - 1735689600)
  |                                                          |   → 16848000 seconds (~195 days!)
  |                                                          |   ↓
  |                                                          | if diff > timestamp_tolerance_secs (30):
  |                                                          |   inc!(lp_client_hello_failed{
  |                                                          |     reason="timestamp_too_old"
  |                                                          |   })
  |                                                          |   ↓
  |                                                          |   error_msg = format!(
  |                                                          |     "ClientHello timestamp too old: {} seconds diff",
  |                                                          |     diff
  |                                                          |   )
  |                                                          |   ↓
  |                                                          |   // Gateway CLOSES connection
  |                                                          |   return Err(TimestampValidationFailed)
  |                                                          |
  |<─ TCP FIN ───────────────────────────────────────────────| Connection closed
  |                                                          |
  | ❌ Error: Connection closed unexpectedly                 |
  | Client logs: "Failed to receive handshake response"      |
  |                                                          |
  | [client.rs:212]                                          | [handler.rs:233-261, 275-323]
  |                                                          |
  |                                                          |
  | [Mitigation]                                             |
  |──────────────────────────────────────────────────────────|
  |                                                          |
  | Option 1: Fix client system time                         |
  |   → NTP sync recommended                                 |
  |                                                          |
  | Option 2: Increase gateway tolerance                     | Option 2: Increase gateway tolerance
  |                                                          | Edit config.toml:
  |                                                          |   [lp]
  |                                                          |   timestamp_tolerance_secs = 300
  |                                                          |     (5 minutes instead of 30s)
  |                                                          |
```

**Code References**:
- Timestamp validation: `gateway/src/node/lp_listener/handler.rs:233-261`
- ClientHello receive: `gateway/src/node/lp_listener/handler.rs:275-323`
- Config: `gateway/src/node/lp_listener/mod.rs:78-136`

---

## 3. Error Scenario: Credential Rejected

**E-cash credential nullifier already spent (double-spend attempt)**

```
Client                                                  Gateway
  |                                                          |
  | ... (TCP Connect + Handshake successful) ...             |
  |                                                          |
  |                                                          |
  | [1] Send Registration with REUSED Credential             |
  |──────────────────────────────────────────────────────────|
  |                                                          |
  | credential = {                                           |
  |   nullifier: 0xABCD... (ALREADY SPENT!)                  |
  |   signature: <valid BLS signature>,                      |
  |   bandwidth_amount: 1073741824,                          |
  |   expiry: <future timestamp>                             |
  | }                                                        |
  |   ↓                                                      |
  | request = LpRegistrationRequest {                        |
  |   credential: credential, // reused!                     |
  |   ...                                                    |
  | }                                                        |
  |                                                          |
  |-- [Encrypted Request: reused credential] ───────────────>| process_registration()
  |                                                          |   ↓
  |                                                          | credential_verification(
  |                                                          |   ecash_verifier,
  |                                                          |   request.credential,
  |                                                          |   client_id
  |                                                          | )
  |                                                          |   ↓
  |                                                          | [Check nullifier in DB]:
  |                                                          | SQL: SELECT COUNT(*) FROM spent_credentials
  |                                                          |      WHERE nullifier = 0xABCD...
  |                                                          |   ↓
  |                                                          | count = 1 (already exists!)
  |                                                          |   ↓
  |                                                          | inc!(lp_credential_verification_failed{
  |                                                          |   reason="already_spent"
  |                                                          | })
  |                                                          | inc!(lp_registration_failed_credential)
  |                                                          |   ↓
  |                                                          | error_response = LpRegistrationResponse {
  |                                                          |   success: false,
  |                                                          |   error: Some(
  |                                                          |     "Credential already spent (nullifier seen)"
  |                                                          |   ),
  |                                                          |   gateway_data: None,
  |                                                          |   allocated_bandwidth: 0,
  |                                                          |   session_id: 0
  |                                                          | }
  |                                                          |   ↓
  |                                                          | Encrypt & send response
  |                                                          |
  |<─ [Encrypted Response: error] ───────────────────────────| send_packet()
  |                                                          |
  | Decrypt response                                         |
  |   ↓                                                      |
  | response.success == false                                |
  | response.error == "Credential already spent..."          |
  |   ↓                                                      |
  | ❌ Error: RegistrationRejected {                         |
  |   reason: "Credential already spent (nullifier seen)"    |
  | }                                                        |
  |                                                          |
  | [client.rs:615-715]                                      | [registration.rs:87-133]
  |                                                          |
  |                                                          |
  | [Recovery Action]                                        |
  |──────────────────────────────────────────────────────────|
  |                                                          |
  | Client must acquire NEW credential:                      |
  |   new_credential = bandwidth_controller                  |
  |     .get_ecash_ticket(                                   |
  |       ticket_type,                                       |
  |       gateway_identity,                                  |
  |       DEFAULT_TICKETS_TO_SPEND                           |
  |     ).await?                                             |
  |   ↓                                                      |
  | Retry registration with new credential                   |
  |                                                          |
```

**Other Credential Rejection Reasons**:

1. **Invalid BLS Signature**:
   ```
   reason: "invalid_signature"
   Cause: Credential tampered with or issued by wrong authority
   ```

2. **Credential Expired**:
   ```
   reason: "expired"
   Cause: credential.expiry < SystemTime::now()
   ```

3. **Bandwidth Amount Mismatch**:
   ```
   reason: "bandwidth_mismatch"
   Cause: Credential bandwidth doesn't match ticket type
   ```

**Code References**:
- Credential verification: `gateway/src/node/lp_listener/registration.rs:87-133`
- Nullifier check: Database query in credential storage manager
- Error response: `common/registration/src/lp_messages.rs`

---

## 4. Noise XKpsk3 Handshake Detail

**Cryptographic operations and authentication flow**

```
Initiator (Client)                                  Responder (Gateway)
  |                                                          |
  | [Pre-Handshake: PSK Derivation]                          |
  |──────────────────────────────────────────────────────────|
  |                                                          |
  | Both sides have:                                         |
  |   • Client static keypair: (c_s_priv, c_s_pub)           |
  |   • Gateway static keypair: (g_s_priv, g_s_pub)          |
  |   • PSK derived from ECDH(c_s, g_s) + salt               |
  |                                                          |
  | Initialize Noise:                                        | Initialize Noise:
  |   protocol = "Noise_XKpsk3_25519_ChaChaPoly_BLAKE2s"     |   protocol = "Noise_XKpsk3_25519_ChaChaPoly_BLAKE2s"
  |   local_static = c_s_priv                                |   local_static = g_s_priv
  |   remote_static = g_s_pub (known)                        |   remote_static = c_s_pub (from ClientHello)
  |   psk_position = 3 (in 3rd message)                      |   psk_position = 3
  |   psk = [32 bytes derived PSK]                           |   psk = [32 bytes derived PSK]
  |   ↓                                                      |   ↓
  | state = HandshakeState::initialize()                     | state = HandshakeState::initialize()
  |   chaining_key = HASH("Noise_XKpsk3...")                 |   chaining_key = HASH("Noise_XKpsk3...")
  |   h = HASH(protocol_name)                                |   h = HASH(protocol_name)
  |   h = HASH(h || g_s_pub)  // Mix in responder static     |   h = HASH(h || g_s_pub)
  |                                                          |
  |                                                          |
  | ═══════════════════════════════════════════════════════════════════
  | Message 1: -> e
  | ═══════════════════════════════════════════════════════════════════
  |                                                          |
  | [Initiator Actions]:                                     |
  |   Generate ephemeral keypair:                            |
  |     c_e_priv, c_e_pub = X25519::generate()               |
  |     ↓                                                    |
  |   Mix ephemeral public into hash:                        |
  |     h = HASH(h || c_e_pub)                               |
  |     ↓                                                    |
  |   Build message:                                         |
  |     msg1 = c_e_pub  (32 bytes, plaintext)                |
  |     ↓                                                    |
  |   Send:                                                  |
  |                                                          |
  |-- msg1: [c_e_pub (32 bytes)] ───────────────────────────>| [Responder Actions]:
  |                                                          |   ↓
  |                                                          |   Extract:
  |                                                          |     c_e_pub = msg1[0..32]
  |                                                          |     ↓
  |                                                          |   Mix into hash:
  |                                                          |     h = HASH(h || c_e_pub)
  |                                                          |     ↓
  |                                                          |   Store: c_e_pub for later DH
  |                                                          |
  |                                                          |
  | ═══════════════════════════════════════════════════════════════════
  | Message 2: <- e, ee, s, es
  | ═══════════════════════════════════════════════════════════════════
  |                                                          |
  |                                                          | [Responder Actions]:
  |                                                          |   ↓
  |                                                          |   Generate ephemeral keypair:
  |                                                          |     g_e_priv, g_e_pub = X25519::generate()
  |                                                          |     ↓
  |                                                          |   [e] Mix ephemeral public into hash:
  |                                                          |     h = HASH(h || g_e_pub)
  |                                                          |     payload = g_e_pub
  |                                                          |     ↓
  |                                                          |   [ee] Compute ECDH (ephemeral-ephemeral):
  |                                                          |     ee = DH(g_e_priv, c_e_pub)
  |                                                          |     (chaining_key, _) = HKDF(
  |                                                          |       chaining_key,
  |                                                          |       ee,
  |                                                          |       2 outputs
  |                                                          |     )
  |                                                          |     ↓
  |                                                          |   [s] Encrypt gateway static public:
  |                                                          |     // Derive temp key from chaining_key
  |                                                          |     (_, key) = HKDF(chaining_key, ..., 2)
  |                                                          |     ↓
  |                                                          |     encrypted_g_s = AEAD_ENCRYPT(
  |                                                          |       key: key,
  |                                                          |       nonce: 0,
  |                                                          |       plaintext: g_s_pub,
  |                                                          |       aad: h
  |                                                          |     )
  |                                                          |     → 32 bytes payload + 16 bytes tag = 48 bytes
  |                                                          |     ↓
  |                                                          |     h = HASH(h || encrypted_g_s)
  |                                                          |     payload = payload || encrypted_g_s
  |                                                          |     ↓
  |                                                          |   [es] Compute ECDH (ephemeral-static):
  |                                                          |     es = DH(g_e_priv, c_s_pub)
  |                                                          |     (chaining_key, _) = HKDF(
  |                                                          |       chaining_key,
  |                                                          |       es,
  |                                                          |       2 outputs
  |                                                          |     )
  |                                                          |     ↓
  |                                                          |   Build message:
  |                                                          |     msg2 = g_e_pub (32) || encrypted_g_s (48)
  |                                                          |     → Total: 80 bytes
  |                                                          |     ↓
  |                                                          |   Send:
  |                                                          |
  |<─ msg2: [g_e_pub (32)] + [encrypted_g_s (48)] ───────────| send_packet()
  |                                                          |
  | [Initiator Actions]:                                     |
  |   ↓                                                      |
  |   Extract:                                               |
  |     g_e_pub = msg2[0..32]                                |
  |     encrypted_g_s = msg2[32..80]                         |
  |     ↓                                                    |
  |   [e] Mix gateway ephemeral into hash:                   |
  |     h = HASH(h || g_e_pub)                               |
  |     ↓                                                    |
  |   [ee] Compute ECDH (ephemeral-ephemeral):               |
  |     ee = DH(c_e_priv, g_e_pub)                           |
  |     (chaining_key, _) = HKDF(chaining_key, ee, 2)        |
  |     ↓                                                    |
  |   [s] Decrypt gateway static public:                     |
  |     (_, key) = HKDF(chaining_key, ..., 2)                |
  |     ↓                                                    | 
  |     decrypted_g_s = AEAD_DECRYPT(                        |
  |       key: key,                                          |
  |       nonce: 0,                                          |
  |       ciphertext: encrypted_g_s,                         |
  |       aad: h                                             |
  |     )                                                    |
  |     ↓                                                    |
  |     if decrypted_g_s != g_s_pub (known):                 |
  |       ❌ ERROR: Gateway authentication failed            |
  |     ✓ Gateway authenticated                              |
  |     ↓                                                    |
  |     h = HASH(h || encrypted_g_s)                         |
  |     ↓                                                    |
  |   [es] Compute ECDH (static-ephemeral):                  |
  |     es = DH(c_s_priv, g_e_pub)                           |
  |     (chaining_key, _) = HKDF(chaining_key, es, 2)        |
  |                                                          |
  |                                                          |
  | ═══════════════════════════════════════════════════════════════════
  | Message 3: -> s, se, psk
  | ═══════════════════════════════════════════════════════════════════
  |                                                          |
  | [Initiator Actions]:                                     |
  |   ↓                                                      |
  |   [s] Encrypt client static public:                      |
  |     (_, key) = HKDF(chaining_key, ..., 2)                |
  |     ↓                                                    |
  |     encrypted_c_s = AEAD_ENCRYPT(                        |
  |       key: key,                                          |
  |       nonce: 0,                                          |
  |       plaintext: c_s_pub,                                |
  |       aad: h                                             |
  |     )                                                    |
  |     → 32 bytes payload + 16 bytes tag = 48 bytes         |
  |     ↓                                                    |
  |     h = HASH(h || encrypted_c_s)                         |
  |     ↓                                                    |
  |   [se] Compute ECDH (static-ephemeral):                  |
  |     se = DH(c_s_priv, g_e_pub)                           |
  |     (chaining_key, _) = HKDF(chaining_key, se, 2)        |
  |     ↓                                                    |
  |   [psk] Mix in pre-shared key:                           |
  |     (chaining_key, temp_key) = HKDF(                     |
  |       chaining_key,                                      |
  |       psk,  ← PRE-SHARED KEY                             |
  |       2 outputs                                          |
  |     )                                                    |
  |     ↓                                                    |
  |     h = HASH(h || temp_key)                              |
  |     ↓                                                    |
  |   Build message:                                         |
  |     msg3 = encrypted_c_s (48 bytes)                      |
  |     ↓                                                    |
  |   Send:                                                  |
  |                                                          |
  |-- msg3: [encrypted_c_s (48)] ───────────────────────────>| [Responder Actions]:
  |                                                          |   ↓
  |                                                          |   Extract:
  |                                                          |     encrypted_c_s = msg3[0..48]
  |                                                          |     ↓
  |                                                          |   [s] Decrypt client static public:
  |                                                          |     (_, key) = HKDF(chaining_key, ..., 2)
  |                                                          |     ↓
  |                                                          |     decrypted_c_s = AEAD_DECRYPT(
  |                                                          |       key: key,
  |                                                          |       nonce: 0,
  |                                                          |       ciphertext: encrypted_c_s,
  |                                                          |       aad: h
  |                                                          |     )
  |                                                          |     ↓
  |                                                          |     if decrypted_c_s != c_s_pub (from ClientHello):
  |                                                          |       ❌ ERROR: Client authentication failed
  |                                                          |     ✓ Client authenticated
  |                                                          |     ↓
  |                                                          |     h = HASH(h || encrypted_c_s)
  |                                                          |     ↓
  |                                                          |   [se] Compute ECDH (ephemeral-static):
  |                                                          |     se = DH(g_e_priv, c_s_pub)
  |                                                          |     (chaining_key, _) = HKDF(chaining_key, se, 2)
  |                                                          |     ↓
  |                                                          |   [psk] Mix in pre-shared key:
  |                                                          |     (chaining_key, temp_key) = HKDF(
  |                                                          |       chaining_key,
  |                                                          |       psk,  ← PRE-SHARED KEY (same as client!)
  |                                                          |       2 outputs
  |                                                          |     )
  |                                                          |     ↓
  |                                                          |     h = HASH(h || temp_key)
  |                                                          |     ↓
  |                                                          |     if PSKs differ, decryption would fail
  |                                                          |     ✓ PSK implicitly verified
  |                                                          |
  |                                                          |
  | ═══════════════════════════════════════════════════════════════════
  | Handshake Complete: Derive Transport Keys
  | ═══════════════════════════════════════════════════════════════════
  |                                                          |
  | [Split chaining_key into transport keys]:                | [Split chaining_key into transport keys]:
  |   (client_to_server_key, server_to_client_key) =         |   (client_to_server_key, server_to_client_key) =
  |     HKDF(chaining_key, empty, 2 outputs)                 |     HKDF(chaining_key, empty, 2 outputs)
  |   ↓                                                      |   ↓
  | tx_cipher = ChaCha20Poly1305::new(client_to_server_key)  | rx_cipher = ChaCha20Poly1305::new(client_to_server_key)
  | rx_cipher = ChaCha20Poly1305::new(server_to_client_key)  | tx_cipher = ChaCha20Poly1305::new(server_to_client_key)
  |   ↓                                                      |   ↓
  | tx_nonce = 0                                             | rx_nonce = 0
  | rx_nonce = 0                                             | tx_nonce = 0
  |   ↓                                                      |   ↓
  | ✅ Transport mode established                            | ✅ Transport mode established
  |                                                          |
  |                                                          |
  | [Security Properties Achieved]:                          |
  |──────────────────────────────────────────────────────────|
  |                                                          |
  | ✅ Mutual authentication:                                |
  |   • Gateway authenticated via (s) in msg2                |
  |   • Client authenticated via (s) in msg3                 |
  |                                                          |
  | ✅ Forward secrecy:                                      |
  |   • Ephemeral keys (c_e, g_e) destroyed after handshake  |
  |   • Compromise of static keys doesn't decrypt past sessions
  |                                                          |
  | ✅ PSK strengthening:                                    |
  |   • Even if X25519 is broken, PSK protects against MITM  |
  |   • PSK derived from separate ECDH + salt                |
  |                                                          |
  | ✅ Key confirmation:                                     |
  |   • Both sides prove knowledge of PSK                    |
  |   • AEAD auth tags verify all steps                      |
  |                                                          |
```

**Code References**:
- Noise protocol impl: `common/nym-lp/src/noise_protocol.rs:40-88`
- State machine: `common/nym-lp/src/state_machine.rs:96-420`
- Session management: `common/nym-lp/src/session.rs:45-180`

---

## 7. PSK Derivation Flow

**Detailed cryptographic derivation**

```
Client Side                                             Gateway Side
  |                                                          |
  | [Inputs]                                                 | [Inputs]
  |──────────────────────────────────────────────────────────|
  |                                                          |
  | • client_static_keypair:                                 | • gateway_ed25519_identity:
  |     - secret_key: [32 bytes] X25519                      |     - secret_key: [32 bytes] Ed25519
  |     - public_key: [32 bytes] X25519                      |     - public_key: [32 bytes] Ed25519
  |   ↓                                                      |   ↓
  | • gateway_ed25519_public: [32 bytes]                     | [Convert Ed25519 → X25519]:
  |   (from gateway identity)                                |   gateway_lp_keypair = ed25519_to_x25519(
  |   ↓                                                      |     gateway_ed25519_identity
  | [Convert Ed25519 → X25519]:                              |   )
  |   gateway_x25519_public = ed25519_to_x25519(             |   ↓
  |     gateway_ed25519_public                               | • gateway_lp_keypair:
  |   )                                                      |     - secret_key: [32 bytes] X25519
  |   ↓                                                      |     - public_key: [32 bytes] X25519
  | • salt: [32 bytes] (from ClientHello)                    |   ↓
  |                                                          | • client_x25519_public: [32 bytes]
  |                                                          |   (from ClientHello)
  |                                                          |   ↓
  |                                                          | • salt: [32 bytes] (from ClientHello)
  |                                                          |
  |                                                          |
  | [Step 1: ECDH Shared Secret]                             | [Step 1: ECDH Shared Secret]
  |──────────────────────────────────────────────────────────|
  |                                                          |
  | shared_secret = ECDH(                                    | shared_secret = ECDH(
  |   client_static_keypair.secret_key,                      |   gateway_lp_keypair.secret_key,
  |   gateway_x25519_public                                  |   client_x25519_public
  | )                                                        | )
  |   ↓                                                      |   ↓
  | // X25519 scalar multiplication:                         | // X25519 scalar multiplication:
  | //   shared_secret = client_secret * gateway_public      | //   shared_secret = gateway_secret * client_public
  | //                 = client_secret * gateway_secret * G  | //                 = gateway_secret * client_secret * G
  | //   (commutative!)                                      | //   (same result!)
  |   ↓                                                      |   ↓
  | shared_secret: [32 bytes]                                | shared_secret: [32 bytes] (IDENTICAL to client!)
  | Example: 0x7a3b9f2c...                                   | Example: 0x7a3b9f2c... (same)
  |                                                          |
  |                                                          |
  | [Step 2: Blake3 Key Derivation Function]                 | [Step 2: Blake3 Key Derivation Function]
  |──────────────────────────────────────────────────────────|
  |                                                          |
  | // Initialize Blake3 in keyed mode                       | // Initialize Blake3 in keyed mode
  | hasher = Blake3::new_keyed(PSK_KDF_KEY)                  | hasher = Blake3::new_keyed(PSK_KDF_KEY)
  |   where PSK_KDF_KEY = b"nym-lp-psk-kdf-v1-key-32bytes!"  |   where PSK_KDF_KEY = b"nym-lp-psk-kdf-v1-key-32bytes!"
  |   (hardcoded 32-byte domain separation key)              |   (hardcoded 32-byte domain separation key)
  |   ↓                                                      |   ↓
  | // Update with context string (domain separation)        | // Update with context string
  | hasher.update(b"nym-lp-psk-v1")                          | hasher.update(b"nym-lp-psk-v1")
  |   → 13 bytes context                                     |   → 13 bytes context
  |   ↓                                                      |   ↓
  | // Update with shared secret                             | // Update with shared secret
  | hasher.update(shared_secret.as_bytes())                  | hasher.update(shared_secret.as_bytes())
  |   → 32 bytes ECDH output                                 |   → 32 bytes ECDH output
  |   ↓                                                      |   ↓
  | // Update with salt (freshness per-session)              | // Update with salt
  | hasher.update(&salt)                                     | hasher.update(&salt)
  |   → 32 bytes random salt                                 |   → 32 bytes random salt
  |   ↓                                                      |   ↓
  | // Total hashed: 13 + 32 + 32 = 77 bytes                 | // Total hashed: 77 bytes
  |   ↓                                                      |   ↓
  |                                                          |
  |                                                          |
  | [Step 3: Extract PSK (32 bytes)]                         | [Step 3: Extract PSK (32 bytes)]
  |──────────────────────────────────────────────────────────|
  |                                                          |
  | // Finalize in XOF (extendable output function) mode     | // Finalize in XOF mode
  | xof = hasher.finalize_xof()                              | xof = hasher.finalize_xof()
  |   ↓                                                      |   ↓
  | // Read exactly 32 bytes                                 | // Read exactly 32 bytes
  | psk = [0u8; 32]                                          | psk = [0u8; 32]
  | xof.fill(&mut psk)                                       | xof.fill(&mut psk)
  |   ↓                                                      |   ↓
  | psk: [32 bytes]                                          | psk: [32 bytes] (IDENTICAL to client!)
  | Example: 0x4f8a1c3e...                                   | Example: 0x4f8a1c3e... (same)
  |   ↓                                                      |   ↓
  |                                                          |
  | ✅ PSK derived successfully                              | ✅ PSK derived successfully
  |                                                          |
  | [psk.rs:28-52]                                           | [psk.rs:28-52]
  |                                                          |
  |                                                          |
  | [Properties of This Scheme]                              |
  |──────────────────────────────────────────────────────────|
  |                                                          |
  | ✅ Session uniqueness:                                   |
  |   • Fresh salt per connection → unique PSK per session   |
  |   • Even with same keypairs, PSK changes each time       |
  |                                                          |
  | ✅ Perfect forward secrecy (within PSK derivation):      |
  |   • Salt is ephemeral (generated once, never reused)     |
  |   • Compromise of static keys + old salt still needed    |
  |                                                          |
  | ✅ Authenticated key agreement:                          |
  |   • Only parties with correct keypairs derive same PSK   |
  |   • MITM cannot compute shared_secret without private keys
  |                                                          |
  | ✅ Domain separation:                                    |
  |   • Context "nym-lp-psk-v1" prevents cross-protocol attacks
  |   • PSK_KDF_KEY ensures output is LP-specific            |
  |                                                          |
  | ✅ Future-proof:                                         |
  |   • Version in context allows protocol upgrades          |
  |   • Blake3 is quantum-resistant hash function            |
  |                                                          |
```

**Code References**:
- PSK derivation: `common/nym-lp/src/psk.rs:28-52`
- Keypair conversion: `common/nym-lp/src/keypair.rs`
- Constants: `common/nym-lp/src/psk.rs:15-26`

---

## 8. Message Format Specifications

### 8.1. Packet Framing (Transport Layer)

**All LP messages use length-prefixed framing over TCP**:

```
┌────────────────┬─────────────────────────────────┐
│   4 bytes      │       N bytes                   │
│  (u32 BE)      │     (packet data)               │
│  packet_len    │   serialized LpPacket           │
└────────────────┴─────────────────────────────────┘

Example:
  [0x00, 0x00, 0x00, 0x50]  → packet_len = 80 (decimal)
  [... 80 bytes of bincode-serialized LpPacket ...]
```

**Code**: `nym-registration-client/src/lp_client/client.rs:333-431`

---

### 8.2. LpPacket Structure

**All LP messages wrapped in `LpPacket`**:

```rust
struct LpPacket {
    header: LpHeader,
    message: LpMessage,
}

struct LpHeader {
    session_id: u32,       // Assigned by gateway after handshake
    sequence_number: u32,  // Monotonic counter (used as AEAD nonce)
    flags: u8,             // Reserved for future use
}

enum LpMessage {
    ClientHello(ClientHelloData),
    Handshake(Vec<u8>),           // Noise handshake messages
    EncryptedData(Vec<u8>),       // Encrypted registration/response
    Busy,                         // Gateway at capacity
}
```

**Serialization**: bincode (binary, compact)

**Code**: `common/nym-lp/src/packet.rs:15-82`, `common/nym-lp/src/message.rs:12-64`

---

### 8.3. ClientHello Message

**Sent first (cleartext), establishes PSK parameters**:

```rust
struct ClientHelloData {
    client_public_key: [u8; 32],  // X25519 public key
    salt: [u8; 32],               // Random salt for PSK derivation
    timestamp: u64,               // Unix timestamp (seconds)
    protocol_version: u8,         // Always 1 for now
}
```

**Wire format** (bincode):
```
┌─────────────────────────────────────────────────────────┐
│  Offset  │  Size  │  Field                               │
├──────────┼────────┼──────────────────────────────────────┤
│  0       │  32    │  client_public_key                   │
│  32      │  32    │  salt                                │
│  64      │  8     │  timestamp (u64 LE)                  │
│  72      │  1     │  protocol_version (u8)               │
├──────────┴────────┴──────────────────────────────────────┤
│  Total: 73 bytes                                         │
└─────────────────────────────────────────────────────────┘
```

**Code**: `common/nym-lp/src/message.rs:66-95`

---

### 8.4. Noise Handshake Messages

**Encapsulated in `LpMessage::Handshake(Vec<u8>)`**:

**Message 1** (-> e):
```
┌─────────────────────────┐
│  32 bytes               │
│  client_ephemeral_pub   │
└─────────────────────────┘
```

**Message 2** (<- e, ee, s, es):
```
┌──────────────────────────┬─────────────────────────────────┐
│  32 bytes                │  48 bytes                       │
│  gateway_ephemeral_pub   │  encrypted_gateway_static_pub   │
│                          │  (32 payload + 16 auth tag)     │
└──────────────────────────┴─────────────────────────────────┘
Total: 80 bytes
```

**Message 3** (-> s, se, psk):
```
┌─────────────────────────────────┐
│  48 bytes                       │
│  encrypted_client_static_pub    │
│  (32 payload + 16 auth tag)     │
└─────────────────────────────────┘
```

**Code**: `common/nym-lp/src/noise_protocol.rs:40-88`

---

### 8.5. LpRegistrationRequest

**Sent encrypted after handshake complete**:

```rust
struct LpRegistrationRequest {
    wg_public_key: [u8; 32],              // WireGuard public key
    credential: CredentialSpendingData,   // E-cash credential (~200-300 bytes)
    ticket_type: TicketType,              // Enum (1 byte)
    mode: RegistrationMode,               // Enum: Dvpn or Mixnet{client_id}
    client_ip: IpAddr,                    // 4 bytes (IPv4) or 16 bytes (IPv6)
    timestamp: u64,                       // Unix timestamp (8 bytes)
}

enum RegistrationMode {
    Dvpn,
    Mixnet { client_id: [u8; 32] },
}

struct CredentialSpendingData {
    nullifier: [u8; 32],
    signature: Vec<u8>,         // BLS12-381 signature (~96 bytes)
    bandwidth_amount: u64,
    expiry: u64,
    // ... other fields
}
```

**Approximate size**: 300-500 bytes (depends on credential size)

**Code**: `common/registration/src/lp_messages.rs:10-85`

---

### 8.6. LpRegistrationResponse

**Sent encrypted from gateway**:

```rust
struct LpRegistrationResponse {
    success: bool,                     // 1 byte
    error: Option<String>,             // Variable (if error)
    gateway_data: Option<GatewayData>, // ~100 bytes (if success)
    allocated_bandwidth: i64,          // 8 bytes
    session_id: u32,                   // 4 bytes
}

struct GatewayData {
    public_key: [u8; 32],              // WireGuard public key
    endpoint: String,                  // "ip:port" (variable)
    private_ipv4: Ipv4Addr,            // 4 bytes
    private_ipv6: Ipv6Addr,            // 16 bytes
}
```

**Typical size**:
- Success response: ~150-200 bytes
- Error response: ~50-100 bytes (depends on error message length)

**Code**: `common/registration/src/lp_messages.rs:87-145`

---

### 8.7. Encrypted Data Format

**After handshake, all data encrypted with ChaCha20-Poly1305**:

```
Plaintext:
  ┌────────────────────────────────┐
  │  N bytes                       │
  │  serialized message            │
  └────────────────────────────────┘

Encryption:
  ciphertext = ChaCha20Poly1305::encrypt(
    key: transport_key,           // Derived from Noise handshake
    nonce: sequence_number,       // From LpHeader
    plaintext: message_bytes,
    aad: header_bytes             // LpHeader as additional auth data
  )

Ciphertext:
  ┌────────────────────────────────┬─────────────────┐
  │  N bytes                       │  16 bytes       │
  │  encrypted message             │  auth tag       │
  └────────────────────────────────┴─────────────────┘
```

**Code**: `common/nym-lp/src/state_machine.rs:250-350`

---

## Summary

This document provides complete technical specifications for:

1. **Happy Path**: Full successful dVPN registration flow
2. **Error Scenarios**: Timestamp, credential, handshake, and WireGuard failures
3. **Noise Handshake**: Cryptographic operations and authentication
4. **PSK Derivation**: Detailed key derivation flow
5. **Message Formats**: Byte-level packet specifications

**All flows include**:
- Exact message formats
- Cryptographic operations
- Database operations
- Error handling
- Code references (file:line)
- Metrics emitted

---

**Document Version**: 1.0
**Last Updated**: 2025-11-11
**Maintainer**: @drazen
