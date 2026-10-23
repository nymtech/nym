# Lewes Protocol (LP)

LP is the transport nym-nodes and clients use to talk to each other directly. It does two jobs:

- **Registration**: a client sets up a session with a gateway and registers on it, either for a
  dVPN (WireGuard) tunnel or for mixnet use.
- **Data plane**: sphinx packets travel between clients and nodes, and between nodes, inside LP
  sessions over UDP. This replaces the websocket and TCP links of the legacy stack.

Both run inside `nym-node`; there is no separate LP binary. LP is still experimental, and the
[known gaps](#known-gaps) at the end are worth reading before relying on it.

## Ports and planes

| Plane   | Transport | Default port | Carries                                                       |
|---------|-----------|--------------|---------------------------------------------------------------|
| Control | TCP       | 41264        | handshakes, registration, telescoped (forwarded) handshakes   |
| Data    | UDP       | 51264        | sphinx packets inside encrypted LP packets                    |

The control plane sets up sessions, and the data plane uses them. Both share one session store, so
a session made over TCP is the one the UDP packets are encrypted under.

## Keys and discovery

Every node publishes a signed LP record on its HTTP API, which nym-api passes on in its node
descriptions:

- `control_port`, `data_port`
- `x25519`: the node's static LP key
- `kem_keys`: digests of its KEM public keys, per KEM and hash function
- `enabled`: always `true`. It stays only because the signed record's format is frozen.

A client never needs a node's full KEM key in advance. It learns the digest from the directory,
receives the key during the handshake, and checks it against that digest.

Inside nym-node and clients, the verified record is used as `LewesProtocolDetailsDataV2` (the same
record without `enabled`). LP has its own topology, `LpTopology`, kept alongside the legacy one and
built from nym-api's listing of the nodes that published LP details. Each `LpRoutingNode` carries
the record together with the node's `build_version`, from which the LP protocol version and the
ciphersuite are inferred. Its rewarded set holds only those nodes, so an LP route is drawn from
nodes it can actually reach.

## Handshake

The handshake runs over the control connection in two parts, four messages in all. Each message
has a size known from the KEM and the mode, so they go on the wire without a length prefix.

1. **KKT request** (initiator → responder). The initiator makes an ephemeral x25519 key, does DH
   with the responder's static LP key, and sends its peer config (including a random seed)
   encrypted under that.
2. **KKT response** (responder → initiator). The responder's full KEM key, encrypted the same way.
   The initiator hashes it and compares it with the directory digest. A mismatch ends the
   handshake.
3. **PSQ msg1** (initiator → responder), built with `libcrux-psq`.
4. **PSQ msg2** (responder → initiator).

Ciphersuites are X25519 combined with ML-KEM-768 (the default) or Classic McEliece, using
AES-GCM-128 and HKDF-SHA256.

Modes (`HandshakeMode`):

| Mode              | Between          | Who has a KEM key                     |
|-------------------|------------------|---------------------------------------|
| `OneWayEntry`     | client → entry   | responder only                        |
| `OneWayExit`      | client → exit    | responder only (telescoped, see below)|
| `MutualInternode` | node → node      | both, each checked against the directory |

A node decides between the client and node handlers by the remote IP: a connection from an address
belonging to a known LP node is treated as that node.

Both sides derive the session's **receiver index** independently, from the seed, the initiator's
x25519 key and the responder's KEM key. It is what the receiver looks sessions up by.

## Packet format

```
EncryptedLpPacket
├── OuterHeader      12 B, cleartext   receiver_idx u32 | counter u64   (little endian)
└── ciphertext
    ├── InnerHeader   4 B              protocol_version u8 | reserved [u8; 3]
    └── LpFrame
        ├── LpFrameHeader 16 B         kind u16 | attributes [u8; 14]
        └── content
```

Encryption adds 26 bytes (8-byte channel id, 16-byte tag, 2-byte length), so an encrypted packet
costs 42 bytes on top of its frame.

Frame kinds:

| Kind                  | Value | Used for                                                    |
|-----------------------|-------|-------------------------------------------------------------|
| `Opaque`              | 0     |                                                             |
| `Registration`        | 1     | registration requests and responses (bincode)               |
| `Forward`             | 2     | telescoping: a packet for the entry to pass on to the exit  |
| `SphinxStream`        | 3     | stream framing inside sphinx payloads (SDK streams, IPR, smolmix) |
| `FragmentedData`      | 4     | one piece of a frame too big for one datagram               |
| `SphinxPacket`        | 5     | a sphinx packet to mix: process, delay, forward             |
| `ForwardSphinxPacket` | 6     | a client's packet for its gateway to forward without mixing |

### Sizes

Datagrams are sized for a path MTU, set in one place: `PATH_MTU` in
`common/nym-lp-data/src/packet/mod.rs`. Everything else follows from it.

| `PathMtu`              | UDP payload | frame | frame payload |
|------------------------|-------------|-------|---------------|
| `Standard` (1500)      | 1452        | 1410  | 1394          |
| `Ipv6Minimum` (1280)   | 1232        | 1190  | 1174          |

A sphinx packet does not fit in one frame, so it travels as two `FragmentedData` frames. A frame
can be split into at most 255 fragments; anything larger is rejected, not truncated.

### Control-plane framing

After the handshake, each packet on the TCP connection is a 4-byte little-endian length followed by
the encrypted packet, up to 64 KiB.

## Registration

Every registration request carries a timestamp, and the node rejects it if it is more than 30
seconds from its own clock.

### dVPN

Two round trips on the same session:

1. `InitialRequest { wg_public_key, psk }`: the gateway pre-allocates an IP pair, remembers the
   pending registration by receiver index, and answers that it needs a credential.
2. `Finalisation { credential }`: the gateway verifies the zk-nym, adds the WireGuard peer, and
   answers with the tunnel config.

### Mixnet

One round trip. The request names the client's ed25519 identity and the node it is meant for, with
a timestamp, all signed with the client's identity key. The node checks the signature, that it is
the node named, and the timestamp, then binds the client's address to the session.

Until that binding the session can only decrypt; it is what lets the node send to the client. After
registering, the client closes the control connection and the data plane carries on with the
session.

### Telescoping

A client can run a second handshake with an exit gateway (`OneWayExit`) through its entry gateway,
wrapped in `Forward` frames. The entry relays the bytes, so the exit only ever sees the entry's IP.
The entry caps concurrent outbound forward connections (`max_concurrent_forwards`).

## Data plane

### On a node

- **Receive.** The UDP listener hands datagrams to a pool of workers (`data_worker_count`). Each
  worker decrypts, checks the replay window, reassembles fragments, and processes the frame:
  `SphinxPacket` is mixed, and `ForwardSphinxPacket` is forwarded by a gateway straight to the
  first mix.
- **Unknown sessions.** A packet naming a session the node does not hold usually means the peer
  restarted, so the node re-dials it. Only known LP nodes are dialled, dials to one peer are
  coalesced, and a session the node just set up is not replaced within `redial_cooldown`, so
  forged packets cannot cost more than one handshake per peer per cooldown.
- **Release.** Processed frames wait unencrypted in per-peer queues until their release time (when
  they arrived plus their sphinx delay). They are encrypted at release, so the cleartext counter
  follows send order and gives nothing away about each packet's delay.
- **No session yet.** A frame for a node without a session triggers a dial at release time, not on
  arrival, so the dial does not reveal when the packet came in. Frames still without a session
  after `stalled_frame_timeout` are dropped.
- **Clients and providers.** Frames for a registered client go to its last-seen address. Frames
  for a service provider hosted by the gateway are handed over in-process, without encryption.

### Sessions

When a newer session for the same peer arrives, the older one is demoted to read-only: it can still
decrypt packets already in flight but no longer sends, and expires after `read_only_session_ttl`.
Other sessions expire after a day without traffic.

Node-to-node dials back off exponentially with jitter after failures, are capped at
`max_concurrent_handshakes`, time out connecting after `dial_connect_timeout`, and are cancelled on
shutdown.

### On a client

The client dials and registers with its gateway at startup, then sends and receives over its data
socket:

- **Outbound:** chunking, then a sphinx route per chunk, then two LP fragments per sphinx packet,
  encrypted on the gateway session.
- **Inbound:** decrypt, reassemble, peel the sphinx layer, reassemble the message.

There are no acknowledgements, retransmission or cover traffic on this path. A lost fragment loses
its message.

It is used through `ClientInput::send_lp` (and `send_lp` in the SDK), and by the socks5 client with
`--use-lp`. A client in LP mode listens only on LP. LP carries no SURBs, so socks5 turns
`send_anonymously` off when `--use-lp` is set.

### Service providers

The authenticator, IPR and network requester hosted in a gateway receive over LP alongside their
legacy mixnet client. LP carries no SURBs yet, so a reply can only go back to a recipient named in
the request:

- the network requester answers when the request names its return address;
- the IPR and the authenticator drop LP requests for now, with the reply paths stubbed out.

## Configuration

### nym-node (`[lp]`)

| Key | Default | |
|-----|---------|-|
| `control_bind_address` | `[::]:41264` | |
| `data_bind_address` | `[::]:51264` | |
| `announce_control_port`, `announce_data_port` | unset | announce a different port than the one bound |

`[lp.debug]`, for when the defaults are not enough:

| Key | Default | |
|-----|---------|-|
| `max_connections` | 10000 | concurrent control connections |
| `handshake_ttl` | 90s | |
| `session_ttl` | 24h | idle, client sessions |
| `internode_session_ttl` | 24h | idle, node sessions |
| `read_only_session_ttl` | 60s | idle, demoted sessions |
| `state_cleanup_interval` | 5m | |
| `max_concurrent_forwards` | 1000 | telescoping connections |
| `max_concurrent_handshakes` | 50 | node-to-node dials in flight |
| `dial_backoff_initial`, `dial_backoff_max` | 1s, 60s | |
| `dial_connect_timeout` | 5s | |
| `redial_cooldown` | 10s | |
| `stalled_frame_timeout` | 5s | |
| `data_worker_count` | 4 | |
| `sp_inbound_worker_count` | 2 | per hosted service provider |

### Clients (`[debug.lewes_protocol]`)

| Key | Default | |
|-----|---------|-|
| `worker_threads` | 4 | per direction |
| `data_socket_addr` | `[::]:0` | made dual-stack, so it reaches IPv4 gateways too |
| `control_socket_addr` | `[::]:0` | |
| `dial_backoff_initial`, `dial_backoff_max` | 1s, 60s | |
| `max_concurrent_handshakes` | 50 | |

Clients build the LP data plane by default; `with_lp(false)` on the builder turns it off.

## Metrics

LP counters are prefixed `lp_`, and the data-plane mixing metrics `mixnet_lp_`. The main groups:

- control connections: `lp_client_connections_*`, `lp_node_connections_*`
- registration: `lp_registration_*`, with a duration histogram
- telescoping: `lp_forward_*`
- data plane: `lp_unknown_session_packets`, `lp_no_session_pending`, `lp_stalled_dropped`,
  `lp_queue_overflow_dropped`, and `mixnet_lp_*` for packets received, forwarded, dropped and why
- cleanup: `lp_states_cleanup_*`

## Known gaps

- **No access control on the data plane.** A session is not checked for its kind or for bandwidth
  before its packets are forwarded
- **Client reachability.** A gateway learns a client's UDP address from its data packets, and
  clients send no keepalive, so a client that only receives, or sits idle behind a NAT, can become
  unreachable. Client sessions are not re-established once they expire.
- **Resource limits.** Fragment reassembly has no memory cap, sessions that never register live for
  a day
- **No SURBs over LP**, which is why service providers can only answer named recipients.
- **Not yet a datagram service.** LP is meant to carry datagrams, with input and output still to
  be defined. Meanwhile it temporarily reuses the legacy sphinx message semantics: arbitrary-size
  messages, chunked and reassembled.

## Code map

| Where | What |
|-------|------|
| `common/nym-lp` | handshake (KKT/PSQ), sessions, replay window, transport traits |
| `common/nym-kkt` | KEM key transfer |
| `common/nym-lp-data` | packet and frame formats, fragmentation, pipeline traits |
| `common/registration` | registration messages |
| `common/client-libs/nym-lp-gateway-client` | client side of the control and data planes, telescoping |
| `common/client-core/src/client/lp` | client data plane and gateway dialer |
| `nym-node/src/node/lp` | node control plane, data plane, dialer, registration |
| `service-providers/common/src/lp` | hosted service providers' LP pipelines |
| `nym-mix-sim` | simulator running the LP pipelines |
