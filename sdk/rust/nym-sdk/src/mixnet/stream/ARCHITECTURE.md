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
- `Ping` (3) is a keepalive probe; its nonce rides in the sequence-number
  field
- `Pong` (4) echoes a ping's nonce back

There is no `Close` type: streams clean up via `Drop` and idle timeout.
Sequence numbers enable reorder buffering (up to `MAX_REORDER_BUFFER_BYTES`,
8 MiB of out-of-order data per stream). Peers running an SDK without these
frame types reject them during attribute parsing and drop the frame, so
mixed-version peers interoperate without a coordinated upgrade.

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
- **Ping** → answered with a `Pong` echoing the nonce, only if the stream
  is still registered; unknown ids get silence
- **Pong** → clears the matching outstanding ping
- Unrecognised messages are silently dropped

The router's periodic tick (every 10 s at most) reaps stale streams and
runs the keepalive sweep described under Keepalive.

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

## Keepalive

Establishment answers "did anyone accept this stream". Keepalive answers
"is the peer still there", which needs periodic probing because a peer
that dies mid-stream sends nothing at all.

### Pinging

The router's tick pings armed outbound streams (see Arming) that have
received nothing for `DEFAULT_PING_INTERVAL` (60 s); a stream with
inbound traffic inside the interval is never pinged, so an active stream
generates no keepalive traffic. Only the
dialer pings, because acceptor-side pings would spend the dialer's SURBs
on every exchange. Each ping carries `PING_SURBS` reply SURBs, more than
the single SURB its pong consumes, so an idle stream with a live peer
does not deplete the acceptor's pool.

A pong is sent only for streams still registered, so it attests "the
stream is still open at the peer", not "the application is reading".
After `DEFAULT_MISSED_PONGS_THRESHOLD` (3) consecutive unanswered pings,
the stream fails in-band with `PeerUnresponsive`, delivered through the
data channel in order, like `DataLoss`. Any later frame from an armed
peer resumes keepalive.

All liveness sends use non-blocking `try_send` on the shared input
channel: a frame that does not fit is deferred (pings, without counting
a miss) or dropped (pongs, acks), so backpressure from application
writes can never stall the router. One nonce is used per outage and
resent until answered, so a pong slower than the ping interval still
matches.

### Arming

There is no protocol version field. A stream arms when its peer sends its
first liveness frame (`OpenAck`, `Ping` or `Pong`), which proves the peer
speaks the extension. Keepalive acts only on armed streams: an unarmed
stream is never pinged and never fails, because a peer that has sent no
liveness frame cannot be told apart from an old SDK or from a server that
tunnels a different protocol over the stream, and probing it would send
frames it cannot answer.

This confinement is a deliberate scope decision, not a missing case.
Keepalive here covers raw `mixnet::stream` used directly between two SDK
peers. A consumer that tunnels another protocol over a stream (smolmix
carrying TCP/IP, or a fire-and-forget UDP flow) brings its own liveness
model, and its stream to an IP packet router never arms, so this module
sends that peer nothing and needs no per-caller opt-out. Liveness for
tunnelled traffic is the consumer's concern, not this layer's.

### Interaction with the idle timeout

Liveness frames refresh `last_activity`, so for peers that speak the
extension the idle reaper (default 30 min) becomes a backstop and
`PeerUnresponsive` (about 3 min at these fixed values) is the effective
failure path. For unarmed peers (old SDKs, or servers like the IPR) the
reaper keeps its original meaning as the only liveness signal.

The ping interval and miss threshold are fixed constants
(`DEFAULT_PING_INTERVAL`, `DEFAULT_MISSED_PONGS_THRESHOLD`), not builder
options. This module keeps to the minimum surface; a consumer that needs
different timing wraps the stream with its own policy.

## Cleanup

- **`Drop` on `MixnetStream`**: deregisters from `StreamMap`
- **`poll_shutdown`**: same, with a `deregistered` flag to avoid double-remove
- **Idle timeout**: streams inactive longer than `stream_idle_timeout`
  (default 30 min) are reaped every 10s

## `StreamMap`

`Arc<Mutex<HashMap<StreamId, StreamEntry>>>`, shared between router,
streams, and listener. Methods: `register_stream`, `remove`,
`send_to_stream`, `cleanup_stale`, `mark_established`, `last_activity`,
plus the keepalive handlers (`on_ping`, `on_pong`, `ping_sweep`).

## Known Limitations

- **No `Close` message**: there is no explicit stream-close signal.
  Streams clean up locally via `Drop` and idle timeout, and a peer's
  pings against a closed stream go unanswered rather than being refused.
  A proper close/EOF mechanism requires further protocol work.
- **The IPR does not speak these frames**: a stream to an IP packet
  router never receives an `OpenAck`, `Ping`, or `Pong`, so it never
  arms and is never pinged, and gets no mid-stream liveness. Teaching
  the IPR's mixnet listener to inspect `SphinxStreamMsgType` is separate
  infrastructure work.
- **The keepalive ping costs reply SURBs**: one 4-hop reply SURB
  serialises to ~460 bytes, and an empty-payload stream message fits 3
  SURBs in one Regular packet (a fourth fragments it), so `PING_SURBS =
  2` stays a single packet. A dialer whose SURB pool is empty skips that
  tick's ping rather than failing. Pinning this against upstream size
  changes belongs to the parked SURB-budgeting work, not here.
- **Reorder buffer cap**: out-of-order messages are buffered up to
  `MAX_REORDER_BUFFER_BYTES` (8 MiB) per stream. A full buffer skips the
  missing range and reports the loss in-band as `InvalidData`. `recv()`
  surfaces it once and later messages keep flowing; `AsyncRead` fails
  the stream permanently. The cap is generous relative to per-tunnel
  throughput, so a late frame with a retransmit in flight does not trip
  it.
