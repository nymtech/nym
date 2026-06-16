## 1. Dedicated mock entry (D2)

- [x] 1.1 Add `WALLET_MOCK_FAMILIES` (default `off`) to webpack `EnvironmentPlugin`, and conditionally register a `mainMock` entry + a `main.mock.html` HtmlWebpackPlugin output **only when the flag is `on`**, so production builds never include the mock entry.
- [x] 1.2 Create `src/main.mock.tsx`: mount real `HashRouter` + real `ApplicationLayout` + a `/family` route, wrapped in `MockMainContextProvider` (mock app bootstrap) + `MockFamiliesContextProvider` (mock families) + `QueryClientProvider` + `NymWalletTheme` — no Tauri, no login gate. Leave `FamilyPage.tsx` and the real `FamilyPageRoute.tsx` untouched (keeps Code Connect valid).
- [x] 1.3 In `main.mock.tsx`, read the persona at runtime from `?persona=owner|operator` (default `owner`) on `window.location.search` and seed `buildOwnerFlowStore` / `buildOperatorFlowStore` + the matching sender (reuse `families.fixtures.ts`); seed the hash to `#/family`. (Added a third `operator-seeded` persona for the multi-node assertion.)
- [x] 1.4 Confirm the Family page is reachable at `/main.mock.html?persona=...#/family` on the dev server (`:9000`), rendering inside the real layout/router (not a Storybook iframe), keeping the existing `data-testid`s. (Layout chrome `Nav`/`AppBar` verified Tauri-free; full browser render pending a live run — see 6.1.)
- [x] 1.5 Confirm a default (flag `off`) production build is unchanged — no `mainMock` entry, no `main.mock.html`, no mock-engine code; the real `/family` route still wires `FamiliesContextProvider`. (Verified: config builds entry `auth,main,log` with flag off; `+mainMock` only with flag on.)
- [x] 1.6 Add an npm script to launch the single mock-wired dev server (`webpack:dev:mock` = `WALLET_MOCK_FAMILIES=on webpack serve --config webpack.dev.js`); both personas are reached on the one server via `?persona=`.

## 2. Primary e2e — Playwright against the mock-wired dev server

- [x] 2.1 Repoint `playwright.config.ts` from Storybook (:6006) to the mock-wired dev server (`baseURL http://localhost:9000`), with a single `webServer` (`webpack:dev:mock`) and `reuseExistingServer` locally; tests pick persona via the `?persona=` URL.
- [x] 2.2 Port the owner lifecycle journey (create → invite → accept → kick → disband) against `main.mock.html?persona=owner`, reusing the existing `data-testid` selectors and the Storybook flow steps.
- [x] 2.3 Port the operator lifecycle journey against `?persona=operator` (accept → leave, then reject), asserting the reject-node invite group ends empty.
- [x] 2.4 Port the multi-node operator invite-states assertion (`node-invite-group-201` present, `node-invite-group-203-empty`) against `?persona=operator-seeded`.
- [x] 2.5 Handle portalled confirmation dialogs via their global test ids (page-scoped, mirroring the Storybook `screen` queries).
- [x] 2.6 Retire the old Storybook-iframe specs (D10): replaced `e2e/families.spec.ts` with the dev-server journeys and factored shared selectors into `e2e/shared/families.ts` for parity (Storybook `play` functions stay as Storybook-level coverage).
- [x] 2.7 Confirm the suite runs green locally on macOS — **DONE: 3/3 passing** (clean cold run, Chromium). Required selector corrections: the original Storybook-derived ids on `NymCard` don't render (its prop is `dataTestid`, not `data-testid`), so journeys scope by the `operator-node-<n>` wrapper and key invite buttons by family id.

## 3. CI — wire the primary suite in

- [x] 3.1 Add a Playwright e2e step to the existing `build` job in `.github/workflows/ci-nym-wallet-frontend.yml` (install `--with-deps chromium` + run `test:e2e`).
- [x] 3.2 The step launches the mock-wired dev server via Playwright's `webServer` and fails the job on any journey failure (no `continue-on-error`).

## 4. Optional — native-webview validation leg (WebdriverIO + tauri-driver)

- [x] 4.1 Add `webdriverio` + `@wdio/{cli,local-runner,mocha-framework,spec-reporter}` + `tsx` to dev deps; documented `cargo install tauri-driver --locked` (README + CI).
- [x] 4.2 Create `wdio.conf.ts` (starts `tauri-driver`, `tauri:options.application` → release binary, mocha timeouts) + `test:e2e:tauri` script. **Binary wiring DONE:** `src-tauri/tauri.mock.conf.json` overrides the window to boot `main.mock.html` (owner persona); `tauri:build:mock` = `WALLET_MOCK_FAMILIES=on webpack:prod` + `tauri build --no-bundle --config tauri.mock.conf.json`. Operator persona reached via in-webview `browser.url('tauri://localhost/main.mock.html?persona=operator')` in the spec.
- [x] 4.3 Implement the skip-not-fail guard (`e2e-tauri/run.mjs`): macOS / missing `tauri-driver` / missing `webkit2gtk-driver` → exit 0 with a clear message (design D5).
- [x] 4.4 Reuse the journey selectors from §2 via `e2e/shared/families.ts` (`e2e-tauri/families.tauri.ts`), so the native leg asserts identical outcomes.
- [x] 4.5 Add a **separate** `e2e-tauri` CI job (ubuntu-22.04, `continue-on-error`) following the Tauri WebDriver-in-CI flow: `libwebkit2gtk-4.1-dev` + `webkit2gtk-driver` + `xvfb`, Rust + cache, `cargo install tauri-driver --locked`, build mock binary, run under `xvfb-run`.

