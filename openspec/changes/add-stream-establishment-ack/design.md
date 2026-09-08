# Design: Add Stream Establishment Acknowledgement

## Context

The stream module multiplexes byte streams over a `MixnetClient`: a router task demuxes reconstructed mixnet messages by `StreamId` into per-stream channels, with a per-stream reorder buffer (sequence-numbered `Data` frames), an orphan buffer for `Data` overtaking its `Open`, and a stale-stream reaper driven by a cleanup tick. Failures are reported in-band: `StreamFailure` values travel through the same channel as data, so `recv()` sees them in order and `AsyncRead` fails the stream.

Facts established during exploration:

- **The SURB-ack layer cannot provide liveness, by design.** Each fragment carries a prepaid SURB-ack that the recipient's gateway forwards regardless of whether the payload was pushed to a connected client, stored to disk, or dropped for an unregistered recipient (`nym-node/src/node/mixnet/handler.rs`, `handle_final_hop`; the comments state that a conditional ack would leak the recipient's state). It attests "reached the destination gateway", is consumed entirely by the retransmission machinery in `common/client-core/.../acknowledgement_control/`, and is not surfaced to the SDK. Any liveness signal gated on recipient state at this layer would be an online-status oracle usable by anyone who knows the address. Establishment confirmation must come from a layer where the recipient consents by responding.
- **Frame parsing rejects unknown msg types cleanly.** An unknown `SphinxStreamMsgType` discriminant fails `SphinxStreamFrameAttributes::parse`, so `decode_stream_message` returns `None` and the router drops the frame as a non-stream message. `OpenAck` sent to an old peer degrades to silence, never to an error.
- The acceptor's reply path (`InputMessage::new_reply(sender_tag)`) already exists and is reused for the `OpenAck` send.

## Goals / Non-Goals

**Goals:**

- Consumers can distinguish "established" from "unknown" at dial time, without reading logs.
- Old and new SDKs interoperate in both directions with no coordinated upgrade: an old peer simply never emits the frame this change introduces.
- The privacy rationale (why acknowledgement is stream-layer) is recorded in the module documentation.

**Non-Goals:**

- Mid-stream liveness (ping/pong, unresponsive-peer failure, arming): deferred to `add-stream-keepalive`.
- Open retransmission and idempotent re-ack (deferred).
- A `Gone` frame for fast dead-stream signalling (deferred; silence suffices).
- Any change to `DEFAULT_NUMBER_OF_SURBS` or non-stream send paths.

## Decisions

All settled interactively on 2026-08-27:

1. **Ack from `accept()`, not from router receipt.** The signal means "a listener took your stream", which is the question consumers ask. Best-effort: a failed ack send (for example SURB starvation from a `reply_surbs = 0` dialer) never fails the accept. The send uses non-blocking `try_send` on the shared input channel, so a channel that is momentarily full cannot stall `accept()`.
2. **`wait_established` is opt-in; `open_stream` stays non-blocking.** One method, `wait_established(timeout)`, with the caller supplying the budget. No default constant and no builder option: a sensible timeout depends on the application, and the SDK has no basis for choosing one. Timeout is inconclusive by definition (old peer, SURB starvation, or loss) and the stream remains usable after it.
3. **`established` is a distinct signal from keepalive arming.** Inbound `Data` also resolves `wait_established`, because the peer provably accepted the stream, but this does not arm anything; arming is introduced in `add-stream-keepalive`.
4. **`OpenAck` stays a distinct frame type** rather than being folded into a later liveness frame; one meaning per variant.
5. **Passive introspection only**: `last_peer_activity()` exposes the last inbound instant; no active `is_alive()` ping method exists at this layer.

## Backwards compatibility

An old peer does not recognise `OpenAck` and drops it during frame parsing: `wait_established` on the new dialer times out, is reported as inconclusive, and the stream stays usable. An old dialer against a new acceptor receives and silently drops the `OpenAck` the same way, at the cost of one SURB from its pool. Neither direction requires a coordinated upgrade or produces a spurious failure.

## Risks / Trade-offs

- **Multi-fragment `Open`** raises establishment loss-variance slightly (all fragments needed for reconstruction). Nothing times out underneath this `Open` (unlike smoltcp SYN handling over the mixnet), so the cost is latency variance in `wait_established`, not correctness.
- **The ack consumes one reply SURB per stream.** A dialer attaching the default 10 is unaffected; one attaching zero gets no acknowledgement and `wait_established` times out, which the API documents as inconclusive rather than as failure.

## Deliberately out of scope

- **SURB budgeting.** The acknowledgement is paid for out of the reply SURBs
  the dialer already attaches to the `Open`, so no SURB-count change is needed
  to make it work. Sizing `Open` and `Data` SURB counts to the Sphinx packet
  boundary is a behaviour change for every stream caller and belongs in its own
  change, kept on `wip/stream-surb-budgeting` with the measurement and its
  pinning test.
- **Anything in `common/nymsphinx`.** Reply-SURB serialisation is too
  fundamental to modify in service of a stream-layer feature.
- **Mid-stream liveness.** Keepalive is `add-stream-keepalive`.
- **The IPR path.** `IpMixStream::connect_tunnel` already performs a connect
  request and response carrying the allocated IPs, which is an establishment
  handshake one layer up. `OpenAck` would duplicate it. This change targets the
  client-to-client case, where no application protocol supplies one.
