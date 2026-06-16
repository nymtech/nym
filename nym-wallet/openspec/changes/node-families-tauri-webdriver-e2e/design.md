## Context

The Node Families feature (parent change `node-families-wallet`) is complete and verified in Storybook. The page component is already cleanly decoupled from Tauri:

- `src/pages/families/FamilyPage.tsx` — pure page, consumes `useFamiliesContext()`, no Tauri imports.
- `src/context/FamiliesContextProvider.tsx` — the **real** provider; the only families file importing `./main` (Tauri runtime).
- `src/context/mocks/families.tsx` (`MockFamiliesContextProvider`) — drives the page from an in-memory contract engine (`familiesMockState.ts`) seeded by `families.fixtures.ts`.
- Storybook flow stories (`FamilyFlows.stories.tsx`) and `e2e/families.spec.ts` (Playwright → Storybook on :6006) already encode the journeys via `data-testid`s.

Environment facts that shape the design:
- The wallet dev server is **webpack-dev-server on `http://localhost:9000`** (`historyApiFallback: true`, `hot: true`) — the same `devUrl` Tauri loads. Pointing a browser-based runner here exercises the real app shell + router.
- **Playwright cannot drive Tauri's WebDriver.** `tauri-driver` exposes the classic W3C WebDriver protocol; Playwright's only WebDriver story is experimental **BiDi**, a different protocol. The Tauri-documented native clients are WebdriverIO and Selenium.
- **`tauri-driver` has no macOS support** (no WKWebView driver). Native-webview e2e runs only on Linux/Windows — in CI.
- The node-families **contract is now deployed to sandbox** (one family, one member), enabling an optional real-IPC read smoke.

This design supersedes an earlier framing that made WebdriverIO + `tauri-driver` the primary suite. Per the latest direction, the primary suite is **Playwright against the mock-wired dev server** (cross-platform, runs on macOS), and the native-webview run is an **optional CI validation leg**.

## Goals / Non-Goals

**Goals:**
- Render the existing Family page inside the real wallet app shell, backed by the Storybook mock providers, with zero chain/IPC dependency.
- A primary e2e suite that runs **everywhere including the developer's Mac**, reusing the existing Playwright dependency, selectors, and journeys.
- Keep all mock code out of the production bundle (compile-time elimination).
- Provide an optional native-binary validation leg (WebdriverIO + `tauri-driver`) in Linux CI for higher fidelity.
- Make use of the sandbox contract deployment via an optional real-IPC read smoke.

**Non-Goals:**
- No real IPC/chain *write* wiring (remains parent-change tasks 9.4/9.5); the sandbox tier is read-only.
- No macOS native-webview e2e (unsupported by Tauri).
- No destructive lifecycle mutations against the shared sandbox.
- No change to the Family feature's behavior or its specs.

## Decisions

