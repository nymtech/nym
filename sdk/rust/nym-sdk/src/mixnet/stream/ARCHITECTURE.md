# Stream Multiplexing: Architecture

## Overview

The stream subsystem multiplexes concurrent `AsyncRead + AsyncWrite` byte
channels over a single `MixnetClient`. Each channel is a `MixnetStream`
identified by a random `StreamId`.

```text
┌─────────────────────────────────────────────────────────┐
│                      MixnetClient                       │
│                                                         │
│  ┌──────────────┐   ┌──────────────┐                    │
│  │ MixnetStream │   │ MixnetStream │  ...               │
│  │  (peer A)    │   │  (peer B)    │                    │
│  └──────┬───────┘   └──────┬───────┘                    │
│         │writes            │writes                      │
│         ▼                  ▼                            │
│  ┌─────────────────────────────────┐                    │
│  │     ClientInput.input_sender    │                    │
│  └──────────────┬──────────────────┘                    │
│                 │                                       │
│                 ▼                                       │
│           ── mixnet ──                                  │
│                 │                                       │
│                 ▼                                       │
│  ┌─────────────────────────────────┐                    │
│  │     reconstructed_receiver      │                    │
│  └──────────────┬──────────────────┘                    │
│                 │                                       │
│                 ▼                                       │
│  ┌─────────────────────────────────┐                    │
│  │           Router task           │                    │
│  │  decode header → dispatch by ID │                    │
│  └──┬──────────────────────────┬───┘                    │
│     │ Open messages            │ Data messages          │
│     ▼                          ▼                        │
│  ┌──────────────┐   ┌──────────────────┐                │
│  │MixnetListener│   │ StreamMap lookup │                │
│  │  .accept()   │   │ → per-stream tx  │                │
│  └──────────────┘   └──────────────────┘                │
└─────────────────────────────────────────────────────────┘
```

## Wire Protocol (`protocol.rs`)

Every stream message uses the system-wide LP frame format (`nym-lp`).
Each message is an `LpFrame` with a 16-byte header:

```text
[LpFrameKind: 2 LE][frame_attributes: 14][payload: N bytes]
```

For streams, `LpFrameKind` is `SphinxStream` (0x0003) and the 14-byte
`frame_attributes` are parsed as `SphinxStreamFrameAttributes`:

```text
[StreamId: 8 BE][MsgType: 1][SequenceNum: 4 BE][reserved: 1]
```

- `Open` (0) initiates a new stream
- `Data` (1) carries payload for an existing stream
- `OpenAck` (2) acknowledges an accepted `Open` (see Establishment)

There is no `Close` type: streams clean up via `Drop` and idle timeout.
Sequence numbers enable reorder buffering (up to `MAX_REORDER_BUFFER`
out-of-order messages per stream). Peers running an SDK without the
`OpenAck` frame type reject it during attribute parsing and drop the
frame, so mixed-version peers interoperate without a coordinated upgrade.

## Initialization

Stream mode activates lazily on the first `open_stream()` or `listener()`
call. This is a one-way transition: message-mode APIs
(`send_plain_message`, `wait_for_messages`, etc.) return
`Error::StreamModeActive` afterwards.

On activation, `reconstructed_receiver` is handed to the router task
exclusively.

## Router Task (`run_router`)

A background task that reads inbound messages and dispatches them:

- **Open** → forwarded to `MixnetListener`'s accept channel
- **Data** → looked up in `StreamMap` by `StreamId`, forwarded to the
  stream's channel
- **OpenAck** → marks the stream established (fires `wait_established`)
- Unrecognised messages are silently dropped

The router's periodic tick (every 10 s at most) reaps stale streams.

Shuts down via `CancellationToken` or when the receiver closes.

## Stream Lifecycle

**Outbound** (`open_stream`): generates a random `StreamId`, registers in
`StreamMap`, sends an `Open` message, returns a `MixnetStream`.

**Inbound** (`MixnetListener::accept`): receives an `InboundOpen` from the
router, registers in `StreamMap`, returns a `MixnetStream` using the
sender's reply SURBs.

## Establishment

Without an acknowledgement, opening a stream is fire-and-forget. The
dial-time topology check proves the recipient's gateway is routable, and
nothing after it tells the dialer whether the peer's client is running or
whether anyone called `accept()`. `OpenAck` closes that gap.

### Why this lives at the stream layer

Every Sphinx fragment already carries a prepaid SURB-ack, but the
recipient's gateway forwards that ack whether the message was delivered
to a connected client, stored for an offline one, or dropped because the
recipient never registered (`handle_final_hop` in
`nym-node/src/node/mixnet/handler.rs`). This is deliberate: an ack
conditional on recipient state would let anyone who knows an address
probe whether that client is online, without its participation. The ack
layer therefore attests only "reached the destination gateway" and is
consumed internally by retransmission. End-to-end liveness has to come
from a layer where the recipient actively consents by responding, which
is this one.

### How it works

`accept()` sends a best-effort `OpenAck` through the dialer's reply SURBs
after registering the stream. The send is non-blocking and a failure
never fails the accept, because the acknowledgement is advisory and the
stream works without it. `MixnetStream::wait_established(timeout)`
resolves when the ack arrives. The caller passes the timeout, because a
sensible budget depends on the application: a mixnet round trip is
seconds, and a cold establishment takes appreciably longer.

Inbound `Data` also resolves `wait_established`, because data on a stream
proves the peer accepted it. That covers a lost ack: the single `OpenAck`
is never retransmitted.

A timeout is inconclusive. It cannot distinguish a peer running an older
SDK, one with no reply SURBs left, and one that is gone, so it must not
be read as proof of death, and the stream stays usable afterwards.

`last_peer_activity()` reports when the peer was last heard from. It
reads local state and sends nothing.

## Cleanup

- **`Drop` on `MixnetStream`**: deregisters from `StreamMap`
- **`poll_shutdown`**: same, with a `deregistered` flag to avoid double-remove
- **Idle timeout**: streams inactive longer than `stream_idle_timeout`
  (default 30 min) are reaped every 10s

## `StreamMap`

`Arc<Mutex<HashMap<StreamId, StreamEntry>>>`, shared between router,
streams, and listener. Methods: `register_stream`, `remove`,
`send_to_stream`, `cleanup_stale`, `mark_established`, `last_activity`.

## Known Limitations

- **No `Close` message**: there is no explicit stream-close signal.
  Streams clean up locally via `Drop` and idle timeout. A proper
  close/EOF mechanism requires further protocol work.
- **No mid-stream liveness**: `OpenAck` answers "did anyone accept this
  stream", not "is the peer still there". A peer that dies mid-stream is
  only caught by the idle reaper. Keepalive is the follow-up change
  `add-stream-keepalive`.
- **The ack costs one reply SURB**, taken from the count the dialer
  already attaches to the `Open` (10 by default). A dialer that attaches
  none gets no acknowledgement. Sizing SURB counts to the Sphinx packet
  boundary is separate work and deliberately not part of this change.
- **Reorder buffer cap**: out-of-order messages are buffered up to
  `MAX_REORDER_BUFFER_BYTES` (8 MiB) per stream. A full buffer skips the
  missing range and reports the loss in-band as `InvalidData`. `recv()`
  surfaces it once and later messages keep flowing; `AsyncRead` fails
  the stream permanently. The cap is generous relative to per-tunnel
  throughput, so a late frame with a retransmit in flight does not trip
  it.
