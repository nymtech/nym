## Why

The Node Families feature is fully built and exercised in Storybook, but its end-to-end journeys have only ever run against Storybook iframes. We want to validate the page and the owner/operator flows inside the **real wallet app shell** (the production router and chrome), backed by the existing Storybook mock providers so no live chain or Rust IPC is required — and we want that primary suite to run **everywhere, including the developer's Mac**, with a heavier native-binary check available in CI.

## What Changes

- Mount the existing Family page via a **dedicated, build-flag-gated mock entry** (`main.mock.tsx` + `main.mock.html`): it renders the real router + layout but with the Storybook mocks (`MockMainContextProvider` + `MockFamiliesContextProvider`), so it runs in a plain browser with no Tauri runtime or login. The real `main` entry and `/family` route are untouched; production builds never include the mock entry. (Revised during apply — the real app bootstrap is Tauri-coupled and can't boot in a browser, so an in-place provider swap was not viable.)
- **Primary e2e — Playwright against the dev server (mock-wired):** point Playwright at the running webpack dev server (`http://localhost:9000`, the same `devUrl` Tauri loads) with the mock flag enabled, and replay the same journeys currently covered by the Storybook flow stories. This drives the real app shell + router in a real browser (Chromium/WebKit), is cross-platform (runs locally on macOS), and reuses the existing `@playwright/test` dependency and `data-testid` selectors. It supersedes the current Playwright→Storybook-iframe suite by pointing at the app shell instead.
- **Optional validation — WebdriverIO + `tauri-driver` in CI:** as a "how far can we get against the actual binary" leg, add the Tauri WebDriver CI flow (Ubuntu + `xvfb-run`, `webkit2gtk-driver` + `tauri-driver`, WebdriverIO) to drive the **packaged app in the native WebKitGTK webview**. Linux/Windows only (macOS has no WKWebView driver), so it lives in CI, not local dev, and starts non-blocking.
- **Optional higher-fidelity tier — sandbox real-IPC smoke:** the node-families contract is now deployed to **sandbox** (currently one family, one member). This unlocks a read-only smoke against the real `FamiliesContextProvider` + `src/requests/families.ts` wiring (parent-change task 9.4), separate from the deterministic mock e2e.
- **Not** adopting Playwright for the Tauri runtime: Playwright can't speak the classic W3C WebDriver protocol `tauri-driver` exposes (its experimental WebDriver **BiDi** support is a different protocol); WebdriverIO remains the client for the native leg.

## Capabilities

### New Capabilities
- `families-app-mock-build`: A build-time flag that mounts the Family page inside the wallet app shell with the Storybook mock providers, seeded by deterministic fixtures, served by the dev server, while keeping mock code out of production builds.
- `families-app-e2e`: End-to-end journey coverage of the owner and operator Node Families flows — primarily Playwright against the mock-wired dev server (cross-platform), with an optional WebdriverIO + `tauri-driver` native-webview validation leg in CI.

### Modified Capabilities
<!-- None: the existing node-families-owner / node-families-operator specs describe behavior this change exercises but does not alter. -->

## Impact

- **Frontend (`nym-wallet/src`)**: provider-selection seam (build-time flag) around `FamiliesContextProvider` vs `MockFamiliesContextProvider`; a Family route reachable in the mock-wired dev server; webpack config gains a mock-flag-driven `DefinePlugin` constant + persona seed.
- **Tests (`nym-wallet/e2e`)**: repoint/extend the Playwright config from Storybook (:6006) to the mock-wired dev server (:9000); journeys reuse existing selectors. Add an optional WebdriverIO config + `tauri-driver` (cargo) for the native leg.
- **CI (`.github/workflows/ci-nym-wallet-frontend.yml`)**: the existing single `build` job (ubuntu-22.04: install → tsc → lint → unit tests → build-storybook → upload) gains the **primary Playwright e2e step** (currently the Playwright suite is NOT in CI), and a **separate optional** native-webview job (xvfb-run + `webkit2gtk-driver` + `tauri-driver` + Tauri/Rust build, starts `continue-on-error`).
- **Figma Code Connect (recently merged)**: `src/pages/families/FamilyPage.figma.tsx` maps `FamilyPage` via `example: () => <FamilyPage />`. The provider seam MUST keep `FamilyPage` itself provider-agnostic and importable in isolation so this mapping (and the `src/**/*.figma.tsx` config) keeps resolving.
- **Theme (recently merged Nym 2.0 swap)**: orthogonal — color-only, no DOM/`data-testid` change, so journeys/selectors are unaffected; the mock-wired build simply renders the new dark palette.
- **Dependencies**: no new dep for the primary suite (`@playwright/test` already present); `webdriverio` (+ runner) and `tauri-driver` (cargo) only for the optional native leg.
- **Out of scope**: full destructive lifecycle mutations against shared sandbox (read smoke only); macOS native-webview e2e (unsupported by Tauri); Figma Code Connect publish (Tier-1/Hux-gated); real IPC wiring beyond the read smoke (parent-change 9.4/9.5).