## 5. Optional — sandbox real-IPC read smoke (manual first, D9)

- [x] 5.1 Documented (e2e/README.md) a manual read-only smoke: connect to a sandbox account, open the Family page, confirm it renders the known sandbox family/member via real `FamiliesContextProvider` + `requests/families.ts` (no state-changing tx).
- [x] 5.2 Documented: pin the known sandbox family id and assert render/shape, not exact contents; kept separate + non-blocking vs the mock suites.
- [x] 5.3 Documented the follow-up: promote to a non-blocking CI job once a sandbox test account can be provisioned headlessly.

## 6. Verification & docs

- [x] 6.1 Run the primary Playwright suite locally (macOS) — **DONE: 3/3 green** against the mock-wired app shell. (CI execution still pending the first push.) Fixes needed to get a clean browser render: skip React Refresh/HMR + the dev-server live-reload client in the mock build (`webpack.dev.js`, avoids missing `core-js-pure`/`ansi-html-community`); add the relative `node_modules` walk for the mock build (pnpm strict + absolute `resolve.modules` dropped `object-assign`); make `src/utils/common.ts` resolve `getCurrentWebviewWindow()` lazily (was crashing at import outside Tauri).
- [x] 6.2a **Fixed the pre-existing `webpack:prod` failure** that blocked the mock binary (and `pnpm build` generally). Root cause: the shared `ForkTsCheckerWebpackPlugin` runs in `mode: 'write-references'` (emit) and, with the wallet's `allowJs: true` + no `outDir`, emitted `.js` next to sources — polluting `src` (broke Jest), erroring "would overwrite input file" on `.test.js`/`.test.ts` pairs, and type-checking test files (jest globals). Contained wallet fix: add `declare module '*.css'` (`src/typings/css.d.ts`), set `outDir: ./.tsbuild` (redirects the emit out of `src`; `tsc --noEmit`/ts-loader/ts-jest ignore it), and exclude `**/*.test.*` from the type-check program (Jest still type-checks tests via ts-jest). Result: `webpack:prod` exits 0, emits `dist/main.mock.html`, no `src` pollution.
- [x] 6.2b Built the mock binary locally (`tauri:build:mock`, ~3m48s) and **visually confirmed** it boots straight into the Family page in the real app shell (owner persona / "Create a family" entry, network "Testnet Sandbox", sidebar "Version mock" — i.e. `MockMainContextProvider` + `MockFamiliesContextProvider`, no Tauri IPC/login). Fixed the binary path in `wdio.conf.ts` (`target/release/NymWallet` — the Cargo workspace target is at the wallet root, not `src-tauri/target`).
- [ ] 6.2c Run the WebdriverIO suite against the binary — **Linux/Windows-only** (`tauri-driver`), so it executes in the `e2e-tauri` CI job (skips on macOS via `run.mjs`). This is the one remaining unrun step.
- [x] 6.3 Confirm `tsc` + eslint stay clean and the production build is unaffected — verified: after `pnpm install`, `tsc` is fully clean (exit 0); `main.mock.tsx` + `utils/common.ts` lint clean; webpack prod-safe (no `mainMock` entry with flag off).
- [x] 6.4 Confirm the provider seam didn't break Code Connect (seam is a separate entry; `FamilyPage.tsx` + `FamilyPageRoute.tsx` untouched; `FamilyPage.figma.tsx` now type-checks once `@figma/code-connect` is installed) and the Nym 2.0 theme swap left journey `data-testid`s intact (color-only).
- [x] 6.5 Document the tiered setup + mock-flag usage (`e2e/README.md`).

## 7. Visual flow report (bonus)

- [x] 7.1 Capture a full-page, captioned screenshot at each journey step (`shot()` in `e2e/shared/report.ts`) — written to `e2e-report/screenshots/<test>/NN-label.png` and attached to the Playwright report.
- [x] 7.2 Assemble a static `e2e-report/index.html` filmstrip (per-test, ordered, captioned) via Playwright `globalSetup`/`globalTeardown` (`report.globalSetup.ts` resets, `report.globalTeardown.ts` builds).
- [x] 7.3 Stage + upload `e2e-report/` in the CI `build` job alongside Storybook, with `if: always()` so a failing run still publishes the filmstrip for inspection; gitignore `e2e-report/` + `playwright-report/`.
- [x] 7.4 Verified locally: `pnpm test:e2e` green (3/3), report renders 11 frames across the owner/operator/multi-node journeys.
