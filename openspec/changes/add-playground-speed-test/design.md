# Design: playground speed test

## Context

The playground ([`MixPlayground.tsx`](../../../documentation/docs/components/playground/MixPlayground.tsx)) sets up one tunnel with `setupMixTunnel` and then runs DNS, GET, WebSocket, stress and download sections on it. Its WebSocket echo burst sends N messages at once and reports RTT percentiles. Nothing on the page models the result.

### Constants the model uses

| Constant | Value | Source |
|---|---|---|
| Main stream mean gap | 20 ms (50 pkt/s) | `DEFAULT_MESSAGE_STREAM_AVERAGE_DELAY`, `common/client-core/config-types/src/lib.rs` |
| Loop cover stream mean gap | 200 ms (5 pkt/s) | `DEFAULT_LOOP_COVER_STREAM_AVERAGE_DELAY`, same file |
| Mean delay per mix hop | 15 ms | `DEFAULT_AVERAGE_PACKET_DELAY`, same file |
| Mix hops per direction | 3 | Docs: [latency is the mechanism](https://nym.com/docs/developers/limitations#latency-is-the-mechanism) |
| IPR return flush interval | 20 ms | `BUFFER_TIMEOUT`, `common/ip-packet-requests/src/codec.rs` |
| Data reply SURBs per packet | 2 (playground default) | `SurbsConfig`, `wasm/smolmix/src/ipr.rs` |
| Client MTU | 1500 desktop, 1280 Android and iOS, or the IPR-negotiated MTU if lower; 1420 for IPRs before v10 | `DEFAULT_MTU`, `CLIENT_MTU_FALLBACK`, `client_mtu` in `common/smol-core/src/device.rs` |
| TCP buffer (and so the TCP window) | 64 KiB | `tcp_connect` caps the buffer at `u16::MAX`, `wasm/smolmix/src/stream.rs` |

### Facts from the code that shape the design

1. **The client send loop.** With Poisson on, `poll_poisson` in `real_traffic_stream.rs` waits an exponential gap, then sends one queued real packet, or a cover packet if the queue is empty. With Poisson off, `poll_immediate` sends each real packet when it arrives. It has no rate cap and sends no main-stream cover.
2. **`disableCoverTraffic` in the wasm client only stops the loop cover stream.** `wasm/smolmix/src/tunnel.rs` sets `disable_loop_cover_traffic_stream` and nothing else. With Poisson on, the main stream still fills empty ticks with cover. This is different from `Config::set_no_cover_traffic` in client-core, which also turns off Poisson. The loop stream has its own clock, but it uses `try_send` on the shared gateway channel, so the client drops loop packets under gateway backpressure (`cover_traffic_stream.rs`).
3. **The sender chooses every hop delay.** The client samples the forward delays into each packet header (`common/nymsphinx/src/preparer/mod.rs`) and the return delays into each SURB that it builds (`common/nymsphinx/anonymous-replies/src/reply_surb.rs`). Mix nodes only apply them. Poisson off therefore does not remove the hop delays.
4. **The IPR does not pace its replies, but it bundles them.** nym-node sets `disable_poisson_rate: true` for its IPR by default (`nym-node/src/config/service_providers.rs`, applied in `nym-node/src/config/helpers.rs`). An operator can change this. The IPR still holds return IP packets in a buffer and flushes it every 20 ms (`BUFFER_TIMEOUT`; `connected_client_handler.rs`). A small reply waits about U(0, 20) ms.
5. **Things that use main-stream slots besides the app data:** TCP ACKs (smoltcp sends a pure ACK about 10 ms after it receives data), SURB top-up requests (the reply-SURB lanes in `real_traffic_stream.rs`) and TCP retransmits. Mixnet acks do not: the final-hop node takes the ack out of the payload and sends it on.
6. **The wasm tunnel is one-shot.** After `disconnectMixTunnel` the page must reload before a new `setupMixTunnel` (see the disconnect message in `MixPlayground.tsx`).
7. **Nagle is on.** Neither smolmix nor smol-core calls `set_nagle_enabled`, so smoltcp's default applies. While data is unacknowledged, smoltcp holds every write smaller than one MSS. With a mixnet RTT of about 0.3 to 1 s, small WebSocket messages merge into about one segment per RTT. A ramp of small messages would therefore never reach the send budget, and the number of packets per message is not a constant.
8. **Every packet takes a new random route** on both legs. The time on network links changes from sample to sample, by about as much as the mixing delays.
9. **Backlogs outlive a closed socket.** All smolmix traffic uses one transmission lane (`TransmissionLane::General`, `ipr.rs`), and `on_close_connection` removes only per-connection lanes. Data still in the client-core queue, the smoltcp send buffer or the WebSocket command channel (`mixwebsocket.rs`) is still sent after the socket closes.

## Goals / Non-Goals

**Goals**

- Show the throughput limit as a knee: RTT stays flat below it and climbs above it.
- Show the Poisson spread as an RTT histogram next to the model of the mixing and pacing delays.
- Show what an observer at the entry gateway sees, with and without Poisson pacing and cover.
- Let a visitor compare the same test with Poisson on and off.

**Non-Goals**

- A packet-level counter from the wasm client (real and cover packets sent, queue depth). The live tests infer from timing. See follow-ups.
- A count of Sphinx packets per message. Nagle (point 7) makes it change with the rate, so the ramp reports throughput instead.
- The packet-size staircase test and the concurrency sweep. See follow-ups.
- Changes to per-hop delay. The playground does not expose `average_packet_delay`, and the docs say not to change it.
- Changes to the wasm crate or the SDK packages.

## Decisions

### D1. Placement and isolation

The simulator is its own `<Demo name="TrafficShapeSimulator" />`, registered with `ssr: false` like the threat-model demos. It needs no tunnel.

The live tests go in `components/playground/speed-test-section.tsx`. `MixPlayground` renders it and passes `mods`, `log`, the tunnel options that `setup()` actually used (captured in state at setup, because the checkboxes stay editable after connect), a flag that says whether any other section is busy, and a `setSpeedTestBusy` setter.

The section opens its own `MixWebSocket`, which is a second TCP connection in the same tunnel, with its own message handler. Probe frames are binary and start with an 8-byte header: a 4-byte magic tag and a 4-byte sequence number. The sequence counter is a ref that only goes up, so numbers are unique across runs on one socket. The handler keeps a map of outstanding probes and ignores any frame that is text, has no tag, or has a number that is not outstanding (a greeting from the echo server, or a late echo from a cancelled run).

All sections share one send queue, so other traffic during a run would distort it. While `speedTestBusy` is set, `MixPlayground` disables every control that sends traffic or changes the tunnel: Disconnect, DNS, GET, WebSocket connect, send, close and burst, stress, both downloads and "Run both". A speed test does not start while another section is busy. `speedTestBusy` stays set until every probe has come back or the drain timeout has passed, and the section shows a "Draining" state for that time. A Cancel button stops the sending and then drains. On unmount the section clears its timers and closes its socket.

### D2. One model module, tested

`lib/speed-test/` holds the maths, because the vitest include covers only `lib/**`.

- `model.ts`: the constants, `mulberry32(seed)` (a seeded PRNG), `sampleExp`, `simulateClient`, `sampleRttMs`, `histogram` and `quantile`.
- `ramp.ts`: the probe framing, the per-step slope fit, the knee and the capacity estimate.
- `storage.ts`: the `sessionStorage` wrapper (D6).

`sampleRttMs` is the sum of six exponential hop delays (mean 15 ms), one exponential forward send-slot wait (mean 20 ms, only with Poisson on) and one uniform IPR flush wait, U(0, 20) ms. It has one slot wait only, because the IPR does not pace replies (point 4).

The constants test reads `common/client-core/config-types/src/lib.rs` and `common/ip-packet-requests/src/codec.rs` with `fs` and compares the Rust values with the TypeScript constants, so a change in Rust fails the docs test. `ci-docs-test.yml` gets those two paths in its `paths` filter.

`model.ts` takes the mix delay and the send interval from `lib/privacy-model/latency.ts`, which this change corrects from 50 ms and 50 ms to the client-core values (see Impact in the proposal).

### D3. Simulator

Inputs:

| Control | Values |
|---|---|
| Traffic preset | Idle; Chat (one 1-packet message every 2 s); Burst (40 packets at t = 3 s); Stream (80 pkt/s, over budget) |
| Poisson pacing | On / off |
| Loop cover stream | On / off |
| View | Colour (real, main-stream cover, loop cover) / Observer (one colour) |
| Seed | Starts at 1; a "Reshuffle" button picks a new seed |

The simulation is a `useMemo` over the inputs. It uses only the seeded PRNG, so the same inputs give the same picture.

Outputs, for a 10 s window:

- Five timeline rows of 2 s each, with an app lane above a gateway lane. One row of 10 s would be a solid bar at 55 pkt/s.
- Queue depth against time.
- A histogram of the gaps between packets at the gateway, with the exponential curve of the merged Poisson streams.
- Totals: packets per second at the gateway, real packets sent, mean wait in the queue, packets still queued at 10 s.

The lesson for each setting:

| Setting | What the visitor sees in observer view |
|---|---|
| Poisson on, cover on | Idle, Chat and Burst have the same packet times. Stream shows a growing queue. |
| Poisson on, loop cover off | Still a packet on every main-stream tick, because the main stream fills empty ticks (point 2). |
| Poisson off | Packets appear only when the app sends. Burst and Chat are visible in observer view. |

The simulator does not model gateway backpressure (which slows the main stream and drops loop cover) or retransmissions. The UI says so in one line.

### D4. Throughput ramp

The ramp measures throughput, not messages, because of Nagle (point 7). With messages of 1 KiB or more, the TCP segments are full at the rates that matter, and the offered load is a clean number of bytes per second.

Protocol:

1. Open the speed-test WebSocket to the echo URL (default `wss://echo.websocket.org`, the same as the existing section).
2. Send fixed-size messages (default 1024 B) at a series of offered loads: 8, 16, 32, 48, 64, 96 and 128 KB/s. Each step lasts 5 s. The send time of each message comes from the step start, so timer drift does not build up.
3. After each step, wait until every echo of that step is back, or until a drain timeout (30 s) passes. Count missing echoes as lost. Then start the next step.
4. For each step, fit RTT against send time by least squares. The slope `s` is ms of extra RTT per ms of run time.
5. Stop after the first step past the knee, or when a step loses more than half of its echoes, or when the median RTT of a step passes 5 s. This keeps the load and the backlog small. The worst case is about 2 MB each way.

Analysis, with a fluid-queue model. If the path carries `mu` bytes per second and the offered load is `lambda > mu`, the queue grows at `lambda - mu` and the RTT slope is `s = lambda / mu - 1`. So:

- The knee is the first step with `s > 0.1`.
- The capacity estimate is `mu = lambda / (1 + s)` from the knee step.
- The page compares `mu` with the raw send budget: 50 packets per second of about 2 KB, so about 100 KB/s. The difference is the overhead: Sphinx and SURB headers, IP, TCP, TLS and WebSocket framing, TCP ACKs, SURB top-ups and retransmits (point 5).
- The page also shows the TCP window limit at the measured base RTT: 64 KiB divided by the median RTT of the first step. With Poisson off there is no send cap, so the knee should move to this limit or to another one, for example SURBs at the IPR. This is the main lesson of the comparison (D6): turn off one limit and the next one appears.

The estimator assumes a constant cost per byte inside a step. If no step has `s > 0.1`, the page reports "no knee up to 128 KB/s" and does not show an estimate.

Nagle still holds the last partial segment until an ACK comes, so it adds up to one RTT to the base RTT. That raises the flat level of the RTT. It does not add a slope.

### D5. RTT histogram

Protocol: send N echo messages (default 100, 32 B), one at a time. Send the next message when the previous echo arrives, after a random gap of U(100, 300) ms. The lower bound of 100 ms keeps the probe clear of the TCP ACK that the client sends about 10 ms after each echo. The random part avoids phase-lock with the IPR's 20 ms flush timer. The Poisson tick needs no gap, because it has no memory.

Outputs:

- A histogram of the measured RTTs.
- The model curve from `sampleRttMs`, labelled "mixing and pacing delays only", shifted by a fitted offset. The offset is the measured 5th percentile minus the model 5th percentile. A low quantile is used so that the right tail is an honest comparison. The page shows the offset as "fixed time: network links, the IPR and the echo server".
- The page says that the measured spread should be the same as the model or wider, because each packet takes a new random route (point 8) and the link times change.
- Summary: min, p50, p95, max, and the fitted offset.

With Poisson off, the model has no slot wait. The six hop delays stay, because the sender chooses them (point 3).

### D6. Comparison across a reload

The tunnel is one-shot (point 6), so the page compares runs across a reload.

1. Storage key `nym-playground-speed-test-v1` holds `{ v: 1, runs, pending }`. Each run holds the tunnel options that were in use, the test inputs, the results and `finishedAt`. The page keeps the last 6 runs.
2. A "Run again with Poisson off" button (or "with Poisson on") writes `pending` and reloads the page.
3. `MixPlayground` renders on the server too (see `demo.tsx`), so every storage read happens in an effect. The first render shows no stored runs and default toggles. The effect reads `pending`, fills in the toggles, clears `pending`, and shows a banner that tells the visitor to click Setup. It does not connect by itself.
4. The charts draw the last 3 stored runs of each kind, each with a label from its options. Storage keeps the last 6.

Every storage access is in `try/catch`. If storage fails, the tests still run and the page says that the comparison is not available. A "Clear stored runs" button removes the key.

### D7. Charts and measurement

Inline SVG with `d3-scale`, which is already a dependency. `components/playground/chart.tsx` holds the theme tokens, axes, legend and tooltip. Series colours are the first three slots of the dataviz reference palette, checked in light and dark mode. A legend is always present, and the totals table gives a text view of the numbers.

Measurement on the main thread:

- The message handler takes `performance.now()` first and writes into refs.
- The charts re-render at 4 Hz or less while a run is in progress.
- If the tab becomes hidden during a run (`visibilitychange`), the browser throttles the timers, so the run stops and the page says why.

## Risks / Trade-offs

| Risk | Effect | Mitigation |
|---|---|---|
| Third-party echo server | Rate limits; its latency mixes into the result | The fitted offset absorbs fixed latency. The URL is editable. Own echo server is a follow-up. |
| Shared IPR | Other users' traffic changes the return leg | Name it in the UI. Run the comparison soon after the first run. |
| Gateway backpressure | The client slows down, which moves the knee down | Name it in the UI next to the estimate. |
| Path changes on reload | A new client can pick a new gateway and, with "Use random IPR", a new IPR | Name it in the comparison view. See open questions. |
| Load from many visitors | A ramp sends up to about 2 MB each way | Stop after the knee. No auto-start. |

## Open Questions

- Do we host our own echo server for the playground? It would remove one source of noise and the third-party rate limit.
- Is 5 s per step enough to measure the slope through the noise? Check on the first real runs.
- Can the page reuse `clientId` after a reload, so the comparison uses the same gateway? Check that `ClientStorage` allows it first.

## Follow-ups (out of scope)

- A stats getter from the wasm client: real and cover packets sent, queue depth, current delay multiplier. With it the ramp could show packets directly.
- A `noDelay` option on `mixWebSocket` that turns Nagle off. It would let the ramp use small messages.
- The packet-size staircase test and the concurrency sweep from the first ideas list.
