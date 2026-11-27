# Add a speed test to the mixnet playground

## Why

The mixnet playground lets a visitor send traffic through mixFetch and MixWebSocket, but it only reports totals (msg/s, KB/s, RTT percentiles). A visitor sees that the mixnet is slow. The page does not show why.

Three properties of the mixnet set its speed, and the docs explain them only in prose ([what Nym cannot do](https://nym.com/docs/developers/limitations), [the always-on stream](https://nym.com/docs/developers/what-is-a-nym-client#the-always-on-stream)):

- **Rate limit.** The client sends on a fixed budget: on average one main-stream packet every 20 ms (50 pkt/s). Traffic above that budget waits in a queue.
- **Poisson process.** The gaps between packets are exponential. The sender also picks an exponential delay (mean 15 ms) for each mix hop, and each node holds the packet for that time. Latency has a wide, deliberate spread.
- **Narrow tunnel.** Each Sphinx packet carries a fixed payload of about 2 KB, headers and reply SURBs take part of it, and all connections share the one send queue.

This change adds three tools that let a visitor see each property. A model predicts the result, a live run measures it, and the page shows both.

## What Changes

- **Traffic-shape simulator.** A new client-only `<Demo name="TrafficShapeSimulator" />`. It needs no tunnel. It draws a timeline of what the application sends and what the entry gateway sees, for a set of traffic presets, with Poisson pacing and the loop cover stream on or off. An "observer view" draws real and cover packets in one colour. A queue-depth chart shows the backlog when the offered traffic is over budget.
- **Throughput ramp (live).** A new section in the playground sends MixWebSocket echo messages at a set of increasing offered loads in KB/s. It fits RTT against send time for each step. Below the limit the RTT stays flat. Above it the RTT climbs. The page reports the knee and a capacity estimate from the slope, next to the raw send budget and the TCP window limit.
- **RTT histogram (live).** The same section sends echo messages one at a time, plots a histogram of the RTTs, and overlays the model of the mixing and pacing delays.
- **Before and after comparison.** The wasm tunnel is one-shot, so a run with Poisson pacing off needs a page reload. The page stores the runs in `sessionStorage`, reloads with the new options filled in, and shows both runs on one chart.
- **Shared model module.** The model (seeded PRNG, Poisson sampling, client send loop, RTT distribution, slope fit) lives in `documentation/docs/lib/speed-test/` with vitest tests. A test reads the constants from the Rust sources.
- **Correct the latency model constants.** `lib/privacy-model/latency.ts` used 50 ms for the mixing delay and 50 ms for the send interval. client-core uses 15 ms and 20 ms. The threat-model demos that use this file (`MixnetDeepDive`, `PacketAnatomy`, `NetworkDiagram`, `LatencyControls`) show the corrected values.

## Capabilities

### New Capabilities

- `playground-speed-test`: the traffic-shape simulator and the live speed tests on the mixnet playground page.

### Modified Capabilities

<!-- none -->

## Impact

- `documentation/docs/lib/speed-test/` (new): `model.ts`, `ramp.ts`, `storage.ts`, `model.test.ts`.
- `documentation/docs/components/playground/` (new): `TrafficShapeSimulator.tsx`, `speed-test-section.tsx`, `chart.tsx`.
- `documentation/docs/components/playground/MixPlayground.tsx`: render the new section; capture the options used at setup; own the `speedTestBusy` flag and disable other traffic while it is set; apply pending options from `sessionStorage` in an effect.
- `documentation/docs/components/demos/demo.tsx`: register `TrafficShapeSimulator`.
- `documentation/docs/lib/privacy-model/latency.ts`: the corrected constants, which `lib/speed-test/model.test.ts` checks against the Rust sources. `components/threat-model/LatencyControls.tsx`: slider step 5 so that 15 ms is on the grid.
- `documentation/docs/.nym-codex/snippets/developers/playground/speed-test.mdx` (new) and `documentation/docs/content/developers/playground.mdx`.
- `.github/workflows/ci-docs-test.yml`: run the docs tests when the Rust files with the model constants change.
- No change to the SDK packages or to the wasm crate. No new npm dependency.
