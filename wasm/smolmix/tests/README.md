# smolmix-wasm Playwright Tests

Automated browser tests for the smolmix-wasm mixnet tunnel. Runs smoke tests and a full test suite (HTTPS cold/warm, stress httpbin) across Chromium, Firefox, and WebKit.

## Prerequisites

1. Build the WASM package and internal-dev harness:

```bash
# from repo root
make build-debug
cd wasm/smolmix/internal-dev && npm run build
```

2. Install test dependencies and browser engines (first time only):

```bash
cd wasm/smolmix/tests
npm install
npx playwright install
```

## Running Tests

Both suites use a hardcoded default IPR (see `internal-dev/index.html` and
`internal-dev/headless.js`); no env var is required to run them. Override
the default by exporting `IPR_ADDRESS` if you want to point at a different
exit node:

```bash
export IPR_ADDRESS="6B6iuWX4bQP4GVA4Yq7XmZencaaGw6BaPY6xJWYSwsbF.6g6LRx1fgU2Q2A4ZPKonYHtfBARh1GPMe1LtXk6vpRR8@q2A2cbooyC16YJzvdYaSMH9X3cSiieZNtfBr8cE8Fi1"
```

Pick any combination of projects to run:

```bash
npx playwright test --project=smoke-chromium
npx playwright test --project=suite-firefox
npx playwright test --project=smoke-webkit --project=suite-webkit
npx playwright test                          # all 6 projects
```

Available projects: `smoke-chromium`, `smoke-firefox`, `smoke-webkit`, `suite-chromium`, `suite-firefox`, `suite-webkit`.

## Test Structure

### Smoke

Loads the internal-dev page, fills in the IPR address, clicks setup, and verifies the tunnel connects without errors. Quick connectivity check (~30s).

### Suite

Loads `headless.html` which auto-runs three tests in sequence:

| Test | What it measures |
|------|-----------------|
| Smoke (cold HTTPS) | Full pipeline: DNS + TCP + TLS + HTTP |
| HTTPS GET (warm) | Pooled connection reuse (HTTP only) |
| Stress httpbin | Mixed-size concurrent requests (serialised per-origin) |

Runs twice — once per traffic configuration:

1. **No cover traffic, no Poisson** — baseline performance
2. **With cover traffic + Poisson distribution** — realistic mixnet conditions

Pass criteria:
- Smoke and HTTPS warm must pass
- Stress httpbin >= 80% success rate

## Manual Headless Testing

Run the headless test runner directly in a browser without Playwright:

```bash
cd wasm/smolmix/internal-dev && npm start
```

Then open:
- `http://localhost:9000/headless.html` — no cover, no Poisson (default)
- `http://localhost:9000/headless.html?cover=true&poisson=true` — with cover + Poisson

URL parameters:

| Param | Default | Description |
|-------|---------|-------------|
| `ipr` | hardcoded default | IPR exit node address |
| `cover` | `false` | Enable cover traffic |
| `poisson` | `false` | Enable Poisson dummy traffic |
| `count` | `10` | Stress test request count |

## Timeouts

- Smoke: 3 minutes (tunnel setup ~10s, connectivity check ~20s)
- Suite: 10 minutes per config (mixnet round-trips are ~1-2s each)

## Known Issues

### Playwright Firefox hangs at IPR connect on Arch/Manjaro

Playwright ships a forked Firefox build (Mozilla's "Juggler" patches) to enable
remote control. On Arch-family hosts (Manjaro confirmed) this bundled Firefox
hangs indefinitely at the IPR connect handshake step, in both headed and
headless modes. The bug is unique to the playwright Firefox build; the same
URL loads fine in the system Firefox installation.

The smoke and suite tests reach `[ipr] sending connect handshake` and then
stall until the playwright timeout fires. Topology fetches against
`validator.nymtech.net` succeed; the gateway WSS connection or its message
flow is where it dies. Adding `firefoxUserPrefs` for timer throttling, DoH,
captive portal probes, and IndexedDB persistence does not help.

You cannot point `executablePath` at the system Firefox; playwright's Firefox
binary must speak the Juggler protocol, which mainline Firefox does not.

**Workaround:** run chromium locally; skip firefox, or run it from a CI image
whose playwright Firefox binary is built for that platform.

```bash
npx playwright test --project=smoke-chromium
```

### Playwright Webkit missing libraries on Arch/Manjaro

Playwright bundles `libwebkit2gtk` and a chain of GTK/glib/icu/freetype deps
expecting Ubuntu library layouts. On Arch-family hosts those library versions
or paths differ and webkit fails to launch. Same class of bug as the Firefox
hang, different symptom.

**Workaround:** run webkit tests from a CI image (or container) with the
Ubuntu-shaped library layout playwright expects.
