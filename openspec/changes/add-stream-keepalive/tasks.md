# Tasks: Add Stream Keepalive

## 1. Wire protocol

- [x] 1.1 Add `Ping = 3`, `Pong = 4` to `SphinxStreamMsgType` in `common/nym-lp-data/src/packet/frame.rs` with parse arms
- [x] 1.2 Wire tests: roundtrip for each new type; nonce in `sequence_num` survives roundtrip

## 2. Stream state

- [x] 2.1 Extend `StreamEntry`: outstanding ping nonce, missed-pong counter, armed flag
- [x] 2.2 Add `StreamFailure::PeerUnresponsive` with `as_io_error` mapping (inconclusiveness caveat in the doc comment)

## 3. Router

- [x] 3.1 Give the router a `ClientInput` handle
- [x] 3.2 `Ping` arm: reply `Pong` echoing the nonce, registered streams only, silence for unknown ids, no orphan buffering for liveness frames; refresh activity and arm
- [x] 3.3 `Pong` arm: match nonce, clear outstanding ping, reset missed counter, refresh activity, arm; ignore stale or mismatched nonces
- [x] 3.4 Ping sweep in the existing cleanup tick, gated on the stream being armed: ping armed outbound streams idle past `DEFAULT_PING_INTERVAL` with a fresh random nonce and `PING_SURBS` attached, count misses, and deliver `PeerUnresponsive` in-band at `DEFAULT_MISSED_PONGS_THRESHOLD`; an unarmed stream is never pinged
- [x] 3.5 Liveness frames refresh `last_activity` (reaper-shift behaviour)
- [x] 3.6 `PING_SURBS = 2` fixed against the single-packet SURB ceiling measured in `add-stream-establishment-ack` (one 4-hop SURB = 460 bytes; empty-payload single-packet ceiling = 3 SURBs): 2 keeps the ping unfragmented while funding more than the one SURB its `Pong` consumes

## 4. Public API

- [x] 4.1 No new public API: streams open through the existing `open_stream`; keepalive engages automatically once a stream arms
- [x] 4.2 Ping interval and missed-pong threshold are fixed module constants (`DEFAULT_PING_INTERVAL`, `DEFAULT_MISSED_PONGS_THRESHOLD`), not client settings

## 5. Tests

- [x] 5.1 Pong only for registered streams; silence after removal
- [x] 5.2 Nonce mismatch ignored; correct nonce resets the missed counter
- [x] 5.3 Armed stream fails in-band with `PeerUnresponsive` after the threshold; ordering preserved relative to buffered data
- [x] 5.4 Unarmed stream is never pinged
- [x] 5.5 Idle armed stream is pinged; stream with recent inbound traffic is not (paused-time tests, as existing)
- `PING_SURBS` single-packet pinning test: out of scope, belongs to the parked SURB-budgeting work (the `serialised_len` helper it needs lives with that reverted crate change)

## 6. Documentation

- [ ] 6.1 `stream/ARCHITECTURE.md`: arming model section and the idle-reaper shift for armed streams

## 7. Out of scope

- The IPR is not modified. A stream to an IPR never arms (the IPR sends no `OpenAck`, `Ping`, or `Pong`), so it is never pinged. Confinement to peers that speak the extension is enforced by arming, so no IPR-side change is needed.
