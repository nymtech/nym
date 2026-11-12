# Add Stream Gap Timeout

## Why

`MixnetStream` delivers frames strictly in sequence order. A missing seq is skipped only when the reorder buffer goes over `MAX_REORDER_BUFFER_BYTES` (8 MiB). A low-rate stream never gets near that, so one lost frame holds every later frame in `pending` for the life of the stream.

For the IP packet router tunnel (`IpMixStream`, and so every `nym-smolmix` `Tunnel`) that turns ordinary mixnet loss into a dead tunnel: outbound keeps working, inbound stops, and nothing returns an error (#7400). Each IPR reply frame is a self-contained bundle of IP packets and TCP inside the tunnel retransmits, so in-order delivery is not needed there and waiting for a lost frame forever is wrong.

## What Changes

- Each stream gets an optional gap timeout, off by default. While it is set, once later frames have waited behind a missing `next_seq` for longer than the timeout, the router skips to the lowest buffered seq and delivers `StreamFailure::DataLoss` in-band, the same signal the overflow skip uses. The check runs as frames arrive. Each gap gets its own timer.
- `IpMixStream` sets a 1.5 second gap timeout on its stream. In the issue's mainnet A/B run with that value, 10 of 10 tunnels stayed up for the full 120 s against 2 of 10 without it.
- Streams that don't set it (byte streams, `AsyncRead` users) keep strict ordering and the 8 MiB overflow rule unchanged.
- A frame that arrives after its seq was skipped is dropped, as any frame below `next_seq` already is.

## Capabilities

### Modified Capabilities

- `sdk-mixnet-stream`: adds an opt-in, time-based gap skip for message streams that tolerate loss.

## Impact

- `sdk/rust/nym-sdk/src/mixnet/stream/mod.rs`: `StreamEntry` gains `gap_timeout` and `gap_since`; `send_to_stream` runs the gap check before the overflow check; `StreamMap::set_gap_timeout`.
- `sdk/rust/nym-sdk/src/mixnet/stream/mixnet_stream.rs`: crate-internal `MixnetStream::set_gap_timeout`.
- `sdk/rust/nym-sdk/src/ipr_wrapper/ip_mix_stream.rs`: sets the timeout right after `open_stream`. `handle_incoming` already turns `DataLoss` into an empty batch.
- No wire-protocol change and no public API change.
