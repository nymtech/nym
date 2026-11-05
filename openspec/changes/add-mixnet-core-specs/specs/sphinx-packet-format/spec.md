# sphinx-packet-format Spec Delta

## ADDED Requirements

### Requirement: Every packet of one size class has the same length

r[sphinx.size-classes]
The client SHALL build each Sphinx packet in one of five size classes. The length of each class SHALL be its plaintext size plus `SPHINX_PACKET_OVERHEAD`, which is the Sphinx header size (348 bytes) plus the payload overhead (17 bytes), 365 bytes in all. `PacketSize::RegularPacket` SHALL be the default. Acknowledgements SHALL use `PacketSize::AckPacket`. The tags 6 and 7, for the deprecated Outfox sizes, SHALL be rejected.

| `PacketSize` (tag) | Plaintext bytes | Total bytes |
|---|---|---|
| `RegularPacket` (1) | 2,048 | 2,413 |
| `AckPacket` (2) | 21 | 386 |
| `ExtendedPacket32` (3) | 32,768 | 33,133 |
| `ExtendedPacket8` (4) | 8,192 | 8,557 |
| `ExtendedPacket16` (5) | 16,384 | 16,749 |

The header size, the payload overhead and the maximum path length come from the `sphinx-packet` crate, version 0.8.0, which `Cargo.toml` pins.

Code: `common/nymsphinx/params/src/packet_sizes.rs#PacketSize`, `common/nymsphinx/params/src/packet_sizes.rs#PacketSize::size`, `common/nymsphinx/params/src/packet_sizes.rs#PacketSize::try_from`.

#### Scenario: A regular packet

- **WHEN** the client builds a packet with the default configuration
- **THEN** the packet is 2,413 bytes long, whatever the length of the message in it

#### Scenario: An Outfox size tag

- **WHEN** a node decodes a frame whose size tag is 6 or 7
- **THEN** decoding fails

### Requirement: A route has one mix from each of three layers, then the egress gateway

r[sphinx.route]
The client SHALL build a forward route with one node from each of the three mix layers of the rewarded set, in the order layer 1, layer 2, layer 3, and then the gateway of the recipient as the egress. In each layer, `NymTopology::choose_mixing_node` SHALL choose the node at random. The entry gateway of the sender SHALL NOT be a Sphinx hop: the client sends the packet to its gateway with the layer 1 mix as the next hop. When `disable_mix_hops` is set, the route of data packets, SURB-ACKs and reply SURBs SHALL hold the egress only. Loop cover packets SHALL keep the full route, because `generate_loop_cover_packet` always calls `random_route_to_egress`.

Code: `common/topology/src/lib.rs#NymTopology::random_mix_path_nodes`, `common/topology/src/lib.rs#NymTopology::random_route_to_egress`, `common/nymsphinx/src/preparer/mod.rs#FragmentPreparer::prepare_chunk_for_sending`.

#### Scenario: The default route

- **WHEN** the client prepares a packet with the default configuration
- **THEN** the Sphinx route has four nodes: three mixes, one from each layer, and the egress gateway

#### Scenario: Mix hops are off

- **WHEN** `disable_mix_hops` is set
- **THEN** the Sphinx route has one node, the egress gateway, and no mix sees the packet

### Requirement: A forward hop learns the next hop, its delay, and the version from the header

r[sphinx.forward-hop]
Each node SHALL derive its keys for a packet from an X25519 exchange between the group element in the header and its own Sphinx private key, expanded with HKDF-SHA256. It SHALL check a 16-byte HMAC-SHA256 over the encrypted routing information before it processes the packet. A forward hop SHALL learn the address of the next hop, the delay for this hop, and the Sphinx version. The final hop SHALL learn the recipient address and the SURB-ACK, and SHALL recover the payload. Each node also reads the size tag, the packet type and the key rotation from the frame header. Each node SHALL blind the group element before it forwards the packet, and SHALL pad the routing information so that the forwarded header keeps its length of 348 bytes.

The MAC covers the routing information only. It does not cover the payload. A node learns the previous hop from the address of the TCP connection, not from the packet.

