# LP Registration - Component Architecture

**Technical architecture deep-dive**

---

## Table of Contents

1. [System Overview](#1-system-overview)
2. [Gateway Architecture](#2-gateway-architecture)
3. [Client Architecture](#3-client-architecture)
4. [Shared Protocol Library](#4-shared-protocol-library)
5. [Data Flow Diagrams](#5-data-flow-diagrams)
6. [State Machines](#6-state-machines)
7. [Database Schema](#7-database-schema)
8. [Integration Points](#8-integration-points)

---

## 1. System Overview

### High-Level System Diagram

```
┌────────────────────────────────────────────────────────────────────────────┐
│                          EXTERNAL SYSTEMS                                  │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│  ┌─────────────────────┐        ┌──────────────────────┐                   │
│  │  Nym Blockchain     │        │  WireGuard Daemon    │                   │
│  │  (Nyx)              │        │  (wg0 interface)     │                   │
│  │                     │        │                      │                   │
│  │  • E-cash contract  │        │  • Kernel module     │                   │
│  │  • Verification     │        │  • Peer management   │                   │
│  │  keys               │        │  • Tunnel routing    │                   │
│  └──────────┬──────────┘        └─────────┬────────────┘                   │
│             │                              │                               │
└─────────────┼──────────────────────────────┼───────────────────────────────┘
              │                              │
              │ RPC calls                    │ Netlink/ioctl
              │ (credential queries)         │ (peer add/remove)
              │                              │
┌─────────────▼──────────────────────────────▼───────────────────────────────┐
│                          GATEWAY COMPONENTS                                │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│  ┌──────────────────────────────────────────────────────────────────────┐  │
│  │                      nym-node (Gateway Mode)                         │  │
│  │                   gateway/src/node/                                  │  │
│  └──────────────────────────────────────────────────────────────────────┘  │
│             │                              │                               │
│    ┌────────▼──────────┐        ┌─────────▼──────────┐                     │
│    │  LpListener       │        │  Mixnet Listener   │                     │
│    │  (LP Protocol)    │        │  (Traditional)     │                     │
│    │  :41264           │        │  :1789, :9000      │                     │
│    └────────┬──────────┘        └────────────────────┘                     │
│             │                                                              │
│    ┌────────▼────────────────────────────────────────┐                     │
│    │         Shared Gateway Services                 │                     │
│    │  ┌────────────┐  ┌──────────────┐  ┌─────────┐  │                     │
│    │  │ EcashMgr   │  │ WG Controller│  │ Storage │  │                     │
│    │  │ (verify)   │  │ (peer mgmt)  │  │ (SQLite)│  │                     │
│    │  └────────────┘  └──────────────┘  └─────────┘  │                     │ 
│    └─────────────────────────────────────────────────┘                     │
│                                                                            │
└────────────────────────────────────────────────────────────────────────────┘
              ▲
              │ TCP :41264
              │ (LP Protocol)
              │
┌─────────────┴───────────────────────────────────────────────────────────────┐
│                          CLIENT COMPONENTS                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │              Application (nym-gateway-probe, nym-vpn-client)        │    │
│  │                                                                     │    │
│  │  Uses:                                                              │    │
│  │  • nym-registration-client (LP registration)                        │    │
│  │  • nym-bandwidth-controller (e-cash credential acquisition)         │    │
│  │  • wireguard-rs (WireGuard tunnel setup)                            │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│             │                              │                                │
│    ┌────────▼──────────────┐    ┌─────────▼────────────┐                    │
│    │ LpRegistrationClient  │    │ BandwidthController  │                    │
│    │ (LP protocol client)  │    │ (e-cash client)      │                    │
│    └────────┬──────────────┘    └──────────────────────┘                    │
│             │                                                               │
│    ┌────────▼────────────────────────────────────┐                          │
│    │     common/nym-lp (Protocol Library)        │                          │
│    │  • State machine                            │                          │
│    │  • Noise protocol                           │                          │
│    │  • Cryptographic primitives                 │                          │
│    └─────────────────────────────────────────────┘                          │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Code Locations**:
- Gateway: `gateway/src/node/lp_listener/`
- Client: `nym-registration-client/src/lp_client/`
- Protocol: `common/nym-lp/src/`

---

## 2. Gateway Architecture

### 2.1. Gateway Module Structure

```
gateway/src/node/
│
├─ lp_listener/
│  │
│  ├─ mod.rs                  [Main module, config, listener]
│  │  ├─ LpConfig             (Configuration struct)
│  │  ├─ LpHandlerState       (Shared state across connections)
│  │  └─ LpListener           (TCP accept loop)
│  │     └─ run() ───────────────────┐
│  │                                  │
│  ├─ handler.rs              [Per-connection handler]
│  │  └─ LpConnectionHandler  <──────┘ spawned per connection
│  │     ├─ handle()          (Main connection lifecycle)
│  │     ├─ receive_client_hello()
│  │     ├─ validate_timestamp()
│  │     └─ [emit metrics]
│  │
│  ├─ registration.rs         [Business logic]
│  │  ├─ process_registration()    (Mode router: dVPN/Mixnet)
│  │  ├─ register_wg_peer()        (WireGuard peer setup)
│  │  ├─ credential_verification() (E-cash verification)
│  │  └─ credential_storage_preparation()
│  │
│  └─ handshake.rs (if exists) [Noise handshake helpers]
│
├─ wireguard/                 [WireGuard integration]
│  ├─ peer_controller.rs      (PeerControlRequest handler)
│  └─ ...
│
└─ storage/                   [Database layer]
   ├─ gateway_storage.rs
   └─ models/
```

### 2.2. Gateway Connection Flow

```
[TCP Accept Loop - LpListener::run()]
  ↓
┌────────────────────────────────────────────────────────────────┐
│  loop {                                                        │
│    stream = listener.accept().await?                           │
│    ↓                                                           │
│    if active_connections >= max_connections {                  │
│      send(LpMessage::Busy)                                     │
│      continue                                                  │
│    }                                                           │
│    ↓                                                           │
│    spawn(async move {                                          │
│      LpConnectionHandler::new(stream, state).handle().await    │
│    })                                                          │
│  }                                                             │
└────────────────────────────────────────────────────────────────┘
  ↓ spawned task
┌────────────────────────────────────────────────────────────────┐
│  [LpConnectionHandler::handle()]                               │
│  gateway/src/node/lp_listener/handler.rs:101-216               │
├────────────────────────────────────────────────────────────────┤
│                                                                │
│  [1] Setup                                                     │
│    ├─ Convert gateway ed25519 → x25519                         │
│    ├─ Start metrics timer                                      │
│    └─ inc!(active_lp_connections)                              │
│                                                                │
│  [2] Receive ClientHello                                       │
│    ├─ receive_client_hello(stream).await?                      │
│    │   ├─ Read length-prefixed packet                          │
│    │   ├─ Deserialize ClientHelloData                          │
│    │   ├─ Extract: client_pub, salt, timestamp                 │
│    │   └─ validate_timestamp(timestamp, tolerance)?            │
│    │       → if invalid: inc!(lp_client_hello_failed)          │
│    │                     return Err(...)                       │
│    └─ ✓ ClientHello valid                                      │
│                                                                │
│  [3] Derive PSK                                                │
│    └─ psk = nym_lp::derive_psk(                                │
│          gw_lp_keypair.secret,                                 │
│          client_pub,                                           │
│          salt                                                  │
│        )                                                       │
│                                                                │
│  [4] Noise Handshake                                           │
│    ├─ state_machine = LpStateMachine::new(                     │
│    │     is_initiator: false,  // responder                    │
│    │     local_keypair: gw_lp_keypair,                         │
│    │     remote_pubkey: client_pub,                            │
│    │     psk: psk                                              │
│    │   )                                                       │
│    │                                                           │
│    ├─ loop {                                                   │
│    │    packet = receive_packet(stream).await?                 │
│    │    action = state_machine.process_input(                  │
│    │      ReceivePacket(packet)                                │
│    │    )?                                                     │
│    │    match action {                                         │
│    │      SendPacket(p) => send_packet(stream, p).await?       │
│    │      HandshakeComplete => break                           │
│    │      _ => continue                                        │
│    │    }                                                      │
│    │  }                                                        │
│    │                                                           │
│    ├─ observe!(lp_handshake_duration_seconds, duration)        │
│    └─ inc!(lp_handshakes_success)                              │
│                                                                │
│  [5] Receive Registration Request                              │
│    ├─ packet = receive_packet(stream).await?                   │
│    ├─ action = state_machine.process_input(ReceivePacket(p))   │
│    ├─ plaintext = match action {                               │
│    │     DeliverData(data) => data,                            │ 
│    │     _ => return Err(...)                                  │
│    │   }                                                       │
│    └─ request = bincode::deserialize::<                        │
│          LpRegistrationRequest                                 │
│        >(&plaintext)?                                          │
│                                                                │
│  [6] Process Registration ───────────────┐                     │
│                                          │                     │
└──────────────────────────────────────────┼─────────────────────┘
                                           │
                                           ▼
┌──────────────────────────────────────────────────────────────────┐
│  [process_registration()]                                        │
│  gateway/src/node/lp_listener/registration.rs:136-288            │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  [1] Validate timestamp (second check)                           │
│    └─ if !request.validate_timestamp(30): return ERROR           │
│                                                                  │
│  [2] Match on request.mode                                       │
│    ├─ RegistrationMode::Dvpn ───────────┐                        │
│    │                                     │                       │
│    └─ RegistrationMode::Mixnet{..} ─────┼────────────┐           │
│                                          │           │           │
└──────────────────────────────────────────┼───────────┼───────────┘
                                           │           │
           ┌───────────────────────────────┘           │
           │                                           │
           ▼                                           ▼
┌───────────────────────────────┐      ┌──────────────────────────┐
│  [dVPN Mode]                  │      │  [Mixnet Mode]           │
├───────────────────────────────┤      ├──────────────────────────┤
│                               │      │                          │
│  [A] register_wg_peer()       │      │  [A] Generate client_id  │
│    ├─ Allocate IPs            │      │    from request          │
│    ├─ Create Peer config      │      │                          │
│    ├─ DB: insert_wg_peer()    │      │  [B] Skip WireGuard      │
│    │   → get client_id        │      │                          │
│    ├─ DB: create_bandwidth()  │      │  [C] credential_verify() │
│    ├─ WG: add_peer()          │      │    (same as dVPN)        │
│    └─ Prepare GatewayData     │      │                          │
│                               │      │  [D] Return response     │
│  [B] credential_verification()│      │    (no gateway_data)     │
│    (see below)                │      │                          │
│                               │      └──────────────────────────┘
│  [C] Return response with     │
│    gateway_data               │
│                               │
└───────────────────────────────┘
           │
           ▼
┌─────────────────────────────────────────────────────────────────┐
│  [register_wg_peer()]                                           │
│  gateway/src/node/lp_listener/registration.rs:291-404           │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  [1] Allocate Private IPs                                       │
│    ├─ random_octet = rng.gen_range(1..255)                      │
│    ├─ ipv4 = Ipv4Addr::new(10, 1, 0, random_octet)              │
│    └─ ipv6 = Ipv6Addr::new(0xfd00, 0, ..., random_octet)        │
│                                                                 │
│  [2] Create Peer Config                                         │
│    └─ peer = Peer {                                             │
│          public_key: request.wg_public_key,                     │
│          allowed_ips: [ipv4/32, ipv6/128],                      │
│          persistent_keepalive: Some(25),                        │
│          endpoint: None                                         │
│        }                                                        │
│                                                                 │
│  [3] CRITICAL ORDER - Database Operations                       │
│    ├─ client_id = storage.insert_wireguard_peer(                │
│    │     &peer,                                                 │
│    │     ticket_type                                            │
│    │   ).await?                                                 │
│    │   ↓                                                        │
│    │ SQL: INSERT INTO wireguard_peers                           │
│    │      (public_key, ticket_type, created_at)                 │
│    │      VALUES (?, ?, NOW())                                  │
│    │      RETURNING id                                          │
│    │   → client_id: i64                                         │
│    │                                                            │
│    └─ credential_storage_preparation(                           │
│          ecash_verifier,                                        │
│          client_id                                              │
│        ).await?                                                 │
│        ↓                                                        │
│      SQL: INSERT INTO bandwidth                                 │
│           (client_id, available)                                │
│           VALUES (?, 0)                                         │
│                                                                 │
│  [4] Send to WireGuard Controller                               │
│    ├─ (tx, rx) = oneshot::channel()                             │
│    ├─ wg_controller.send(                                       │
│    │     PeerControlRequest::AddPeer {                          │
│    │       peer: peer.clone(),                                  │
│    │       response_tx: tx                                      │
│    │     }                                                      │
│    │   ).await?                                                 │
│    │                                                            │
│    ├─ result = rx.await?  // Wait for controller response       │
│    │                                                            │
│    └─ if result.is_err() {                                      │
│          // ROLLBACK:                                           │
│          storage.delete_bandwidth(client_id).await?             │
│          storage.delete_wireguard_peer(client_id).await?        │
│          return Err(WireGuardPeerAddFailed)                     │
│        }                                                        │
│                                                                 │
│  [5] Prepare Gateway Data                                       │
│    └─ gateway_data = GatewayData {                              │
│          public_key: wireguard_data.public_key,                 │
│          endpoint: format!("{}:{}", announced_ip, port),        │
│          private_ipv4: ipv4,                                    │
│          private_ipv6: ipv6                                     │
│        }                                                        │
│                                                                 │
│  [6] Return                                                     │
│    └─ Ok((gateway_data, client_id))                             │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
           │
           ▼
┌─────────────────────────────────────────────────────────────────┐
│  [credential_verification()]                                    │
│  gateway/src/node/lp_listener/registration.rs:87-133            │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  [1] Check Mock Mode                                            │
│    └─ if ecash_verifier.is_mock() {                             │
│          inc!(lp_bandwidth_allocated_bytes_total, MOCK_BW)      │
│          return Ok(1073741824)  // 1 GB                         │
│        }                                                        │
│                                                                 │
│  [2] Create Verifier                                            │
│    └─ verifier = CredentialVerifier::new(                       │
│          CredentialSpendingRequest(request.credential),         │
│          ecash_verifier.clone(),                                │
│          BandwidthStorageManager::new(storage, client_id)       │
│        )                                                        │
│                                                                 │
│  [3] Verify Credential (multi-step)                             │
│    └─ allocated_bandwidth = verifier.verify().await?            │
│        ↓                                                        │
│      [Internal Steps]:                                          │
│        ├─ Check nullifier not spent:                            │
│        │   SQL: SELECT COUNT(*) FROM spent_credentials          │
│        │        WHERE nullifier = ?                             │
│        │   if count > 0: return Err(AlreadySpent)               │
│        │                                                        │
│        ├─ Verify BLS signature:                                 │
│        │   if !bls12_381_verify(                                │
│        │     public_key: ecash_verifier.public_key(),           │
│        │     message: hash(gateway_id + bw + expiry),           │
│        │     signature: credential.signature                    │
│        │   ): return Err(InvalidSignature)                      │
│        │                                                        │
│        ├─ Mark nullifier spent:                                 │
│        │   SQL: INSERT INTO spent_credentials                   │
│        │        (nullifier, expiry, spent_at)                   │
│        │        VALUES (?, ?, NOW())                            │
│        │                                                        │
│        └─ Allocate bandwidth:                                   │
│            SQL: UPDATE bandwidth                                │
│                 SET available = available + ?                   │
│                 WHERE client_id = ?                             │
│            → allocated_bandwidth = credential.bandwidth_amount  │
│                                                                 │
│  [4] Update Metrics                                             │
│    ├─ inc_by!(lp_bandwidth_allocated_bytes_total, allocated)    │
│    └─ inc!(lp_credential_verification_success)                  │ 
│                                                                 │
│  [5] Return                                                     │
│    └─ Ok(allocated_bandwidth)                                   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
           │
           │ (Back to process_registration)
           ▼
┌─────────────────────────────────────────────────────────────────┐
│  [Build Success Response]                                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  response = LpRegistrationResponse {                            │
│    success: true,                                               │
│    error: None,                                                 │
│    gateway_data: Some(gateway_data),  // dVPN only              │
│    allocated_bandwidth,                                         │
│    session_id                                                   │
│  }                                                              │
│                                                                 │
│  inc!(lp_registration_success_total)                            │
│  inc!(lp_registration_dvpn_success)  // or mixnet               │
│  observe!(lp_registration_duration_seconds, duration)           │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
           │
           │ (Back to handler)
           ▼
┌─────────────────────────────────────────────────────────────────┐
│  [Send Response]                                                │
│  gateway/src/node/lp_listener/handler.rs:177-211                │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  [1] Serialize                                                  │
│    └─ response_bytes = bincode::serialize(&response)?           │
│                                                                 │
│  [2] Encrypt                                                    │
│    ├─ action = state_machine.process_input(                     │
│    │     SendData(response_bytes)                               │
│    │   )                                                        │
│    └─ packet = match action {                                   │
│          SendPacket(p) => p,                                    │
│          _ => unreachable!()                                    │
│        }                                                        │
│                                                                 │
│  [3] Send                                                       │
│    └─ send_packet(stream, &packet).await?                       │
│                                                                 │
│  [4] Cleanup                                                    │
│    ├─ dec!(active_lp_connections)                               │
│    ├─ inc!(lp_connections_completed_gracefully)                 │
│    └─ observe!(lp_connection_duration_seconds, total_duration)  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

**Code References**:
- Listener: `gateway/src/node/lp_listener/mod.rs:226-289`
- Handler: `gateway/src/node/lp_listener/handler.rs:101-478`
- Registration: `gateway/src/node/lp_listener/registration.rs:58-404`

---

## 3. Client Architecture

### 3.1. Client Module Structure

```
nym-registration-client/src/
│
└─ lp_client/
   ├─ mod.rs               [Module exports]
   ├─ client.rs            [Main client implementation]
   │  ├─ LpRegistrationClient
   │  │  ├─ new()
   │  │  ├─ connect()
   │  │  ├─ perform_handshake()
   │  │  ├─ send_registration_request()
   │  │  ├─ receive_registration_response()
   │  │  └─ [private helpers]
   │  │
   │  ├─ send_packet()     [Packet I/O]
   │  └─ receive_packet()
   │
   └─ error.rs             [Error types]
      └─ LpClientError
```

### 3.2. Client Workflow

```
┌───────────────────────────────────────────────────────────────┐
│  Application (e.g., nym-gateway-probe, nym-vpn-client)       │
└───────────────────────────────────┬───────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────┐
│  [Create LP Client]                                             │
│  nym-registration-client/src/lp_client/client.rs:64-132         │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  let mut client = LpRegistrationClient::new_with_default_psk(   │
│    client_lp_keypair,         // X25519 keypair                 │
│    gateway_lp_public_key,     // X25519 public (from ed25519)   │
│    gateway_lp_address,        // SocketAddr (IP:41264)          │
│    client_ip,                 // Client's IP address            │
│    LpConfig::default()        // Timeouts, TCP_NODELAY, etc.    │
│  );                                                             │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────┐
│  [1] Connect to Gateway                                         │
│  client.rs:133-169                                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  client.connect().await?                                        │
│    ↓                                                            │
│  stream = tokio::time::timeout(                                 │
│    self.config.connect_timeout,  // e.g., 5 seconds             │
│    TcpStream::connect(self.gateway_lp_address)                  │
│  ).await?                                                       │
│    ↓                                                            │
│  stream.set_nodelay(self.config.tcp_nodelay)?  // true          │
│    ↓                                                            │
│  self.tcp_stream = Some(stream)                                 │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────┐
│  [2] Perform Noise Handshake                                    │
│  client.rs:212-325                                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  client.perform_handshake().await?                              │
│    ↓                                                            │
│  [A] Generate ClientHello:                                      │
│    ├─ salt = random_bytes(32)                                   │
│    ├─ client_hello_data = ClientHelloData {                     │
│    │     client_public_key: self.local_keypair.public,          │
│    │     salt,                                                  │
│    │     timestamp: unix_timestamp(),                           │
│    │     protocol_version: 1                                    │ 
│    │   }                                                        │
│    └─ packet = LpPacket {                                       │
│          header: LpHeader { session_id: 0, seq: 0 },            │
│          message: ClientHello(client_hello_data)                │
│        }                                                        │
│                                                                 │
│  [B] Send ClientHello:                                          │
│    └─ Self::send_packet(stream, &packet).await?                 │
│                                                                 │
│  [C] Derive PSK:                                                │
│    └─ psk = nym_lp::derive_psk(                                 │
│          self.local_keypair.private,                            │
│          &self.gateway_public_key,                              │
│          &salt                                                  │
│        )                                                        │
│                                                                 │
│  [D] Create State Machine:                                      │
│    └─ state_machine = LpStateMachine::new(                      │
│          is_initiator: true,                                    │
│          local_keypair: &self.local_keypair,                    │
│          remote_pubkey: &self.gateway_public_key,               │
│          psk: &psk                                              │
│        )?                                                       │
│                                                                 │
│  [E] Exchange Handshake Messages:                               │
│    └─ loop {                                                    │
│          match state_machine.current_state() {                  │
│            WaitingForHandshake =>                               │
│              // Send initial handshake packet                   │
│              action = state_machine.process_input(              │
│                StartHandshake                                   │
│              )?                                                 │
│              packet = match action {                            │
│                SendPacket(p) => p,                              │
│                _ => unreachable!()                              │
│              }                                                  │
│              Self::send_packet(stream, &packet).await?          │
│                                                                 │
│            HandshakeInProgress =>                               │
│              // Receive gateway response                        │
│              packet = Self::receive_packet(stream).await?       │
│              action = state_machine.process_input(              │
│                ReceivePacket(packet)                            │
│              )?                                                 │
│              if let SendPacket(p) = action {                    │
│                Self::send_packet(stream, &p).await?             │
│              }                                                  │
│                                                                 │
│            HandshakeComplete =>                                 │
│              break  // Done!                                    │
│                                                                 │
│            _ => return Err(...)                                 │
│          }                                                      │
│        }                                                        │
│                                                                 │
│  [F] Store State Machine:                                       │
│    └─ self.state_machine = Some(state_machine)                  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────┐
│  [3] Send Registration Request                                  │
│  client.rs:433-507                                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  client.send_registration_request(                              │
│    wg_public_key,                                               │
│    bandwidth_controller,                                        │
│    ticket_type                                                  │
│  ).await?                                                       │
│    ↓                                                            │
│  [A] Acquire Bandwidth Credential:                              │
│    └─ credential = bandwidth_controller                         │
│          .get_ecash_ticket(                                     │
│            ticket_type,                                         │
│            gateway_identity,                                    │
│            DEFAULT_TICKETS_TO_SPEND  // e.g., 1                 │
│          ).await?                                               │
│          .data  // CredentialSpendingData                       │
│                                                                 │
│  [B] Build Request:                                             │
│    └─ request = LpRegistrationRequest::new_dvpn(                │
│          wg_public_key,                                         │
│          credential,                                            │
│          ticket_type,                                           │
│          self.client_ip                                         │
│        )                                                        │
│                                                                 │
│  [C] Serialize:                                                 │
│    └─ request_bytes = bincode::serialize(&request)?             │
│                                                                 │
│  [D] Encrypt via State Machine:                                 │
│    ├─ state_machine = self.state_machine.as_mut()?              │
│    ├─ action = state_machine.process_input(                     │
│    │     LpInput::SendData(request_bytes)                       │
│    │   )?                                                       │
│    └─ packet = match action {                                   │
│          LpAction::SendPacket(p) => p,                          │
│          _ => return Err(...)                                   │
│        }                                                        │
│                                                                 │
│  [E] Send:                                                      │
│    └─ Self::send_packet(stream, &packet).await?                 │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────┐
│  [4] Receive Registration Response                              │
│  client.rs:615-715                                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  gateway_data = client.receive_registration_response().await?   │
│    ↓                                                            │
│  [A] Receive Packet:                                            │
│    └─ packet = Self::receive_packet(stream).await?              │
│                                                                 │
│  [B] Decrypt via State Machine:                                 │
│    ├─ state_machine = self.state_machine.as_mut()?              │
│    ├─ action = state_machine.process_input(                     │
│    │     LpInput::ReceivePacket(packet)                         │
│    │   )?                                                       │
│    └─ response_data = match action {                            │
│          LpAction::DeliverData(data) => data,                   │
│          _ => return Err(UnexpectedAction)                      │
│        }                                                        │
│                                                                 │
│  [C] Deserialize:                                               │
│    └─ response = bincode::deserialize::<                        │
│          LpRegistrationResponse                                 │
│        >(&response_data)?                                       │
│                                                                 │
│  [D] Validate:                                                  │
│    ├─ if !response.success {                                    │
│    │     return Err(RegistrationRejected {                      │
│    │       reason: response.error.unwrap_or_default()           │
│    │     })                                                     │
│    │   }                                                        │
│    └─ gateway_data = response.gateway_data                      │
│          .ok_or(MissingGatewayData)?                            │
│                                                                 │
│  [E] Return:                                                    │
│    └─ Ok(gateway_data)                                          │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────┐
│  [Application: Setup WireGuard Tunnel]                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  // Client now has:                                             │
│  // • gateway_data.public_key (WireGuard public key)            │
│  // • gateway_data.endpoint (IP:port)                           │
│  // • gateway_data.private_ipv4 (10.1.0.x)                      │
│  // • gateway_data.private_ipv6 (fd00::x)                       │
│  // • wg_private_key (from wg_keypair generated earlier)        │
│                                                                 │
│  wg_config = format!(r#"                                        │
│    [Interface]                                                  │
│    PrivateKey = {}                                              │
│    Address = {}/32, {}/128                                      │
│                                                                 │
│    [Peer]                                                       │
│    PublicKey = {}                                               │
│    Endpoint = {}                                                │
│    AllowedIPs = 0.0.0.0/0, ::/0                                 │
│    PersistentKeepalive = 25                                     │
│  "#,                                                            │
│    wg_private_key,                                              │
│    gateway_data.private_ipv4,                                   │
│    gateway_data.private_ipv6,                                   │
│    gateway_data.public_key,                                     │
│    gateway_data.endpoint                                        │
│  )                                                              │
│                                                                 │
│  // Apply config via wg-quick or wireguard-rs                   │
│  wireguard_tunnel.set_config(wg_config).await?                  │
│                                                                 │
│  ✅ VPN tunnel established!                                     │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

**Code References**:
- Client main: `nym-registration-client/src/lp_client/client.rs:39-780`
- Packet I/O: `nym-registration-client/src/lp_client/client.rs:333-431`

---

## 4. Shared Protocol Library

### 4.1. nym-lp Module Structure

```
common/nym-lp/src/
│
├─ lib.rs                    [Public API exports]
│  ├─ pub use session::*
│  ├─ pub use state_machine::*
│  ├─ pub use psk::*
│  └─ ...
│
├─ session.rs                [LP session management]
│  └─ LpSession
│     ├─ new_initiator()
│     ├─ new_responder()
│     ├─ encrypt()
│     ├─ decrypt()
│     └─ [replay validation]
│
├─ state_machine.rs          [Noise protocol state machine]
│  ├─ LpStateMachine
│  │  ├─ new()
│  │  ├─ process_input()
│  │  └─ current_state()
│  │
│  ├─ LpState (enum)
│  │  ├─ WaitingForHandshake
│  │  ├─ HandshakeInProgress
│  │  ├─ HandshakeComplete
│  │  └─ Failed
│  │
│  ├─ LpInput (enum)
│  │  ├─ StartHandshake
│  │  ├─ ReceivePacket(LpPacket)
│  │  └─ SendData(Vec<u8>)
│  │
│  └─ LpAction (enum)
│     ├─ SendPacket(LpPacket)
│     ├─ DeliverData(Vec<u8>)
│     └─ HandshakeComplete
│
├─ noise_protocol.rs         [Noise XKpsk3 implementation]
│  └─ LpNoiseProtocol
│     ├─ new()
│     ├─ build_initiator()
│     ├─ build_responder()
│     └─ into_transport_mode()
│
├─ psk.rs                    [PSK derivation]
│  └─ derive_psk(secret_key, public_key, salt) -> [u8; 32]
│
├─ keypair.rs                [X25519 keypair management]
│  └─ Keypair
│     ├─ generate()
│     ├─ from_bytes()
│     └─ ed25519_to_x25519()
│
├─ packet.rs                 [Packet structure]
│  ├─ LpPacket { header, message }
│  └─ LpHeader { session_id, seq, flags }
│
├─ message.rs                [Message types]
│  └─ LpMessage (enum)
│     ├─ ClientHello(ClientHelloData)
│     ├─ Handshake(Vec<u8>)
│     ├─ EncryptedData(Vec<u8>)
│     └─ Busy
│
├─ codec.rs                  [Serialization]
│  ├─ serialize_lp_packet()
│  └─ parse_lp_packet()
│
└─ replay/                   [Replay protection]
   ├─ validator.rs           [Main validator]
   │  └─ ReplayValidator
   │     ├─ new()
   │     └─ validate(nonce: u64) -> bool
   │
   └─ simd/                  [SIMD optimizations]
      ├─ mod.rs
      ├─ avx2.rs            [AVX2 bitmap ops]
      ├─ sse2.rs            [SSE2 bitmap ops]
      ├─ neon.rs            [ARM NEON ops]
      └─ scalar.rs          [Fallback scalar ops]
```

### 4.2. State Machine State Transitions

```
┌────────────────────────────────────────────────────────────────┐
│                   LP State Machine (Initiator)                 │
├────────────────────────────────────────────────────────────────┤
│                                                                │
│  [Initial State]                                               │
│    WaitingForHandshake                                         │
│      │                                                         │
│      │ Input: StartHandshake                                   │
│      │ Action: SendPacket(Handshake msg 1)                     │
│      ▼                                                         │
│    HandshakeInProgress                                         │
│      │                                                         │
│      │ Input: ReceivePacket(Handshake msg 2)                   │
│      │ Action: SendPacket(Handshake msg 3)                     │
│      │         HandshakeComplete                               │
│      ▼                                                         │
│    HandshakeComplete ──────────────────┐                       │
│      │                                  │                      │
│      │ Input: SendData(plaintext)      │ Input: ReceivePacket  │
│      │ Action: SendPacket(encrypted)   │ Action: DeliverData   │
│      └─────────────┬────────────────────┘                      │
│                    │                                           │
│                    │ (stays in HandshakeComplete)              │
│                    │                                           │
│      ┌─────────────▼────────────────────────┐                  │
│      │  Any state + error input:            │                  │
│      │    → Failed                           │                 │
│      └──────────────────────────────────────┘                  │
│                                                                │
└────────────────────────────────────────────────────────────────┘

┌────────────────────────────────────────────────────────────────┐
│                  LP State Machine (Responder)                  │
├────────────────────────────────────────────────────────────────┤
│                                                                │
│  [Initial State]                                               │
│    WaitingForHandshake                                         │
│      │                                                         │
│      │ Input: ReceivePacket(Handshake msg 1)                   │
│      │ Action: SendPacket(Handshake msg 2)                     │
│      ▼                                                         │
│    HandshakeInProgress                                         │
│      │                                                         │
│      │ Input: ReceivePacket(Handshake msg 3)                   │
│      │ Action: HandshakeComplete                               │
│      ▼                                                         │
│    HandshakeComplete ──────────────────┐                       │
│      │                                  │                      │
│      │ Input: SendData(plaintext)      │ Input: ReceivePacket  │
│      │ Action: SendPacket(encrypted)   │ Action: DeliverData   │
│      └─────────────┬────────────────────┘                      │
│                    │                                           │
│                    │ (stays in HandshakeComplete)              │
│                    │                                           │
└────────────────────────────────────────────────────────────────┘
```

**Code References**:
- State machine: `common/nym-lp/src/state_machine.rs:96-420`
- Session: `common/nym-lp/src/session.rs:45-180`

---

## 5. Data Flow Diagrams

### 5.1. Successful dVPN Registration Data Flow

```
Client                Gateway              DB              WG Controller    Blockchain
  │                      │                  │                    │              │
  │ [TCP Connect]        │                  │                    │              │
  ├─────────────────────>│                  │                    │              │
  │                      │                  │                    │              │
  │ [ClientHello]        │                  │                    │              │
  ├─────────────────────>│                  │                    │              │
  │                      │ [validate time]  │                    │              │
  │                      │                  │                    │              │
  │ [Noise Handshake]    │                  │                    │              │
  │<────────────────────>│                  │                    │              │
  │   (3 messages)       │                  │                    │              │
  │                      │                  │                    │              │
  │ [Encrypted Request]  │                  │                    │              │
  │  • wg_pub_key        │                  │                    │              │
  │  • credential        │                  │                    │              │
  │  • mode: Dvpn        │                  │                    │              │
  ├─────────────────────>│                  │                    │              │
  │                      │ [decrypt]        │                    │              │
  │                      │                  │                    │              │
  │                      │ [register_wg_peer]                    │              │
  │                      │                  │                    │              │
  │                      │ INSERT peer      │                    │              │
  │                      ├─────────────────>│                    │              │
  │                      │ ← client_id: 123 │                    │              │
  │                      │                  │                    │              │
  │                      │ INSERT bandwidth │                    │              │
  │                      ├─────────────────>│                    │              │
  │                      │ ← OK             │                    │              │
  │                      │                  │                    │              │
  │                      │ AddPeer request  │                    │              │
  │                      ├────────────────────────────────────────>             │
  │                      │                  │ wg set wg0 peer... │              │
  │                      │                  │    ← OK            │              │
  │                      │ ← AddPeer OK ────────────────────────┤               │
  │                      │                  │                    │              │
  │                      │ [credential_verification]             │              │
  │                      │                  │                    │              │
  │                      │ SELECT nullifier │                    │              │
  │                      ├─────────────────>│                    │              │
  │                      │ ← count: 0       │                    │              │
  │                      │                  │                    │              │
  │                      │ [verify BLS sig] │                    │              │
  │                      │                  │                    │      [query  │
  │                      │                  │                    │   public key]│
  │                      │                  │                    │<─────────────┤
  │                      │                  │                    │ ← pub_key ───┤
  │                      │                  │                    │              │
  │                      │ ✓ signature OK   │                    │              │
  │                      │                  │                    │              │
  │                      │ INSERT nullifier │                    │              │
  │                      ├─────────────────>│                    │              │
  │                      │ ← OK             │                    │              │
  │                      │                  │                    │              │
  │                      │ UPDATE bandwidth │                    │              │
  │                      ├─────────────────>│                    │              │
  │                      │ ← OK             │                    │              │
  │                      │                  │                    │              │
  │                      │ [build response] │                    │              │
  │                      │ [encrypt]        │                    │              │
  │                      │                  │                    │              │
  │ [Encrypted Response] │                  │                    │              │
  │  • success: true     │                  │                    │              │
  │  • gateway_data      │                  │                    │              │
  │  • allocated_bw      │                  │                    │              │
  │<─────────────────────┤                  │                    │              │
  │                      │                  │                    │              │
  │ [decrypt]            │                  │                    │              │
  │ ✓ Registration OK    │                  │                    │              │
  │                      │                  │                    │              │

[Client sets up WireGuard tunnel with gateway_data]
```

### 5.2. Error Flow: Credential Already Spent

```
Client                Gateway              DB
  │                      │                  │
  │ ... (handshake)...   │                  │
  │                      │                  │
  │ [Encrypted Request]  │                  │
  │  • credential        │                  │
  │    (nullifier reused)│                  │
  ├─────────────────────>│                  │
  │                      │ [decrypt]        │
  │                      │                  │
  │                      │ [credential_verification]
  │                      │                  │
  │                      │ SELECT nullifier │
  │                      ├─────────────────>│
  │                      │ ← count: 1 ✗     │
  │                      │                  │
  │                      │ ✗ AlreadySpent   │
  │                      │                  │
  │                      │ [build error]    │
  │                      │ [encrypt]        │
  │                      │                  │
  │ [Encrypted Response] │                  │
  │  • success: false    │                  │
  │  • error: "Credential│                  │
  │    already spent"    │                  │
  │<─────────────────────┤                  │
  │                      │                  │
  │ ✗ Registration Failed│                  │
  │                      │                  │

[Client must acquire new credential and retry]
```

**Code References**:
- Overall flow: See sequence diagrams in `LP_REGISTRATION_SEQUENCES.md`
- Data structures: `common/registration/src/lp_messages.rs`

---

## 6. State Machines

### 6.1. Replay Protection State

**ReplayValidator maintains sliding window for nonce validation**:

```
┌─────────────────────────────────────────────────────────────────┐
│                     ReplayValidator State                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  struct ReplayValidator {                                       │
│    nonce_high: u64,        // Highest seen nonce                │
│    nonce_low: u64,         // Lowest in window                  │
│    seen_bitmap: [u64; 16]  // Bitmap: 1024 bits total           │
│  }                                                              │
│                                                                 │
│  Window size: 1024 packets                                      │
│  Memory: 144 bytes per session                                  │
│                                                                 │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  [Validation Algorithm]                                         │
│                                                                 │
│  validate(nonce: u64) -> Result<bool> {                         │
│    // Case 1: nonce too old (outside window)                    │
│    if nonce < nonce_low:                                        │
│      return Ok(false)  // Reject: too old                       │
│                                                                 │
│    // Case 2: nonce within current window                       │
│    if nonce <= nonce_high:                                      │
│      offset = (nonce - nonce_low) as usize                      │
│      bucket_idx = offset / 64                                   │
│      bit_idx = offset % 64                                      │
│      bit_mask = 1u64 << bit_idx                                 │
│      ↓                                                          │
│      if seen_bitmap[bucket_idx] & bit_mask != 0:                │
│        return Ok(false)  // Reject: duplicate                   │
│      ↓                                                          │
│      // Mark as seen (SIMD-optimized if available)              │
│      seen_bitmap[bucket_idx] |= bit_mask                        │
│      return Ok(true)  // Accept                                 │
│                                                                 │
│    // Case 3: nonce advances window                             │
│    if nonce > nonce_high:                                       │
│      advance = nonce - nonce_high                               │
│      ↓                                                          │
│      if advance >= 1024:                                        │
│        // Reset entire window                                   │
│        seen_bitmap.fill(0)                                      │
│        nonce_low = nonce                                        │
│        nonce_high = nonce                                       │
│      else:                                                      │
│        // Shift window by 'advance' bits                        │
│        shift_bitmap_left(&mut seen_bitmap, advance)             │
│        nonce_low += advance                                     │
│        nonce_high = nonce                                       │
│      ↓                                                          │
│      // Mark new nonce as seen                                  │
│      offset = (nonce - nonce_low) as usize                      │
│      bucket_idx = offset / 64                                   │
│      bit_idx = offset % 64                                      │
│      seen_bitmap[bucket_idx] |= 1u64 << bit_idx                 │
│      return Ok(true)  // Accept                                 │
│  }                                                              │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘

[Visualization of Sliding Window]

Time ──────────────────────────────────────────────────────────>

Packet nonces:  100  101  102 ... 1123  [1124 arrives]
                 │                  │
                 nonce_low          nonce_high

Bitmap (1024 bits):
  [111111111111...111111111110000000000000000000000]
   ↑ bit 0            ↑ bit 1023 (most recent)
   (nonce 100)        (nonce 1123)

When nonce 1124 arrives:
  1. Shift bitmap left by 1 bit
  2. nonce_low = 101
  3. nonce_high = 1124
  4. Set bit 1023 (for nonce 1124)

Bitmap becomes:
  [11111111111...1111111111100000000000000000000]
   ↑ bit 0              ↑ bit 1023
   (nonce 101)          (nonce 1124)
```

**Code References**:
- Replay validator: `common/nym-lp/src/replay/validator.rs:25-125`
- SIMD ops: `common/nym-lp/src/replay/simd/`

---

## 7. Database Schema

### 7.1. Gateway Database Tables

```sql
-- WireGuard peers table
CREATE TABLE wireguard_peers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,  -- client_id
    public_key BLOB NOT NULL UNIQUE,       -- WireGuard public key [32 bytes]
    ticket_type TEXT NOT NULL,             -- "V1MixnetEntry", etc.
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    last_seen TIMESTAMP,
    INDEX idx_public_key (public_key)
);

-- Bandwidth tracking table
CREATE TABLE bandwidth (
    client_id INTEGER PRIMARY KEY,
    available INTEGER NOT NULL DEFAULT 0,  -- Bytes remaining
    used INTEGER NOT NULL DEFAULT 0,       -- Bytes consumed
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (client_id) REFERENCES wireguard_peers(id)
        ON DELETE CASCADE
);

-- Spent credentials (nullifier tracking)
CREATE TABLE spent_credentials (
    nullifier BLOB PRIMARY KEY,            -- Credential nullifier [32 bytes]
    expiry TIMESTAMP NOT NULL,             -- Credential expiration
    spent_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    client_id INTEGER,                     -- Optional link to client
    FOREIGN KEY (client_id) REFERENCES wireguard_peers(id)
        ON DELETE SET NULL,
    INDEX idx_nullifier (nullifier),       -- Critical for performance!
    INDEX idx_expiry (expiry)              -- For cleanup queries
);

-- LP session tracking (optional, for metrics/debugging)
CREATE TABLE lp_sessions (
    session_id INTEGER PRIMARY KEY,
    client_ip TEXT NOT NULL,
    started_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    completed_at TIMESTAMP,
    status TEXT,  -- "success", "handshake_failed", "credential_rejected", etc.
    client_id INTEGER,
    FOREIGN KEY (client_id) REFERENCES wireguard_peers(id)
        ON DELETE SET NULL
);
```

### 7.2. Database Operations by Component

```
┌─────────────────────────────────────────────────────────────┐
│                  Registration Flow DB Ops                   │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  [1] register_wg_peer()                                     │
│    ├─ INSERT INTO wireguard_peers                           │
│    │    (public_key, ticket_type)                           │
│    │  VALUES (?, ?)                                         │
│    │  RETURNING id                                          │
│    │  → client_id                                           │
│    │                                                        │
│    └─ INSERT INTO bandwidth                                 │
│         (client_id, available)                              │
│       VALUES (?, 0)                                         │
│                                                             │
│  [2] credential_verification()                              │
│    ├─ SELECT COUNT(*) FROM spent_credentials                │
│    │  WHERE nullifier = ?                                   │
│    │  → count (should be 0)                                 │
│    │                                                        │
│    ├─ INSERT INTO spent_credentials                         │
│    │    (nullifier, expiry, client_id)                      │
│    │  VALUES (?, ?, ?)                                      │
│    │                                                        │
│    └─ UPDATE bandwidth                                      │
│         SET available = available + ?,                      │
│             updated_at = NOW()                              │
│       WHERE client_id = ?                                   │
│                                                             │
│  [3] Connection lifecycle (optional)                        │
│    ├─ INSERT INTO lp_sessions                               │
│    │    (session_id, client_ip, status)                     │
│    │  VALUES (?, ?, 'in_progress')                          │
│    │                                                        │
│    └─ UPDATE lp_sessions                                    │
│         SET completed_at = NOW(),                           │
│             status = 'success',                             │
│             client_id = ?                                   │
│       WHERE session_id = ?                                  │
│                                                             │
└─────────────────────────────────────────────────────────────┘

[Cleanup/Maintenance Queries]

-- Remove expired nullifiers (run daily)
DELETE FROM spent_credentials
WHERE expiry < datetime('now', '-30 days');

-- Find stale WireGuard peers (not seen in 7 days)
SELECT p.id, p.public_key, p.last_seen
FROM wireguard_peers p
WHERE p.last_seen < datetime('now', '-7 days');

-- Bandwidth usage report
SELECT
  p.public_key,
  b.available,
  b.used,
  b.updated_at
FROM wireguard_peers p
JOIN bandwidth b ON b.client_id = p.id
ORDER BY b.used DESC
LIMIT 100;
```

**Code References**:
- Database models: Gateway storage module
- Queries: `gateway/src/node/lp_listener/registration.rs`

---

## 8. Integration Points

### 8.1. External System Integration

```
┌──────────────────────────────────────────────────────────────┐
│                  LP Registration Integrations                │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  [1] Blockchain (Nym Chain / Nyx)                            │
│    ├─ E-cash Contract                                        │
│    │  ├─ Query: Get public verification keys                 │
│    │  ├─ Used by: EcashManager in gateway                    │
│    │  └─ Frequency: Cached, refreshed periodically           │
│    │                                                         │
│    └─ Mixnet Contract (optional, future)                     │
│       ├─ Query: Gateway info, capabilities                   │
│       └─ Used by: Client gateway selection                   │
│                                                              │
│  [2] WireGuard Daemon                                        │
│    ├─ Interface: Netlink / wg(8) command                     │
│    │  ├─ AddPeer: wg set wg0 peer <key> allowed-ips ...      │
│    │  ├─ RemovePeer: wg set wg0 peer <key> remove            │ 
│    │  └─ ListPeers: wg show wg0 dump                         │
│    │                                                         │
│    ├─ Used by: WireGuard Controller (gateway)                │
│    ├─ Communication: mpsc channel (async)                    │
│    └─ Frequency: Per registration/deregistration             │
│                                                              │
│  [3] Gateway Storage (SQLite/PostgreSQL)                     │
│    ├─ Tables: wireguard_peers, bandwidth, spent_credentials  │
│    ├─ Used by: LP registration, credential verification      │
│    ├─ Access: SQLx (async, type-safe)                        │
│    └─ Transactions: Required for peer registration           │
│                                                              │
│  [4] Metrics System (Prometheus)                             │
│    ├─ Exporter: Built into nym-node                          │
│    ├─ Endpoint: http://<gateway>:8080/metrics                │
│    ├─ Metrics: lp_* namespace (see main doc)                 │
│    └─ Scrape interval: Typically 15-60s                      │
│                                                              │
│  [5] BandwidthController (Client-side)                       │
│    ├─ Purpose: Acquire e-cash credentials                    │
│    ├─ Methods:                                               │
│    │  └─ get_ecash_ticket(type, gateway, count)              │
│    │     → CredentialSpendingData                            │
│    │                                                         │
│    ├─ Blockchain interaction: Queries + blind signing        │
│    └─ Used by: LP client before registration                 │ 
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### 8.2. Module Dependencies

```
[Gateway Dependencies]

nym-node (gateway mode)
  ├─ gateway/src/node/lp_listener/
  │   ├─ Depends on:
  │   │  ├─ common/nym-lp (protocol library)
  │   │  ├─ common/registration (message types)
  │   │  ├─ gateway/storage (database)
  │   │  ├─ gateway/wireguard (WG controller)
  │   │  └─ common/bandwidth-controller (e-cash verification)
  │   │
  │   └─ Provides:
  │      └─ LP registration service (:41264)
  │
  ├─ gateway/src/node/wireguard/
  │   ├─ Depends on:
  │   │  ├─ wireguard-rs (WG tunnel)
  │   │  └─ gateway/storage (peer tracking)
  │   │
  │   └─ Provides:
  │      ├─ PeerController (mpsc handler)
  │      └─ WireGuard daemon interface
  │
  └─ gateway/src/node/storage/
      ├─ Depends on:
      │  └─ sqlx (database access)
      │
      └─ Provides:
         ├─ GatewayStorage trait
         └─ Database operations

[Client Dependencies]

nym-vpn-client (or other app)
  ├─ nym-registration-client/
  │   ├─ Depends on:
  │   │  ├─ common/nym-lp (protocol library)
  │   │  ├─ common/registration (message types)
  │   │  └─ common/bandwidth-controller (credentials)
  │   │
  │   └─ Provides:
  │      └─ LpRegistrationClient
  │
  ├─ common/bandwidth-controller/
  │   ├─ Depends on:
  │   │  ├─ Blockchain RPC client
  │   │  └─ E-cash cryptography
  │   │
  │   └─ Provides:
  │      ├─ BandwidthController
  │      └─ Credential acquisition
  │
  └─ wireguard-rs/
      ├─ Depends on:
      │  └─ System WireGuard
      │
      └─ Provides:
         └─ Tunnel management

[Shared Dependencies]

common/nym-lp/
  ├─ Depends on:
  │  ├─ snow (Noise protocol)
  │  ├─ x25519-dalek (ECDH)
  │  ├─ chacha20poly1305 (AEAD)
  │  ├─ blake3 (KDF, hashing)
  │  ├─ bincode (serialization)
  │  └─ tokio (async runtime)
  │
  └─ Provides:
     ├─ LpStateMachine
     ├─ LpSession
     ├─ Noise protocol
     ├─ PSK derivation
     ├─ Replay protection
     └─ Message types

common/registration/
  ├─ Depends on:
  │  ├─ serde (serialization)
  │  └─ common/crypto (credential types)
  │
  └─ Provides:
     ├─ LpRegistrationRequest
     ├─ LpRegistrationResponse
     └─ GatewayData
```

**Code References**:
- Gateway dependencies: `gateway/Cargo.toml`
- Client dependencies: `nym-registration-client/Cargo.toml`
- Protocol dependencies: `common/nym-lp/Cargo.toml`

---

## Summary

This document provides complete architectural details for:

1. **System Overview**: High-level component interaction
2. **Gateway Architecture**: Module structure, connection flow, data processing
3. **Client Architecture**: Workflow from connection to WireGuard setup
4. **Shared Protocol Library**: nym-lp module organization and state machines
5. **Data Flow**: Successful and error case flows with database operations
6. **State Machines**: Handshake states and replay protection
7. **Database Schema**: Tables, indexes, and operations
8. **Integration Points**: External systems and module dependencies

**All diagrams include**:
- Component boundaries
- Data flow arrows
- Code references (file:line)
- Database operations
- External system calls

---

**Document Version**: 1.0
**Last Updated**: 2025-11-11
**Maintainer**: @drazen