**D1 — Playwright against the mock-wired dev server is the PRIMARY suite.**
Point Playwright at `http://localhost:9000` with the mock flag on, and replay the journeys against the real app shell + router. Rationale: runs on every OS (incl. the developer's Mac), reuses the already-present `@playwright/test` dep and the existing selectors/specs, and tests the actual app chrome rather than Storybook iframes. *Alternative considered:* keep driving Storybook stories — lower fidelity (no router/app shell) and no closer to the real app. *Alternative considered:* Cypress — no advantage over the Playwright suite already in the repo.

**D2 — Dedicated mock ENTRY (gated by the build flag); persona chosen at runtime within it.**
*Revised during apply:* the original "swap the families provider inside the prod `main` entry" cannot work in a browser — `main.tsx` → `AppCommon` → `AppProvider` is Tauri-coupled at bootstrap (imports `tauri-forage`, `@tauri-apps/api/app`, ~9 `requests/*`, fires `invoke` effects on mount) and the main routes are login-gated, so a plain browser never reaches `/family`. Instead, add a **dedicated mock entry** (`src/main.mock.tsx` + generated `main.mock.html`) that mounts the real `HashRouter` + real `ApplicationLayout` + the Family page, but with the existing Storybook mocks (`MockMainContextProvider` for the app bootstrap + `MockFamiliesContextProvider` for families) — no Tauri, no login gate, browser-safe. The webpack flag (`WALLET_MOCK_FAMILIES=on|off`, default `off`, also added to `EnvironmentPlugin`) **gates whether the mock entry + its HTML are built at all**, so production builds never include it (cleaner than tree-shaking a swapped import — the mock code is simply never an entry). The **persona** (`buildOwnerFlowStore` / `buildOperatorFlowStore` + sender) is read at *runtime* from `?persona=owner|operator` (default `owner`) on `window.location.search`, so a **single** dev server serves both personas and Playwright just navigates to different URLs. The real `/family` route (`FamilyPageRoute.tsx` → `FamiliesContextProvider`) and `FamilyPage.tsx` are left untouched, keeping the merged Code Connect mapping valid. *Alternatives considered:* in-place provider swap (rejected — bootstrap can't boot in a browser); thin shell without `ApplicationLayout` (rejected — little fidelity over the Storybook suite being replaced); native-only via Tier 2 (rejected — never exercises the shell on macOS).

**D3 — Reuse the existing fixtures and selectors verbatim.**
The mock-wired dev server seeds the same fixtures as the Storybook flows, and the page already exposes the journey `data-testid`s, so the Playwright journeys mirror `FamilyFlows.stories.tsx` step-for-step. The same selectors then carry over to the optional WebdriverIO leg, keeping all suites observably equivalent.

**D4 — Optional native-webview validation: WebdriverIO + `tauri-driver` in CI (not Playwright).**
For "how far can we get against the actual binary," follow the Tauri WebDriver-in-CI flow: Ubuntu runner, `xvfb-run` for a headless display, `libwebkit2gtk-4.1-dev` + `webkit2gtk-driver` + `tauri-driver` (cargo, `--locked`), WebdriverIO driving the packaged app's native WebKitGTK webview. Playwright is not an option here (protocol mismatch, D-context). This leg is optional and starts non-blocking.

**D5 — Skip-not-fail on unsupported platforms / missing tools.**
The WebdriverIO leg detects macOS (or a missing `tauri-driver`/`webkit2gtk-driver`) and skips with a clear message, so invoking it locally on a Mac is a no-op rather than a red failure. The Playwright suite is the local source of truth; the native leg is CI-only.

**D6 — CI: primary step in `build`, native leg as a separate job.**
The merged `ci-nym-wallet-frontend.yml` has one `build` job (ubuntu-22.04: install → tsc → lint → unit tests → build-storybook → upload). Add the **primary Playwright e2e as a step in `build`** (the Playwright suite is not yet in CI — only unit tests + Storybook build are). Add the **native-webview run as a separate job** (`needs: build` or independent) because it adds a Rust/Tauri compile + system webdriver deps that would slow and couple the main job; start it `continue-on-error`.

**D7 — Sandbox real-IPC smoke is a separate, optional, READ-only tier.**
The sandbox contract (one family/one member) lets us smoke the real `FamiliesContextProvider` + `src/requests/families.ts` against a live chain — validating the IPC wiring that the mock deliberately stands in for (parent-change 9.4). Keep it read-only and separate from the deterministic mock e2e: a shared sandbox is a poor place to run create/kick/disband lifecycles, and live-network reads are inherently flakier. *Alternative considered:* fold sandbox into the main e2e — rejected (non-determinism + shared-state mutation).

**D8 — Native leg is Linux-only initially (no Windows leg yet).**
Tier 2 ships as a single Ubuntu job. Adding the Windows/WebView2 leg (also supported by `tauri-driver`) doubles CI cost and maintenance for a tier that is already optional/`continue-on-error`; WebKitGTK on Linux is the higher-value target since it is closest to the Linux desktop builds. Revisit Windows only if a WebView2-specific regression surfaces.

**D9 — Sandbox smoke ships as a documented MANUAL step first, pinning the known family id.**
A live sandbox read needs a connected, funded wallet account, which is not provisionable non-interactively in CI today (mnemonic in secrets + network + chain availability). So D7's smoke starts as a documented manual procedure that pins the known sandbox family id and asserts render/shape, not exact contents. It graduates to a CI job only once a sandbox test account can be provisioned headlessly — tracked as a follow-up, not a blocker.

**D10 — One Playwright suite: repoint to the dev server, retire the Storybook-iframe specs.**
Rather than maintain two Playwright suites, repoint the single `e2e/families.spec.ts` at the mock-wired dev server (`:9000`). The Storybook `play` functions remain as Storybook-level interaction coverage (runnable via the test-runner), but we do not keep a parallel Playwright-against-Storybook suite — it would be lower fidelity and a drift source for no added coverage.

## Risks / Trade-offs

- **Browser fidelity ≠ packaged app** → The primary Playwright suite tests the web frontend, not the native binary or real `invoke`; the optional WebdriverIO leg covers the binary, and D7 covers real IPC. The three tiers together close the gap the mock leaves.
- **WebKitGTK rendering/timing differs from Chromium (native leg)** → Journeys wait on `data-testid`s (as the Storybook flows do) with low mock latency and generous CI timeouts.
- **Build-flag branch could ship mock code** → Default off; bundle/guard check + the spec scenario "Production build excludes mock code" lock it in.
- **Provider seam breaks the merged Figma Code Connect mapping** (`FamilyPage.figma.tsx` → `example: () => <FamilyPage />`) → Keep mock/real selection in a *separate* module (D2); never make `FamilyPage`'s module depend on the flag or on Tauri.
- **Theme swap (Nym 2.0) under test** → Confirmed color-only (families components untouched by the merge); journeys assert visibility/test ids, not pixels.
- **`tauri-driver`/`webkit2gtk-driver` version drift in CI** → Pin `tauri-driver` (`cargo install --locked`), install a known WebKitGTK driver in the runner, cache cargo bin; the leg is `continue-on-error` until stable.
- **Sandbox state drifts / contract redeploys** → The read smoke asserts shape/render, not exact contents (or pins to a known family id) so a changed fixture doesn't hard-fail; keep it non-blocking.

## Migration Plan

Additive only. Rollout: (1) build-time flag + provider seam; (2) repoint Playwright at the mock-wired dev server and add it to the `build` CI job; (3) add the optional WebdriverIO native job (`continue-on-error`); (4) add the optional sandbox read smoke. Any optional leg can be dropped/gated without affecting the app or the primary suite.

## Open Questions

All four prior open questions are now resolved:
- **Persona handling** → single dev server; persona via runtime `?persona=` URL param inside the build-gated mock branch (**D2**).
- **Windows native leg** → no; Linux-only initially (**D8**).
- **Sandbox smoke placement / family id** → documented manual step first, pinning the known sandbox family id; CI only once a headless test account exists (**D9**).
- **Retire Storybook-iframe Playwright specs?** → yes; one suite, repointed at the dev server (**D10**).

Residual (a follow-up, not a blocker): provisioning a headless sandbox test account would let D9's smoke graduate from manual to CI. **Update:** that account now exists (sandbox `n13jtj2...e0tzv`, mnemonic in vault secret `TAURI-WALLET-MNEMONIC`) — the graduation is now actionable, tracked in the `node-families-real-ipc` change (§4–5).
