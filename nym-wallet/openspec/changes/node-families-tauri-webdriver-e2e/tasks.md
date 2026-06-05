## 1. Build-time mock provider seam

- [ ] 1.1 Add a webpack `DefinePlugin` boolean for the families mock gate (`WALLET_MOCK_FAMILIES=on|off`, default `off`) in the dev/mock webpack config (mock-vs-real only; persona is runtime, see 1.3).
- [ ] 1.2 Introduce a provider-selection module that exports either `FamiliesContextProvider` (real) or `MockFamiliesContextProvider` (mock) based on the compile-time flag, behind a `const` guard so the unused branch tree-shakes. Keep this seam in its **own** module — do NOT make `FamilyPage.tsx` depend on the flag or on Tauri, so the merged `FamilyPage.figma.tsx` Code Connect mapping (`example: () => <FamilyPage />`) still imports it in isolation.
- [ ] 1.3 Have the Family route/entry consume the selection module; in mock mode, read the persona at runtime from a `?persona=owner|operator` URL param (default `owner`) and seed `buildOwnerFlowStore` / `buildOperatorFlowStore` accordingly (reuse `families.fixtures.ts`). The persona reader lives only inside the build-gated mock branch.
- [ ] 1.4 Ensure the Family page is reachable via normal in-app navigation on the dev server (`:9000`) and renders inside the app shell + router (not a Storybook iframe), keeping the existing `data-testid`s.
- [ ] 1.5 Verify a default (flag `off`) production build excludes families mock-engine code (inspect bundle / add a guard); confirm the real provider still wires unchanged.
- [ ] 1.6 Add an npm script to launch the single mock-wired dev server (e.g. `WALLET_MOCK_FAMILIES=on pnpm webpack:dev`); both personas are reached on the one server via `?persona=`.

## 2. Primary e2e — Playwright against the mock-wired dev server

- [ ] 2.1 Repoint `playwright.config.ts` from Storybook (:6006) to the mock-wired dev server (`baseURL http://localhost:9000`), with a single `webServer` (`WALLET_MOCK_FAMILIES=on`) and `reuseExistingServer` locally; tests pick persona via the `?persona=` URL.
- [ ] 2.2 Port the owner lifecycle journey (create → invite → accept → kick → disband) against `/family?persona=owner`, reusing the existing `data-testid` selectors and the Storybook flow steps.
- [ ] 2.3 Port the operator lifecycle journey against `/family?persona=operator` (accept → leave, then reject), asserting the reject-node invite group ends empty.
- [ ] 2.4 Port the multi-node operator invite-states assertion (`node-invite-group-201` present, `node-invite-group-203-empty`).
- [ ] 2.5 Handle portalled confirmation dialogs via their global test ids (mirror the Storybook `screen`-scoped queries).
- [ ] 2.6 Retire the old Storybook-iframe specs (D10): replace `e2e/families.spec.ts` with the dev-server journeys and factor shared selector/step constants for parity (Storybook `play` functions stay as Storybook-level coverage).
- [ ] 2.7 Confirm the suite runs green locally on macOS (Chromium; optionally WebKit project).

## 3. CI — wire the primary suite in

- [ ] 3.1 Add a Playwright e2e step to the existing `build` job in `.github/workflows/ci-nym-wallet-frontend.yml` (it is not in CI today — only unit tests + `build-storybook` run); install browsers (`npx playwright install --with-deps chromium`) and run `test:e2e`.
- [ ] 3.2 Ensure the step launches the mock-wired dev server (per persona) and fails the job on any owner/operator journey failure.

## 4. Optional — native-webview validation leg (WebdriverIO + tauri-driver)

- [ ] 4.1 Add `webdriverio` (+ runner) to `nym-wallet` dev deps; document `cargo install tauri-driver --locked`.
- [ ] 4.2 Create the WebdriverIO config that starts `tauri-driver`, points `tauri:options.application` at the mock-wired binary, and sets CI step timeouts; add a `test:e2e:tauri` script.
- [ ] 4.3 Implement the skip-not-fail guard: detect macOS / missing `tauri-driver` / missing `webkit2gtk-driver` and skip with a clear message (design D5).
- [ ] 4.4 Reuse the journey steps/selectors from §2 (shared constants) so the native leg asserts identical outcomes.
- [ ] 4.5 Add a **separate** CI job (ubuntu-22.04) following the Tauri WebDriver-in-CI flow: install `libwebkit2gtk-4.1-dev` + `webkit2gtk-driver` + `xvfb`, set up Rust + cache, `cargo install tauri-driver --locked`, build the mock-wired binary, run the suite under `xvfb-run`. Start it `continue-on-error` until stable.

## 5. Optional — sandbox real-IPC read smoke (manual first, D9)

- [ ] 5.1 Document a manual read-only smoke: connect the app to a sandbox account, open the Family page, and confirm it renders the known sandbox family/member via the real `FamiliesContextProvider` + `requests/families.ts` (no state-changing transaction).
- [ ] 5.2 Pin the known sandbox family id and assert render/shape rather than exact contents, so a contract redeploy doesn't hard-fail; keep it separate from and non-blocking relative to the mock suites.
- [ ] 5.3 (Follow-up, not a blocker) If/when a sandbox test account can be provisioned headlessly (mnemonic in CI secrets), promote the smoke to a non-blocking CI job.

## 6. Verification & docs

- [ ] 6.1 Run the primary Playwright suite in CI and locally (macOS) — confirm both owner and operator journeys pass against the app shell.
- [ ] 6.2 If implemented, confirm the native leg passes in Linux CI and skips cleanly on macOS.
- [ ] 6.3 Confirm `tsc` + eslint stay clean and the production build is unaffected (no mock code, no behavior change).
- [ ] 6.4 Confirm the provider seam didn't break Code Connect (`FamilyPage.figma.tsx` still type-checks) and the Nym 2.0 theme swap left all journey `data-testid`s intact.
- [ ] 6.5 Document the tiered setup (primary Playwright→dev-server; optional WebdriverIO→tauri-driver CI; optional sandbox read smoke) and the mock-flag usage in the wallet README / e2e comments.
