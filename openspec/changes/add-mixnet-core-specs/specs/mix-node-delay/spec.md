# mix-node-delay Spec Delta

## ADDED Requirements

### Requirement: The sender draws one exponential delay for each hop

The client SHALL draw the delay for each hop of a packet from an exponential distribution with the mean `average_packet_delay`, one independent sample for each node of the route, rounded to whole nanoseconds. The default mean SHALL be `DEFAULT_AVERAGE_PACKET_DELAY`, 15 ms. When the mean is zero, each delay SHALL be zero. The samples SHALL come from the thread random number generator of the `rand` crate, not from the generator that the preparer receives, so `deterministic_route_selection` does not fix them.

Code: `common/nymsphinx/routing/src/lib.rs#generate_hop_delays`, the `sphinx-packet` crate 0.8.0 (`generate_delays`), `common/client-core/config-types/src/lib.rs#Traffic::default`.

#### Scenario: The default mean

- **WHEN** a client prepares a packet with the default configuration
- **THEN** each hop delay in the packet is an independent sample from an exponential distribution with a mean of 15 ms

#### Scenario: A zero mean

- **WHEN** `average_packet_delay` is zero
- **THEN** each hop delay is zero

### Requirement: Only the node that processes a hop can read its delay

The client SHALL put the delay of each forward hop into the routing information of that hop, which is encrypted and covered by the MAC of that hop. The delay SHALL be 8 bytes, a big-endian count of nanoseconds. The final node of the route SHALL get no delay: the client draws one and discards it.

Code: the `sphinx-packet` crate 0.8.0 (`EncapsulatedRoutingInformation::for_forward_hops`, `Delay::to_bytes`).

#### Scenario: An observer on a link

- **WHEN** an observer records a packet on the link between two mixes
- **THEN** the packet does not show the delay that the next mix applies, because the delay is inside encrypted routing information

### Requirement: Acknowledgements, replies and cover use the sender's delays

A SURB-ACK packet SHALL take its hop delays from `average_ack_delay`, whose default SHALL be `DEFAULT_AVERAGE_PACKET_DELAY`, 15 ms. A reply SURB SHALL hold the hop delays that its creator drew with the creator's `average_packet_delay` when it built the SURB. The party that uses the SURB SHALL NOT be able to change them. A loop cover packet SHALL take its hop delays from `average_packet_delay`, as a data packet does.

Code: `common/nymsphinx/acknowledgements/src/surb_ack.rs#SurbAck::construct`, `common/nymsphinx/src/preparer/mod.rs#MessagePreparer::generate_reply_surbs`, `common/nymsphinx/cover/src/lib.rs#generate_loop_cover_packet`, `common/client-core/config-types/src/lib.rs#Acknowledgements::default`.

#### Scenario: A destination replies through a SURB

- **WHEN** a destination sends a reply through a SURB that a client made
- **THEN** each mix on the return route holds the reply for the delay that the client drew, and the destination cannot set a shorter one

### Requirement: A mix node holds each forward packet until its delay ends

A mix node SHALL set the release time of a forward packet to the time at which it finishes the replay check of the packet's batch, plus the smaller of the header delay and `maximum_forward_packet_delay`. When the replay filter is busy, a packet can wait up to `maximum_replay_detection_deferral` (default 50 ms) or until 100 packets are pending before that check, so its delay starts late. The default of `maximum_forward_packet_delay` SHALL be 10 s. A node SHALL clamp a larger delay to that value and SHALL NOT drop the packet for it. A node SHALL forward a packet at once when its release time has passed before it can queue the packet, and SHALL otherwise hold the packet in its delay queue until the release time. A node that does not run in mixnode mode SHALL drop forward packets.

The node trusts the delay in the header. It does not check a minimum delay, and it does not check that the delays follow an exponential distribution.

Code: `nym-node/src/node/mixnet/handler.rs#ConnectionHandler::create_delay_target`, `nym-node/src/node/mixnet/packet_forwarding/mod.rs#PacketForwarder::handle_new_packet`, `nym-node/src/node/mixnet/handler.rs#ConnectionHandler::handle_forward_packet`.

#### Scenario: A delay above the cap

- **WHEN** a mix node receives a packet whose header delay is 60 s, with the default configuration
- **THEN** the node holds the packet for 10 s and forwards it, and counts an excessive delay

#### Scenario: A delay that has already passed

- **WHEN** processing takes longer than the header delay of a packet
- **THEN** the node forwards the packet at once and records the overrun

### Requirement: Gateways do not delay packets

The entry gateway SHALL forward each packet from its client with no delay. The egress SHALL forward the acknowledgement that it splits from a packet with no delay. So of the four Sphinx hops of a default route, only the three mixes hold a packet.

Code: `gateway/src/node/client_handling/websocket/connection_handler/authenticated.rs#AuthenticatedHandler::forward_packet`, `common/client-libs/mixnet-client/src/forwarder.rs#MixForwardingSender::forward_client_packet_without_delay`, `nym-node/src/node/mixnet/shared/mod.rs#SharedData::forward_ack_packet`.

#### Scenario: A packet from a client

- **WHEN** an entry gateway receives a packet from its client
- **THEN** it forwards the packet to the layer 1 mix at once

### Requirement: Named settings remove or reduce the delay

`Config::set_no_per_hop_delays` SHALL set `average_packet_delay` and `average_ack_delay` to zero. `Config::set_experimental_medium_toggle`, and the `--medium-toggle` option of the SOCKS5 client, SHALL call it. `Config::set_high_default_traffic_volume` SHALL set `average_packet_delay` to 10 ms and SHALL NOT change `average_ack_delay`. In the two-hop configuration of the registration client, `disable_mix_hops` SHALL be true, so the route holds no mix and no packet gets a mix delay.

Code: `common/client-core/config-types/src/lib.rs#Config::set_no_per_hop_delays`, `common/client-core/config-types/src/lib.rs#Config::set_high_default_traffic_volume`, `nym-registration-client/src/builder/config.rs#two_hop_debug_config`.

#### Scenario: The medium toggle

- **WHEN** a SOCKS5 client starts with `--medium-toggle`
- **THEN** each hop delay of its data packets and its acknowledgements is zero
