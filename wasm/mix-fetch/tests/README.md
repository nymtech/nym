# mix-fetch Playwright Tests

Automated browser tests for the mix-fetch internal-dev harness. Tests run against the webpack-built `internal-dev/dist/` served locally.

## Prerequisites

WASM build artifacts must exist (Go first, then Rust). For local dev, use the debug targets:

```bash
# Builds to go-mix-conn/build/
make -C wasm/mix-fetch/go-mix-conn build-debug-dev
# Builds to pkg/ (needs Go bindings)
make -C wasm/mix-fetch build-rust-debug
```

CI uses the same debug targets.

Build the internal-dev webpack bundle:

```bash
cd wasm/mix-fetch/internal-dev && npm install && npm run build
```

Install Playwright and browsers:

```bash
cd wasm/mix-fetch/tests && npm install && npx playwright install --with-deps
```

## Running

```bash
# Smoke tests (all browsers, WASM load + MixFetch init)
npm run test:smoke

# Stress tests (all browsers, stresstest on mainnet)
npm run test:stress

# Single browser
npx playwright test --project=smoke-chromium
npx playwright test --project=stress-firefox
```

## Test tiers

### Smoke (`smoke.spec.mjs`)

Verifies the internal-dev harness loads in a headless browser: Rust WASM + Go WASM initialise, the worker signals readiness, MixFetch connects to a random Entry Gateway, and no console errors are emitted. Runs on Chromium, Firefox, and WebKit.

- **CI workflow**: `ci-sdk-wasm.yml`
- **Trigger**: every PR that touches `wasm/**`, `clients/client-core/**`, `common/**`, or the workflow itself
- **Timeout**: 1 minute

### Stress (`stress.spec.mjs`)

Connects to mainnet via a random Entry Gateway, fires 10 concurrent mixed-size fetches through the mixnet, and asserts >= 80% succeed. Runs on all three browsers.

- **CI workflow**: `nightly-mix-fetch-stress.yml`
- **Trigger**: daily at 03:00 UTC via cron, also available via `workflow_dispatch` for manual runs
- **Timeout**: 2 minutes per browser, 2 retries

## Arch/Manjaro note

Playwright's WebKit is built for Ubuntu 24.04 and links against specific soname versions that don't match Arch's (e.g. `libicu*.so.74` vs `.so.78`, `libxml2.so.2` vs `.so.16`). `playwright install --with-deps` also fails because it uses `apt-get`.

Chromium and Firefox work without any workarounds. To skip WebKit locally:

```bash
npx playwright test --project=smoke-chromium --project=smoke-firefox
npx playwright test --project=stress-chromium --project=stress-firefox
```

All three browsers work on CI (Ubuntu runners with `--with-deps`).

> **TODO**: investigate getting WebKit running on Arch/Manjaro (soname symlinks or alternative Playwright WebKit build).

## TODO

- [ ] Add Playwright CI for `wasm/client/` (nym-client-wasm) via the chat-app examples after WASM cleanup
- [ ] Add Playwright CI for other SDK examples once stale dependencies are resolved
- [ ] Consider WebKit system deps in CI runner setup (currently relies on `playwright install --with-deps` on Ubuntu)
