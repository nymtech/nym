# sdk-mixnet-stream Spec Delta

## ADDED Requirements

### Requirement: Automatic keepalive pings armed idle streams only
The stream router SHALL send a `Ping` (nonce in the sequence-number field, empty payload) on an armed outbound stream that has received no inbound frame for the fixed ping interval (`DEFAULT_PING_INTERVAL`, 60 seconds). An unarmed stream SHALL NOT be pinged, and a stream with inbound activity within the interval SHALL NOT be pinged. Any received frame for a stream SHALL reset its liveness clock and refresh its idle-reaper activity. Liveness frames SHALL be sent without blocking the router: a frame that does not fit the input channel is deferred to the next sweep and SHALL NOT count as a miss. The nonce SHALL stay fixed while a ping is unanswered, so a pong slower than one interval still matches.

#### Scenario: Idle armed stream is pinged
- **WHEN** an armed outbound stream receives nothing for a full ping interval
- **THEN** the router sends one `Ping` with a fresh nonce

#### Scenario: Active stream generates no keepalive traffic
- **WHEN** data frames keep arriving on a stream
- **THEN** no `Ping` is sent for it

#### Scenario: Congested input channel defers rather than stalls
- **WHEN** the shared input channel is full at sweep time
- **THEN** the ping is skipped without counting a miss and the router keeps demultiplexing; the send is retried next sweep

#### Scenario: Unarmed stream is never pinged
- **WHEN** a stream's peer has never sent an `OpenAck`, `Ping`, or `Pong` (for example a stream to an IP packet router)
- **THEN** the stream never arms and no `Ping` is ever sent on it

### Requirement: Pong attests stream registration
The router SHALL answer a received `Ping` with a `Pong` echoing the nonce, if and only if the stream id is currently registered. Pings for unknown stream ids SHALL be dropped silently and SHALL NOT occupy orphan-buffer slots.

#### Scenario: Registered stream pongs
- **WHEN** a `Ping` arrives for a registered stream
- **THEN** a `Pong` with the same nonce is sent via the stream's reply path

#### Scenario: Removed stream is silent
- **WHEN** a `Ping` arrives for a stream id not in the table
- **THEN** no response is sent and no state is created

### Requirement: Unresponsive peers fail the stream in-band
On a stream where liveness enforcement is armed, after a fixed threshold (`DEFAULT_MISSED_PONGS_THRESHOLD`, 3) of consecutive unanswered pings, the router SHALL deliver `StreamFailure::PeerUnresponsive` through the stream's ordered failure channel, with the same delivery semantics as existing stream failures: `recv()` yields it once in order, `AsyncRead` fails the stream.

#### Scenario: Missed pongs fail an armed stream
- **WHEN** the threshold of consecutive pings on an armed stream elicit no `Pong` and no other inbound frame
- **THEN** the consumer observes `PeerUnresponsive` in-band, after any data received before the failure

#### Scenario: Any inbound frame resets the count
- **WHEN** a data frame arrives after two unanswered pings
- **THEN** the missed-pong count resets and no failure is delivered

#### Scenario: Armed stream regains keepalive after life returns
- **WHEN** an armed stream tripped the miss threshold and any frame from the peer later arrives
- **THEN** keepalive pinging resumes, so a later real death is detected at ping cadence rather than by the idle reaper

### Requirement: Liveness enforcement arms only on proof of the extension
A stream SHALL arm liveness enforcement only after receiving an `OpenAck`, `Pong`, or `Ping` from its peer. An unarmed stream SHALL never be pinged and SHALL never fail, so a peer running an older SDK, or a peer that speaks a different protocol, never causes a spurious failure. Gating pings on arming confines keepalive to peers that have proven they speak the extension.

#### Scenario: Old peers never cause failure
- **WHEN** an outbound stream's peer never sends any liveness frame
- **THEN** the stream never arms, is never pinged, and never receives `PeerUnresponsive`

#### Scenario: First pong arms enforcement
- **WHEN** a stream that has never been armed receives a `Pong`
- **THEN** subsequent missed-pong runs reaching the threshold fail the stream in-band

### Requirement: Wire compatibility for keepalive frames
`Ping` and `Pong` SHALL be carried in the existing `SphinxStream` LP frame layout with unchanged header size and field positions, such that a peer running code without this change rejects them during attribute parsing and drops them without error, stream disruption, or state creation.

#### Scenario: Unknown discriminant is dropped cleanly
- **WHEN** a frame whose msg-type discriminant is not recognised is received
- **THEN** it is dropped as a non-stream message with no effect on any stream

### Requirement: Keepalive SURB allocation
The stream path SHALL attach a count of reply SURBs to `Ping` frames that exceeds the single SURB its `Pong` consumes, while keeping the ping a single Sphinx packet. `DEFAULT_NUMBER_OF_SURBS` and non-stream send paths SHALL be unchanged.

#### Scenario: Keepalive does not deplete the SURB pool
- **WHEN** an idle armed stream completes a ping/pong exchange
- **THEN** the dialer-funded SURB pool at the acceptor does not shrink

### Requirement: Fixed keepalive timing
The ping interval and the missed-pong threshold SHALL be fixed module constants (`DEFAULT_PING_INTERVAL` 60 seconds, `DEFAULT_MISSED_PONGS_THRESHOLD` 3), with no client override. A consumer that needs different timing wraps the stream with its own liveness policy.

#### Scenario: Timing comes from the constants
- **WHEN** the router pings an armed stream and counts missed pongs
- **THEN** it uses the fixed `DEFAULT_PING_INTERVAL` and `DEFAULT_MISSED_PONGS_THRESHOLD` values
