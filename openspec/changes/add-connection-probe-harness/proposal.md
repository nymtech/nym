# Add Connection-Probe Harness

## Why

The IPR connect-retry (`add-ipr-connect-retry`) and DoH switch (`switch-mixdns-to-doh`) changed how the WASM tunnel establishes and resolves names, and both behaviours only show up against the live mixnet. We have validated them by watching the console on a handful of manual runs. That does not tell us how often, across many random exits, a connect needs to rotate, how often a node advertises v10 but answers v9, or how often a public DoH resolver rate-limits us.

This change adds a headless probe that runs establishment many times against randomly selected exits, classifies each run from the logs the tunnel already emits, probes each default DoH resolver, and writes the aggregate to a markdown `results.md`. It turns the retry and DoH behaviour into measured numbers instead of anecdotes.

## What Changes

- A new Playwright test at `wasm/smolmix/tests/`, run as its own project (not part of `smoke`/`suite`), that runs `setupMixTunnel` at least a configurable number of times (default 100) on the auto-discovery path (no pinned IPR), each with a fresh random `clientId` so it registers a fresh gateway and auto-discovers a fresh, performance-weighted random exit.
- Each run uses a fresh WASM instance (a new worker per iteration), because the `TUNNEL` `OnceLock` allows only one `setupMixTunnel` per module instance. The probe captures that instance's console output and classifies the run from the existing `[smolmix]`/`[ipr]` log lines:
  - **attempts**: how many exits were tried (count of `connecting to IPR ...`).
  - **downgrade**: whether any attempt logged `v10 connect timed out; retrying v9`.
  - **rotated**: whether it switched to a different exit after a failure (attempts > 1, from `IPR ... failed ...; rotating`).
  - **outcome**: connected (with the winning exit address, negotiated version, MTU, and elapsed time) or failed.
  - **failure reason** when it fails: no eligible exits, gateway registration failed, all attempts exhausted, or other, each distinguishable from the logs.
  - **entry gateway**: the gateway each run registered with, recorded on success and failure, so failures concentrated on one gateway are visible rather than hidden.
- On a successful connection, the run then probes each default DoH resolver individually (`1.1.1.1`, `8.8.8.8`, `9.9.9.9`) by setting `dohEndpoints` to that one endpoint and resolving a fixed hostname, classifying each as: resolved, timed out, rate-limited (HTTP 429), server error, or (for the multi-endpoint default) rotated to a backup.
- The Node side of the test aggregates every run and writes `results.md`: one table of connection runs, one of DoH probes, and a summary block (success rate, downgrade rate, mean attempts, per-resolver 429 rate).

## Non-goals

- Running in CI. This is a slow, real-network measurement (each run is a full gateway registration plus IPR handshake, so 100 runs is tens of minutes). It stays a manual, opt-in project.
- Asserting pass/fail thresholds. The probe measures and records; it does not fail the build on a given rotation or 429 rate. Turning specific rates into assertions is a later, separate decision.
- Changing any runtime code. The probe reads the logs the tunnel already emits; it adds no WASM API.

## Capabilities

### New Capabilities

- `smolmix-connection-probe`: a headless harness that measures IPR establishment (attempts, downgrades, rotations, failures) and DoH resolver health (resolve, timeout, 429, server error) across many random-exit runs, and writes the results to a markdown file.

### Modified Capabilities

<!-- none -->

## Impact

- `wasm/smolmix/tests/tests/connection-probe.spec.mjs`: the new test.
- `wasm/smolmix/tests/playwright.config.mjs`: a new `connection-probe` project with a long timeout, excluded from the smoke/suite runs.
- `wasm/smolmix/internal-dev/`: may need a minimal probe page or a per-iteration worker entry, if the existing `headless.html`/`worker.js` cannot be driven to spawn a fresh instance per iteration and expose per-run console capture. To be confirmed in `design.md`.
- Output `results.md` is a generated artifact (git-ignored), not committed.
- Reads the console log lines added by `add-ipr-connect-retry` and `switch-mixdns-to-doh`; depends on both being present in the build under test.
