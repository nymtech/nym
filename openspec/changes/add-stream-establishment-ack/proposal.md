# Add Stream Establishment Acknowledgement

## Why

The SDK stream module (`sdk/rust/nym-sdk/src/mixnet/stream/`) is fire-and-forget. `open_stream` verifies at dial time that the recipient's gateway is routable, sends `Open`, and returns a usable `MixnetStream`; nothing ever confirms that the peer's client is running or that anyone called `accept()`.

This cannot be fixed at a lower layer. Every Sphinx fragment already carries a prepaid SURB-ack, but the recipient's gateway forwards that ack whether the message was delivered to a connected client, stored to disk for an offline one, or dropped because the recipient never registered (`nym-node/src/node/mixnet/handler.rs`, `handle_final_hop`). The code comments state the reason: an ack conditional on recipient state would let anyone who knows an address probe whether that client is online, without its participation. The ack layer is deliberately liveness-blind. Establishment confirmation must therefore be built at the stream layer, where the recipient actively consents by responding.

## What Changes

- Add `OpenAck = 2` to `SphinxStreamMsgType` in `common/nym-lp-data`, with a parse arm and wire tests. Wire-compatible: a peer running current code drops the unknown discriminant cleanly during frame parsing.
- `MixnetListener::accept()` sends a best-effort `OpenAck` to the dialer through the existing anonymous reply path (the dialer's supplied SURBs), immediately after registering the inbound stream. A failed send never fails the accept.
- An `established` watch channel per stream. `MixnetStream::wait_established(timeout)` waits for it; the caller supplies the timeout rather than the SDK imposing a default. `open_stream` itself stays non-blocking. A timeout is reported as inconclusive (older peer, SURB starvation, or an unreachable peer) and the stream stays usable afterwards.
- Inbound `Data` also resolves `wait_established`: data on the stream proves the peer accepted it, covering a lost ack.
- A passive `MixnetStream::last_peer_activity()` getter reporting the time of the most recent inbound frame; calling it generates no traffic.
- Documentation requirement: the SURB-ack privacy rationale (the recipient's gateway forwards acks regardless of recipient state precisely so they cannot become an online-status oracle, so liveness must live at the stream layer where the recipient consents by responding) is recorded in the stream module documentation. It motivates the whole design, including the keepalive follow-up in `add-stream-keepalive`.

## Capabilities

### New Capabilities

- `sdk-mixnet-stream`: establishment acknowledgement for the SDK stream module: an `OpenAck` frame type, an opt-in wait for establishment, per-frame SURB allocation for `Open` and `Data`, and the documentation recording why acknowledgement lives at the stream layer.

### Modified Capabilities

<!-- none: no existing spec covers the stream module -->

## Impact

- `common/nym-lp-data/src/packet/frame.rs`: one enum variant and its parse arm; existing variants and layout untouched.
- `sdk/rust/nym-sdk/src/mixnet/stream/mod.rs`: `OpenAck` router arm, `established` watch state on `StreamEntry`, `accept()` ack send.
- `sdk/rust/nym-sdk/src/mixnet/stream/mixnet_stream.rs`: `wait_established(timeout)`, `last_peer_activity`.
- `sdk/rust/nym-sdk/src/mixnet/native_client.rs`: one doc-comment line. No configuration, no SURB-count changes, and `common/nymsphinx` is untouched.
- `sdk/rust/nym-sdk/src/mixnet/stream/ARCHITECTURE.md`: establishment section with the SURB-ack privacy rationale.
- Outstanding: stream tutorial updates (unchecked in tasks.md).
- Wire compatibility: an old peer never sends the ack; `wait_established` times out and the stream stays usable. An old dialer against a new acceptor drops the incoming `OpenAck` silently, at the cost of one SURB from its pool.
- Follow-up: `add-stream-keepalive` builds on this change to add mid-stream liveness (ping/pong, in-band unresponsive-peer failure).