Code: the `sphinx-packet` crate 0.8.0 (`SphinxHeader::compute_expanded_shared_secret`, `SphinxHeader::ensure_header_integrity`, `RawRoutingInformation::parse_as_forward_hop`); `common/nymsphinx/framing/src/processing.rs#process_forward_hop`.

#### Scenario: A tampered header

- **WHEN** a node receives a packet whose encrypted routing information has changed in transit
- **THEN** the MAC check fails, and the node drops the packet

### Requirement: Each payload fills its packet

r[sphinx.payload-fills]
The client SHALL pad each message to fill whole packets, as message, then one byte with the value 1, then zero bytes. It SHALL split the padded message into fragments, and SHALL put each fragment into one packet as the SURB-ACK, then the variant data, then the fragment encrypted with AES-128-CTR. For a regular packet, the variant data SHALL be an ephemeral X25519 public key, and the fragment key SHALL come from an exchange between that key and the recipient's encryption key. The Sphinx layer SHALL add one Lioness layer to the payload for each hop.

Code: `common/nymsphinx/src/message.rs#NymMessage::pad_to_full_packet_lengths`, `common/nymsphinx/src/preparer/payload.rs#NymPayloadBuilder::build`, `common/nymsphinx/src/preparer/payload.rs#NymPayloadBuilder::build_regular`.

#### Scenario: A short message

- **WHEN** an application sends a message of 10 bytes
- **THEN** the client sends one packet of the full size of its class, and the padding fills the rest

### Requirement: A node rejects a frame it cannot decode

r[sphinx.frame-rejection]
A node SHALL reject a frame whose packet version is lower than 7 or higher than `CURRENT_PACKET_VERSION` (8), and a frame with an unknown size tag, packet type, or key rotation byte. A node SHALL reject the deprecated Outfox packet type. Nodes SHALL emit the 3-byte version 7 frame header while `use_legacy_packet_encoding` is true, which is the default.

Code: `common/nymsphinx/framing/src/packet.rs#Header::decode`, `common/nymsphinx/framing/src/codec.rs#NymCodec::decode`.

#### Scenario: A future version

- **WHEN** a node receives a frame with a packet version higher than 8
- **THEN** decoding fails, and the node does not process the packet

### Requirement: A node drops a packet whose replay tag it has seen

r[sphinx.replay-drop]
After the MAC check, a mix node SHALL check the 32-byte replay tag of each packet against the bloom filter of the rotation of the key that unwrapped the packet, and SHALL drop the packet when the filter has seen the tag. When the node holds no filter for that rotation, the check SHALL treat the tag as not seen. The default false-positive rate of the filter SHALL be `DEFAULT_REPLAY_DETECTION_FALSE_POSITIVE_RATE`, 1e-5. The node SHALL skip the check when `unsafe_disabled` is set, and SHALL forward a replayed packet that comes from an authorised network-monitor agent.

Code: `nym-node/src/node/mixnet/handler.rs#ConnectionHandler::handle_received_packet_with_replay_detection`, `nym-node/src/node/replay_protection/bloomfilter.rs#ReplayProtectionBloomfiltersInner::batch_check_and_set`.

#### Scenario: A replayed packet

- **WHEN** a mix node receives a packet whose replay tag is in the filter of its rotation, from a peer that is not an authorised network monitor
- **THEN** the node drops the packet and counts a replay

### Requirement: Each data packet carries an acknowledgement packet for its fragment

r[sphinx.ack-packet]
The client SHALL put a SURB-ACK in each data packet: the first-hop address of the acknowledgement, padded with zero bytes, and an `AckPacket` Sphinx packet whose payload is a 16-byte IV and the fragment id encrypted with AES-128-CTR. The egress SHALL split the SURB-ACK from the message and forward the acknowledgement packet.

Code: `common/nymsphinx/acknowledgements/src/surb_ack.rs#SurbAck::construct`, `common/nymsphinx/framing/src/processing.rs#split_into_ack_and_message`.

#### Scenario: A packet reaches its egress

- **WHEN** the egress gateway processes the final hop of a data packet
- **THEN** it forwards the acknowledgement packet to its first hop, and delivers the message
