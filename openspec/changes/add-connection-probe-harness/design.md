# Design: Add Connection-Probe Harness

## Context

The harness lives in the existing Playwright setup at `wasm/smolmix/tests/` (projects `smoke`/`suite` over chromium/firefox/webkit, `webServer` serving the built `internal-dev` bundle). It reuses that plumbing: a browser loads the WASM through `internal-dev`, Playwright drives it, and the Node side of the test writes files.

## Decisions

### Fresh WASM instance per iteration

`setupMixTunnel` is one-shot: the `TUNNEL` `OnceLock` rejects a second call in the same module instance. So each of the 100 runs needs its own instance. A new `Worker` per iteration is the cheapest way: each worker instantiates the WASM afresh, so its `OnceLock` starts empty. The probe spawns a worker, runs one setup plus the DoH probes, tears the worker down, and moves to the next. Reusing one page is fine; only the worker is per-iteration.

A fresh random `clientId` per run is what makes the exit random: with no stored gateway for that id, the base client registers a new (performance-weighted) entry gateway, and with no `preferredIpr` the tunnel auto-discovers a fresh performance-weighted exit. Same-id reuse would pin the stored gateway and defeat the point.

### Classify from logs, not a new API

The tunnel already emits everything the probe needs on the console: `auto-discovered N IPR candidate(s)`, `connecting to IPR <addr>`, `v10 connect timed out; retrying v9`, `IPR <addr> failed ...; rotating`, `IPR connected: <addr> (in <t>)`, and on the DoH side `resolved <host> => <ip> via <endpoint>`, `resolver <endpoint> rate-limited us (HTTP 429)`, `resolver <endpoint> returned HTTP <status>`. Parsing these avoids a WASM API change (a structured setup-result), which is the heavier option. The cost is coupling the probe to log wording; if that proves brittle, the upgrade is a structured result object from `setupMixTunnel`, and the probe switches to reading it. Log parsing first, structured result only if it breaks.

### DoH probe: one resolver at a time

To attribute a timeout or 429 to a specific resolver, each probe sets `dohEndpoints` to a single endpoint and resolves a fixed hostname. Cloudflare, Quad9, and Google each get an isolated verdict. A final probe with the default three-endpoint list records whether resolution rotated to a backup, which the single-endpoint probes cannot show.

### Output

`results.md` is written by the Node side after all runs. Three parts: a connection-runs table (run, exit, version, MTU, attempts, downgrade, rotated, outcome, ms, failure reason), a DoH table (run, resolver, verdict, ms), and a summary (success rate, downgrade rate, mean/max attempts, per-resolver 429 and timeout rates). It is a generated artifact, git-ignored, not committed.

## Interactions and risks

- Runtime: 100 runs of gateway registration plus IPR handshake is tens of minutes. The `connection-probe` project needs a long per-test timeout and stays out of `smoke`/`suite` and CI.
- Storage growth: a fresh `clientId` per run creates a browser storage namespace per run. The probe should clear storage between runs, or accept the growth for a single session and wipe at the end.
- Concurrency: sequential runs are simplest and kindest to the network. A small parallel factor could cut wall-clock time but strains the gateway/registration path and muddies per-run attribution. Default sequential; parallelism is a later tuning knob.
- Build dependency: the probe only means anything against a build that has the retry + DoH changes. Run it on this branch's WASM, not a stale `pkg/`.

### Record the entry gateway per run

Every run records the entry gateway it registered with (it is in the `finished registration with gateway <id>` log line), on success and on failure. This is what lets a failure row be attributed: a run that fails against several exits on one gateway, repeated across runs that share that gateway, points at the gateway rather than the exits. Cheap, and it turns the common-mode failure case into something the results can show rather than hide.

## Open questions

- Whether `internal-dev`'s existing `headless.html` + `worker.js` can be driven to spawn a fresh worker per iteration and surface per-run console capture, or whether a small dedicated probe page is cleaner. To settle when implementing.
