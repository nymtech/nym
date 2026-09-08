# Design: Add Stream Keepalive

## Context

The stream module multiplexes byte streams over a `MixnetClient`: a router task demuxes reconstructed mixnet messages by `StreamId` into per-stream channels, with a per-stream reorder buffer, an orphan buffer for `Data` overtaking its `Open`, and a stale-stream reaper driven by a cleanup tick (`min(stream_idle_timeout, 10 s)`). Failures are reported in-band: `StreamFailure` values travel through the same channel as data, so `recv()` sees them in order and `AsyncRead` fails the stream. `add-stream-establishment-ack` added the `established` watch channel and the `OpenAck` frame type; this change adds mid-stream liveness on top of that plumbing.

Facts established during exploration:

- **The SURB-ack layer cannot provide liveness, by design** (see `add-stream-establishment-ack`). Any liveness signal gated on recipient state at the ack layer would be an online-status oracle; liveness must come from a layer where the recipient consents by responding.
- **Frame parsing rejects unknown msg types cleanly.** An unknown `SphinxStreamMsgType` discriminant fails `SphinxStreamFrameAttributes::parse`, so `decode_stream_message` returns `None` and the router drops the frame as a non-stream message. `Ping` and `Pong` sent to an old peer degrade to silence, never to errors.
- **SURB wire cost**, restated from `add-stream-establishment-ack`: one 4-hop SURB serialises to 460 bytes; usable plaintext per Regular packet is 1610 bytes (1603 per unlinked fragment); the single-packet ceiling for an empty-payload message is 3 SURBs. `PING_SURBS` is sized against this same ceiling: 2 SURBs keeps a ping a single Sphinx packet while still funding more than the one SURB its `Pong` consumes.
- The acceptor's reply path (`InputMessage::new_reply(sender_tag)`) and the reply controller's additional-SURB request machinery (`PacketDestination::Anonymous { extra_surb_request }`) already exist and are reused for pongs.

## Goals / Non-Goals

**Goals:**

- Consumers can distinguish "alive but quiet" from "gone" mid-stream, without reading logs.
- Zero keepalive traffic on active streams; bounded traffic on idle ones that does not deplete the acceptor's SURB pool.
- Old and new SDKs interoperate in both directions with no coordinated upgrade and no spurious failures.

**Non-Goals:**

- Open retransmission and idempotent re-ack (deferred).
- A `Gone` frame for fast dead-stream signalling (deferred; silence suffices).
- Acceptor-initiated pings and acceptor-side ping-deadline enforcement (deferred; v1 acceptors answer pings and observe them passively).
- Re-probing streams misclassified as old-peer (deferred; see Risks).
- Any change to `DEFAULT_NUMBER_OF_SURBS` or non-stream send paths.
- Keepalive for streams that tunnel another protocol (IPR traffic, smolmix TCP/IP, fire-and-forget UDP). These peers do not speak the extension, so their streams never arm and are never pinged; that consumer supplies its own liveness model. This is deliberate confinement, not a deferred case.

## Decisions

All settled interactively on 2026-08-27:

1. **Keepalive is automatic**: `DEFAULT_PING_INTERVAL` 60 s and `DEFAULT_MISSED_PONGS_THRESHOLD` 3 are fixed module constants; a consumer that needs different timing wraps the stream with its own liveness policy. The dialer is the sole active pinger; the router answers pings for registered streams only, so a pong attests "stream still registered at the peer", not "application is reading". The ping nonce rides in the otherwise-unused sequence-number field.
2. **Failure is in-band**: `StreamFailure::PeerUnresponsive` through the existing ordered failure channel, mirroring `DataLoss`.
3. **Arming by observation** is the version negotiation (no version field): keepalive arms per-stream on the first received `OpenAck`, `Pong`, or `Ping`. An unarmed stream is never pinged and never fails, so an old-SDK peer, which sends no liveness frame, never arms and never sees a spurious failure. Gating pings on arming also confines keepalive: only a peer that has proven it speaks the extension is ever pinged.
4. **Liveness frames refresh `last_activity`.** For armed streams the idle reaper (default 30 min) becomes a backstop and `PeerUnresponsive` (about 4 min from the last inbound frame: one interval to the first ping, then the threshold of unanswered intervals) is the effective reaper. This holds only while `stream_idle_timeout` exceeds that horizon; below it the reaper removes the stream first and a dead peer surfaces as a silent reap. The reaper keeps its current meaning for unarmed peers and one-sided deaths. Documented shift.
5. **`PING_SURBS`: 2.** Each ping delivers more SURBs than its pong consumes, so an idle stream with a live peer does not deplete the pool.
6. **Confinement to speaking peers is enforced by arming, not by an opt-out.** There is no keepalive-disabled stream and no opt-out method; the IPR wrapper uses plain `open_stream`. Because a stream is pinged only after it arms, and an IPR speaks a different protocol and never sends an `OpenAck`, `Ping`, or `Pong`, a stream to an IPR never arms and is never pinged. The IPR is left unchanged by design. This is the intended scope: keepalive covers raw `mixnet::stream` used directly between two SDK peers, and a consumer that tunnels another protocol over a stream brings its own liveness model.

## Review round (2026-09-01)

A fresh-agent review of the implementation produced findings relevant to keepalive; all were fixed in this change except the IPR-side one:

1. Router ping and pong sends use non-blocking `try_send` on the shared capacity-1 input channel; a liveness frame that does not fit is deferred (pings, without counting a miss) or dropped (pongs, both re-triggerable), so outbound backpressure can no longer stall the demux loop or shutdown.
2. One ping nonce per outage, resent until answered, so a pong slower than the ping interval still counts as proof of life. A miss is only counted for a ping that actually left the client.
3. Establishment and inbound pings clear stale miss state.
4. `armed` is split from `established` (the latter added in `add-stream-establishment-ack`): armed streams regain keepalive when the peer shows life after a threshold trip. An unarmed stream is never pinged, so there is nothing to cut off.
5. Confinement is enforced by arming: pinging is gated on the stream having armed, so a peer that does not speak the extension (the IPR, or any tunnelled-protocol endpoint) never arms and is never pinged. The IPR wrapper keeps plain `open_stream` and the IPR is not modified. This is the intended scope, not outstanding work.
6. The ping interval and miss threshold are fixed module constants, so there is no runtime override and no floors to enforce. The wire-layout doc comment lists the new msg types.

## Backwards compatibility

| Dialer | Acceptor | Behaviour |
|---|---|---|
| old | old | Unchanged. |
| new | new | Full behaviour. |
| new | old | `Open`/`OpenAck` behaviour as in `add-stream-establishment-ack`. The old acceptor sends no liveness frame, so the stream never arms and is never pinged. No failures. |
| old | new | Open accepted as today. No pings arrive; acceptor behaviour identical to today. |

No rollout ordering requirement. Acceptors upgrading first is the useful order, but nothing breaks either way.

## Risks / Trade-offs

- **Misclassification (accepted for v1):** a new peer never arms if every arming frame (the `OpenAck`, and any later `Ping` or `Pong`) is lost, in which case its stream is never pinged and relies on the idle reaper as backstop. Fragment-level retransmission makes losing all of these unlikely; re-probing an unarmed stream is deferred.
- **Keepalive changes the idle reaper's effective meaning** for armed streams (see Decision 4); consumers relying on 30-minute reaping of quiet-but-alive streams will observe streams staying open. Judged correct and documented.

## Measurements

`PING_SURBS = 2` is fixed against the single-packet SURB ceiling measured in `add-stream-establishment-ack` (one 4-hop SURB = 460 bytes; empty-payload single-packet ceiling = 3 SURBs). 2 SURBs keeps the ping itself unfragmented while funding more than the one SURB its `Pong` consumes. See that change's design.md for the full derivation; it is not repeated here.
