# Tasks: Add a speed test to the mixnet playground

## 1. Model module

- [x] 1.1 Add `documentation/docs/lib/speed-test/model.ts`: constants, `mulberry32`, `sampleExp`, `simulateClient`, `sampleRttMs` (hop delays, slot wait, IPR flush), `histogram`, `quantile`
- [x] 1.2 Add `ramp.ts`: probe framing (magic and sequence number), per-step least-squares slope, knee, capacity estimate, TCP window limit
- [x] 1.3 Add `storage.ts`: `{ v: 1, runs, pending }`, last 6 runs, every access in `try/catch`
- [x] 1.4 Add `model.test.ts`: constants read from the Rust sources; same seed gives the same output; `sampleExp` mean; idle and burst have the same packet times with Poisson on; no main-stream cover with Poisson off; Stream builds a queue; RTT model means; framing; slope and capacity estimate
- [x] 1.5 Add the two Rust paths to `ci-docs-test.yml`
- [ ] 1.6 `pnpm test` passes (user runs)

## 2. Latency model constants

- [x] 2.1 `lib/privacy-model/latency.ts`: 15 ms mixing delay, 20 ms send interval, with the client-core sources named
- [x] 2.2 `LatencyControls.tsx`: slider step 5 (`model.test.ts` checks the two values against the Rust sources)
- [ ] 2.3 Check the threat-model demos that use these values (user runs the docs visual check)

## 3. Traffic-shape simulator

- [x] 3.1 Add `components/playground/TrafficShapeSimulator.tsx`: controls from design D3, five timeline rows, queue-depth chart, gap histogram with exponential curve, totals
- [x] 3.2 Register it in `components/demos/demo.tsx` with `ssr: false`
- [ ] 3.3 Check light and dark themes and phone width (user runs the docs visual check)

## 4. Live speed-test section

- [x] 4.1 Add `components/playground/speed-test-section.tsx`: own MixWebSocket, outstanding-probe map, monotonic sequence counter, timestamps first in the handler, results in refs, render at 4 Hz or less
- [x] 4.2 Ramp from design D4: drift-corrected schedule, drain between steps, slope fit, knee, capacity estimate, stop rules
- [x] 4.3 Histogram from design D5: one in flight, U(100, 300) ms gap, model overlay with p5 offset, summary
- [x] 4.4 Cancel, Draining state, stop on hidden tab, cleanup on unmount
- [x] 4.5 `MixPlayground.tsx`: capture the setup options, own `speedTestBusy`, disable all other traffic controls and Disconnect while it is set, pass an "other busy" flag

## 5. Comparison across reload

- [x] 5.1 "Run again with Poisson off / on" writes `pending` and reloads
- [x] 5.2 `MixPlayground` reads `pending` in an effect, fills in the toggles, clears `pending`, shows the banner, and does not connect by itself
- [x] 5.3 Charts draw the last 3 stored runs of each kind with their labels; "Clear stored runs"

## 6. Docs page

- [x] 6.1 Add `.nym-codex/snippets/developers/playground/speed-test.mdx`: `## Speed test [#speed-test]`, reads on its own, `snippet:` links, the simulator demo
- [x] 6.2 Import the snippet in `content/developers/playground.mdx` and update `lastUpdated`
- [x] 6.3 `node lib/snippets/records.mts` and `node lib/snippets/compile.mts` pass
- [ ] 6.4 Run the docs Playwright visual check (user runs) and read its report

## 7. Verification (user runs)

- [ ] 7.1 Ramp with Poisson on: record the knee, the capacity estimate and the base RTT in the scratch file
- [ ] 7.2 Reload with Poisson off, run the ramp again; the knee moves up or goes away
- [ ] 7.3 Histogram run: the measured spread is equal to the model or wider, and the left edges line up after the p5 fit
- [ ] 7.4 The existing echo burst still reports no mismatches after a speed test

## 8. Out of scope (captured in design.md)

- wasm stats getter for packet counts and queue depth
- `noDelay` option on `mixWebSocket`
- Packet-size staircase and concurrency sweep
- Own echo server
