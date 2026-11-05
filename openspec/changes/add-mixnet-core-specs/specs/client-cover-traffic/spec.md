# client-cover-traffic Spec Delta

## ADDED Requirements

### Requirement: A loop cover packet is a Sphinx packet that the client sends to its own address

r[cover.loop-own-address]
`generate_loop_cover_packet` SHALL build each loop cover packet with the client's own full address as the recipient, through a route that `random_route_to_egress` selects. The payload SHALL be the constant `LOOP_COVER_MESSAGE_PAYLOAD`, then one byte with the value 1, then zero bytes up to the payload size of the packet. The client SHALL encrypt the payload with a key from an ephemeral X25519 exchange against its own encryption key. The SURB-ACK of the packet SHALL carry `COVER_FRAG_ID`.

Code: `common/nymsphinx/cover/src/lib.rs#generate_loop_cover_packet`, `common/client-core/src/client/cover_traffic_stream.rs#LoopCoverTrafficStream::on_new_message`.

#### Scenario: The recipient of a loop cover packet

- **WHEN** the loop cover stream builds a packet
- **THEN** the recipient of the packet is the client's own address, and the packet goes through the mixnet and back to the client

### Requirement: The loop cover stream sends at times drawn from an exponential distribution

r[cover.loop-exponential]
The loop cover stream SHALL draw each interval to its next packet with `sample_poisson_duration`, which samples an exponential distribution whose mean is `loop_cover_traffic_average_delay`. The default mean SHALL be `DEFAULT_LOOP_COVER_STREAM_AVERAGE_DELAY`, 200 ms.

Code: `common/client-core/src/client/cover_traffic_stream.rs#LoopCoverTrafficStream::poll_next`, `common/nymsphinx/src/utils/mod.rs#sample_poisson_duration`, `common/client-core/config-types/src/lib.rs#CoverTraffic::default`.

#### Scenario: The default mean interval

- **WHEN** a client starts with the default configuration
- **THEN** the loop cover stream sends one packet each 200 ms on average, with exponential intervals

### Requirement: The main stream sends a cover packet in each slot that has no real message

r[cover.main-stream-slots]
The main stream SHALL send one packet in each slot of a Poisson schedule. In each slot, it SHALL send the next queued real message if one exists, and otherwise a cover packet built with `generate_loop_cover_packet`. The default mean interval SHALL be `DEFAULT_MESSAGE_STREAM_AVERAGE_DELAY`, 20 ms.

Code: `common/client-core/src/client/real_messages_control/real_traffic_stream.rs#OutQueueControl::poll_poisson`, `common/client-core/src/client/real_messages_control/real_traffic_stream.rs#OutQueueControl::on_message`.

#### Scenario: An idle client

- **WHEN** no real message is queued at the time of a slot
- **THEN** the main stream sends a cover packet in that slot, so the client keeps sending while it is idle

#### Scenario: A queued message

- **WHEN** a real message is queued at the time of a slot
- **THEN** the main stream sends the real message in that slot, and no cover packet

### Requirement: Cover packets and real packets use the same format with the default configuration

r[cover.same-format]
With the default configuration, a cover packet and a real packet SHALL have the same size (`PacketSize::RegularPacket`), SHALL be built with `NymPacket::sphinx_build`, SHALL take their per-hop delays from `generate_hop_delays` with the same `average_packet_delay`, and SHALL take their route from `random_route_to_egress`. The two can differ when a secondary packet size is set, because cover then picks its size at random with the ratio `cover_traffic_primary_size_ratio` (default 0.70), and real traffic picks its size from the message length. They can also differ when `disable_mix_hops` or `deterministic_route_selection` is set, because those settings change the route of real packets only.

Code: `common/nymsphinx/cover/src/lib.rs#generate_loop_cover_packet`, `common/nymsphinx/src/preparer/mod.rs#FragmentPreparer::prepare_chunk_for_sending`, `common/client-core/config-types/src/lib.rs#Traffic::default`.

#### Scenario: Default sizes

- **WHEN** the configuration sets no secondary packet size
- **THEN** each cover packet and each real packet is a regular packet of `2 * 1024 + SPHINX_PACKET_OVERHEAD` bytes

#### Scenario: A secondary size is set

