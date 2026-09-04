# Tasks: Add Stream Establishment Acknowledgement

## 1. Wire protocol

- [x] 1.1 Add `OpenAck = 2` to `SphinxStreamMsgType` in `common/nym-lp-data/src/packet/frame.rs` with a parse arm
- [x] 1.2 Wire tests: roundtrip for `OpenAck`; unknown discriminant still rejected (old-peer drop path)

## 2. Stream state

- [x] 2.1 Extend `StreamEntry` with the `established` watch channel; `accept()` sends the ack via the `sender_tag` carried on the inbound `Open`
- [x] 2.2 `register_stream` returns the established watch receiver alongside the data receiver

## 3. Router

- [x] 3.1 `OpenAck` arm: fire the established watch, refresh activity
- [x] 3.2 Inbound `Data` also resolves `wait_established` (peer provably accepted the stream), covering a lost ack

## 4. Establishment

- [x] 4.1 `MixnetListener::accept()` sends best-effort `OpenAck` via `new_reply(sender_tag)` after successful registration, using non-blocking `try_send`; send failure logs and continues
- [x] 4.2 The ack is paid for out of the reply SURBs already attached to the `Open`; no SURB-count changes

## 5. Public API

- [x] 5.1 `MixnetStream::wait_established(timeout)`; documented as inconclusive on timeout, stream usable afterwards
- [x] 5.2 Passive `last_peer_activity()` getter on `MixnetStream`; no traffic generated

## 6. Tests

- [x] 6.1 `OpenAck` roundtrip through `StreamMap`: register, ack, established watch fires
- [x] 6.2 `wait_established` timeout leaves the stream usable
- [x] 6.3 `accept()` survives an `OpenAck` send failure

## 7. Documentation

- [x] 7.1 `stream/ARCHITECTURE.md`: establishment section with the SURB-ack privacy rationale (gateway forwards acks regardless of recipient state so they cannot become an online-status oracle; acknowledgement therefore lives at the stream layer, by consent), cross-referencing `handle_final_hop`
- [x] 7.2 Stream tutorial: `wait_established(timeout)` usage and the `reply_surbs >= 1` requirement
