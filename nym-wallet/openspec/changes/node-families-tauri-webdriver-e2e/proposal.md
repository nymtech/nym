## Why

The Node Families feature is fully built and exercised in Storybook, but its end-to-end journeys have only ever run in a browser (Storybook `play` functions + Playwright against the Storybook iframe). Nothing yet proves the page renders and the owner/operator flows work **inside the real Tauri desktop shell** — the native webview, the app router, the production chrome. We want that confidence without needing a live chain or Rust IPC handlers, so we run the same journeys against the page mounted in the Tauri app but backed by the existing Storybook mock providers.

## What Changes

- Mount the existing Family page in the Tauri app behind a **build-time mock flag**: when the flag is set, the app wires `MockFamiliesContextProvider` (the in-memory contract engine already used by Storybook) instead of the Tauri-backed `FamiliesContextProvider`; the production bundle tree-shakes the mock code out entirely.
- Produce a dedicated **mock-wired Tauri build** the test harness can launch — the real binary and native webview, but seeded with the deterministic Storybook fixture stores (`buildOwnerFlowStore` / `buildOperatorFlowStore`) so flows are reproducible offline.
- Add a **WebdriverIO + `tauri-driver`** e2e harness that drives the actual native webview and replays the same journeys currently covered by the Storybook flow stories: owner lifecycle (create → invite → accept → kick → disband) and operator lifecycle (accept → leave, then reject), plus the multi-node invite-states assertion. Selectors reuse the existing `data-testid`s.
- Wire the WebdriverIO suite into **CI on Linux** (Tauri WebDriver supports only Linux/Windows; macOS has no WKWebView driver), keeping the existing Playwright-against-Storybook suite as the cross-platform/local check.
- **Not** adopting Playwright for the Tauri runtime: Playwright cannot speak the classic W3C WebDriver protocol that `tauri-driver` exposes; WebdriverIO is the supported client.

## Capabilities

### New Capabilities
- `families-app-mock-build`: A build-time flag that mounts the Family page inside the Tauri wallet app with the Storybook mock providers, seeded by deterministic fixtures, while keeping mock code out of production builds.
- `families-webdriver-e2e`: A WebdriverIO + `tauri-driver` end-to-end suite that launches the mock-wired Tauri binary and verifies the owner and operator Node Families journeys against the native webview in CI.

### Modified Capabilities
<!-- None: the existing node-families-owner / node-families-operator specs describe behavior that this change exercises but does not alter. -->

## Impact

- **Frontend (`nym-wallet/src`)**: provider-selection seam (build-time flag) around `FamiliesContextProvider` vs `MockFamiliesContextProvider`; a Family route/entry reachable in the mock build; webpack config gains a mock-flag-driven define + entry path.
- **Build/config**: webpack (`webpack.dev.js` / a mock variant), Tauri (`tauri.conf.json` `devUrl`/`frontendDist` or a build profile) so the harness can launch the mock-wired app.
- **Tests (`nym-wallet/e2e`)**: new WebdriverIO config + spec(s) mirroring `e2e/families.spec.ts` journeys; `tauri-driver` as a dev/CI dependency (`cargo install tauri-driver`).
- **CI (`.github`)**: a Linux job that installs `WebKitWebDriver` + `tauri-driver`, builds the mock-wired binary, and runs the WebdriverIO suite.
- **Dependencies**: add `webdriverio` (+ runner) to `nym-wallet` dev deps; `tauri-driver` via cargo. No production dependency or runtime change.
- **Out of scope**: real IPC/chain wiring (still task 9.4/9.5 of the parent change); macOS native-webview e2e (unsupported by Tauri).
