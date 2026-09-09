# Tasks: Add Connection-Probe Harness

## 1. Fresh-instance driver

- [ ] 1.1 Establish a way to run `setupMixTunnel` in a fresh WASM instance per iteration (new worker per run), so the `TUNNEL` `OnceLock` does not block the second run
- [ ] 1.2 Use a fresh random `clientId` per run so each registers a new gateway and auto-discovers a fresh random exit; no `preferredIpr`
- [ ] 1.3 Clean up per-run browser storage (or use ephemeral storage) so 100 runs do not accumulate IndexedDB namespaces

## 2. Connection classification (from logs)

- [ ] 2.1 Capture the instance's console output for the run
- [ ] 2.2 Parse `connecting to IPR <addr>` occurrences into an attempt count
- [ ] 2.3 Detect `v10 connect timed out; retrying v9` as a downgrade
- [ ] 2.4 Detect `IPR <addr> failed ...; rotating` as a rotation (attempts > 1)
- [ ] 2.5 Detect `IPR connected: <addr> (in <t>)` as success; record the exit address, negotiated version + MTU, and elapsed time
- [ ] 2.6 On no success, classify the failure: no eligible exits, gateway registration failed, all attempts exhausted, or other
- [ ] 2.7 Record the entry gateway per run (from `finished registration with gateway <id>`), on success and failure, so failures concentrated on one gateway are visible

## 3. DoH resolver probe

- [ ] 3.1 On a successful connection, for each of `1.1.1.1`, `8.8.8.8`, `9.9.9.9`: set `dohEndpoints` to that one endpoint and resolve a fixed hostname
- [ ] 3.2 Classify each resolver: resolved, timed out, rate-limited (429), server error
- [ ] 3.3 Add a multi-endpoint run (default list) that records whether resolution rotated to a backup

## 4. Aggregate + output

- [ ] 4.1 Collect every run's connection + DoH results on the Node side
- [ ] 4.2 Write `results.md`: a connection-runs table, a DoH-probes table, and a summary (success rate, downgrade rate, mean attempts, per-resolver 429 rate)
- [ ] 4.3 Git-ignore `results.md` (generated artifact)

## 5. Wiring

- [ ] 5.1 Add a `connection-probe` Playwright project with a long timeout, excluded from `smoke`/`suite`
- [ ] 5.2 Make the iteration count configurable (env or config), default 100
- [ ] 5.3 README note: what it measures, how to run it, that it is slow and real-network

## 6. Out of scope

- CI integration and pass/fail thresholds.
- Any change to WASM runtime code.
