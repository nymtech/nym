# sdk-mixnet-stream Spec Delta

## ADDED Requirements

### Requirement: Establishment acknowledgement
`MixnetListener::accept()` SHALL send an `OpenAck` frame to the dialer through the stream's anonymous reply path immediately after the inbound stream is registered. The ack SHALL be best-effort: a failed send is logged and the accepted stream is returned regardless.

#### Scenario: Listener acknowledges an accepted stream
- **WHEN** a remote `Open` is accepted and the stream registered
- **THEN** an `OpenAck` for that stream id is sent via the dialer's supplied SURBs

#### Scenario: Ack failure does not lose the stream
- **WHEN** the `OpenAck` send fails (for example, no reply SURBs available)
- **THEN** `accept()` still returns the usable stream and logs the failure

### Requirement: Opt-in establishment wait
`MixnetStream` SHALL provide `wait_established(timeout)`, resolving when the stream's `OpenAck`, or any subsequent frame from the peer, arrives. The caller supplies the timeout; the SDK SHALL NOT impose a default. `open_stream` SHALL NOT block on establishment. A timeout SHALL be reported as inconclusive (the peer may run an older SDK, be SURB-starved, or be unreachable) and SHALL leave the stream usable.

#### Scenario: Ack arrives
- **WHEN** the `OpenAck` for an outbound stream is received
- **THEN** pending and subsequent `wait_established` calls resolve successfully

#### Scenario: Peer data establishes when the ack was lost
- **WHEN** the `OpenAck` never arrives but a `Data` frame from the peer does
- **THEN** `wait_established` resolves, because data on the stream proves the peer accepted it

#### Scenario: Timeout is inconclusive
- **WHEN** `wait_established` times out
- **THEN** it returns a distinct timeout error and reads and writes on the stream continue to work

### Requirement: Wire compatibility for the establishment ack
`OpenAck` SHALL be carried in the existing `SphinxStream` LP frame layout with unchanged header size and field positions, such that a peer running code without this change rejects it during attribute parsing and drops it without error, stream disruption, or state creation. Decoding SHALL likewise reject unknown future discriminants cleanly.

#### Scenario: Unknown discriminant is dropped cleanly
- **WHEN** a frame whose msg-type discriminant is not recognised is received
- **THEN** it is dropped as a non-stream message with no effect on any stream

### Requirement: Passive liveness introspection
`MixnetStream` SHALL expose a getter reporting the time of the most recent inbound activity from the peer. Calling it SHALL generate no mixnet traffic.

#### Scenario: Getter reflects inbound frames
- **WHEN** any frame for the stream is received
- **THEN** a subsequent call to the getter reflects that activity

### Requirement: Liveness model documentation
The stream module documentation SHALL record why establishment acknowledgement is implemented at the stream layer: the SURB-ack layer forwards acknowledgements regardless of recipient state precisely so delivery acks cannot become an online-status oracle, so acknowledgement requires the recipient's active consent by responding. The documentation SHALL state that the acknowledgement consumes one of the reply SURBs the dialer already attaches to the `Open`.

#### Scenario: Architecture documentation explains the layer choice
- **WHEN** a developer reads the stream `ARCHITECTURE.md`
- **THEN** it explains the gateway's unconditional ack forwarding, the oracle risk of a conditional ack, and the consent-based stream-layer design, with a reference to the gateway's final-hop handling
