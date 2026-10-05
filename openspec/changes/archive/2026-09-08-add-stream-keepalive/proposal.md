# Add Stream Keepalive

## Why

`add-stream-establishment-ack` confirms that a peer accepted a stream at dial time, but mid-stream a dead peer is still indistinguishable from a quiet one. The only failure signal a consumer can observe today is `StreamFailure::DataLoss` and silence. This change adds automatic ping/pong keepalive so an idle stream's peer can be proven alive, or the stream can be failed in-band when it stops answering.

This builds on `add-stream-establishment-ack`: keepalive reuses the `OpenAck` frame's reply-path plumbing and the `established` signal, and its version tolerance follows the same reasoning: a frame type an old peer does not recognise is dropped cleanly during parsing, never surfaced as an error.

## What Changes

- Add `Ping = 3` and `Pong = 4` to `SphinxStreamMsgType` in `common/nym-lp-data`, with parse arms and wire tests. The ping nonce is carried in the otherwise-unused sequence-number field; roundtrip tests confirm it survives.
- Automatic keepalive on the router's existing cleanup tick: an armed outbound stream that has received no inbound frame for `DEFAULT_PING_INTERVAL` (a fixed module constant, 60 s) is sent a `Ping` with a fresh nonce. Streams with recent inbound activity are never pinged, and an unarmed stream is never pinged at all. The router answers inbound `Ping` with `Pong` (echoing the nonce), for registered streams only; unknown stream ids get silence.
- `StreamFailure::PeerUnresponsive`, delivered in-band through the existing ordered failure channel, after `DEFAULT_MISSED_PONGS_THRESHOLD` (a fixed module constant, 3) consecutive unanswered pings on an armed stream.
- Arming by observation: keepalive activates per stream only after the peer proves it speaks the extension (any received `OpenAck`, `Pong`, or `Ping`). An unarmed stream is never pinged and never fails, so an old-SDK peer never fails a stream. Armed streams that tripped the miss threshold resume keepalive when the peer shows life again, so a later real death is still detected at ping cadence.
- One nonce per outage, resent unchanged until answered, so a pong slower than one ping interval still matches; a miss is counted only for a ping that actually left the client.
- All liveness sends (pings, pongs, the establishment ack) use non-blocking `try_send` on the shared, capacity-bounded router input channel: a ping that does not fit is deferred to the next sweep without counting as a miss; a pong or ack that does not fit is dropped, both being re-triggerable. The router cannot stall the demux loop on outbound backpressure.
- `PING_SURBS = 2`: more than the single SURB a pong consumes, so an idle armed stream's ping/pong exchange does not deplete the acceptor's SURB pool.
- Keepalive is confined to raw `mixnet::stream` used directly between two SDK peers. Because a stream is pinged only after it arms, and a peer that does not speak the extension never sends an `OpenAck`, `Ping`, or `Pong`, such a stream never arms and is never pinged. This covers the IP-packet-router case with no special handling: a stream to an IPR never arms, so the IPR is never pinged and is left unchanged. A consumer that tunnels another protocol over a stream (for example smolmix carrying TCP/IP, or fire-and-forget UDP) brings its own liveness model.
- The ping interval and the missed-pong threshold are fixed module constants. A consumer that needs different timing wraps the stream with its own liveness policy.

## Capabilities

### New Capabilities

<!-- none: this change modifies the sdk-mixnet-stream capability added by add-stream-establishment-ack -->

### Modified Capabilities

- `sdk-mixnet-stream`: adds mid-stream liveness on top of establishment acknowledgement: automatic idle-stream keepalive with in-band unresponsive-peer failure, version-tolerant arming, and keepalive-specific SURB allocation.

## Impact

- `common/nym-lp-data/src/packet/frame.rs`: two enum variants and their parse arms; existing variants and layout untouched.
- `sdk/rust/nym-sdk/src/mixnet/stream/mod.rs`: `Ping`/`Pong` router arms, `StreamEntry` armed/miss-counter state, ping sweep folded into the existing cleanup tick, router gains a `ClientInput` handle, and the fixed `DEFAULT_PING_INTERVAL`, `DEFAULT_MISSED_PONGS_THRESHOLD`, and `PING_SURBS` constants.
- `sdk/rust/nym-sdk/src/mixnet/stream/mixnet_stream.rs`: `PeerUnresponsive` mapping.
- `sdk/rust/nym-sdk/src/mixnet/stream/ARCHITECTURE.md`: arming model and idle-reaper interaction.
- The IP-packet-router wrapper (`ipr_wrapper/ip_mix_stream.rs`) is unchanged: it uses plain `open_stream`, and because an IPR never sends a liveness frame its stream never arms and is never pinged.
- Wire compatibility: old and new SDKs interoperate freely in both directions with no coordinated upgrade; keepalive degrades to no pings and no failures against an old peer.