- **WHEN** an integrator sets a secondary packet size
- **THEN** the size of a cover packet comes from a random draw, and the size of a real packet comes from its length, so the two size distributions can differ

### Requirement: The client drops its own loop cover packets after it decrypts them

r[cover.drop-own-loop]
When a packet arrives at the client, the client SHALL decrypt its payload with its own key and SHALL drop the packet when `is_cover` finds the prefix `LOOP_COVER_MESSAGE_PAYLOAD`. The client SHALL ignore acknowledgements that carry `COVER_FRAG_ID`. Only the holder of the client's key can make this check, because it needs the decrypted payload.

Code: `common/client-core/src/client/received_buffer.rs#ReceivedMessagesBufferInner::recover_from_fragment`, `common/nymsphinx/cover/src/lib.rs#is_cover`.

#### Scenario: A loop cover packet returns

- **WHEN** a loop cover packet returns to the client that sent it
- **THEN** the client drops it and passes nothing to the application

### Requirement: Both cover streams are on by default, and named settings turn them off

r[cover.defaults-and-settings]
The default configuration SHALL set `disable_loop_cover_traffic_stream` to false and `disable_main_poisson_packet_distribution` to false. `Config::set_no_cover_traffic` SHALL set both to true. When `disable_loop_cover_traffic_stream` is true, the client SHALL NOT start the loop cover stream. When `disable_main_poisson_packet_distribution` is true, the main stream SHALL send real messages at once and SHALL NOT send cover packets.

These settings turn one or both streams off, or slow them, in code today:

- `set_no_cover_traffic`, used by the `no_cover` argument of the native and SOCKS5 clients: both streams off.
- `set_no_cover_traffic_with_keepalive` and `set_experimental_medium_toggle`: main stream off, loop interval 5 s.
- `set_high_default_traffic_volume`: loop interval 2,000 s.
- The `disable_poisson_rate: true` default of the network requester, the IP packet router and the nym-node service providers: main stream off.
- The two-hop configuration of the registration client: both streams off.
- `no_cover_debug` of the WASM client: both streams off.

Code: `common/client-core/config-types/src/lib.rs#Config::set_no_cover_traffic`, `common/client-core/src/client/base_client/mod.rs#BaseClientBuilder::start_base`, `common/client-core/src/client/real_messages_control/real_traffic_stream.rs#OutQueueControl::poll_next_message`.

#### Scenario: A client with no cover

- **WHEN** a native client starts with the `no_cover` argument
- **THEN** it sends neither loop cover packets nor main-stream cover packets

#### Scenario: A service provider

- **WHEN** a network requester starts with its default configuration
- **THEN** its main stream sends no cover packets, and its loop cover stream runs

### Requirement: The send rate can be lower than the configured rate

r[cover.rate-can-drop]
The main stream SHALL multiply its mean interval by a factor from `MIN_DELAY_MULTIPLIER` (1) to `MAX_DELAY_MULTIPLIER` (6), which the sending-delay controller raises when the channel to the gateway fills. When the network topology is not valid, the loop cover stream SHALL skip its packet, and the main stream SHALL skip a slot that would carry a cover packet. A slot that carries a real message is not skipped for this reason. The loop cover stream SHALL drop a packet with no error when the channel to the gateway is full.

Code: `common/client-core/src/client/real_messages_control/real_traffic_stream.rs#OutQueueControl::current_average_message_sending_delay`, `common/client-core/src/client/cover_traffic_stream.rs#LoopCoverTrafficStream::on_new_message`.

#### Scenario: Backpressure

- **WHEN** the channel to the gateway stays full
- **THEN** the mean interval of the main stream rises, up to six times the configured value

### Requirement: Cover traffic stops when the client stops

r[cover.stops-with-client]
Both streams SHALL stop when the client's shutdown token is cancelled. The mix traffic controller SHALL cancel that token after `MAX_FAILURE_COUNT` (20) consecutive failures to send to the gateway. Before that point, the loop cover stream SHALL continue when a send fails.

Code: `common/client-core/src/client/mix_traffic/mod.rs#MixTrafficController::run`, `common/client-core/src/client/real_messages_control/real_traffic_stream.rs#OutQueueControl::run`.

#### Scenario: The gateway is gone

- **WHEN** 20 sends to the gateway fail in a row
- **THEN** the client cancels its shutdown token, and both cover streams stop
