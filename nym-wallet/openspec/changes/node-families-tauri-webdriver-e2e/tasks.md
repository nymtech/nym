## 1. Build-time mock provider seam

- [ ] 1.1 Add a webpack `DefinePlugin` constant for the families mock flag (e.g. `process.env.WALLET_MOCK_FAMILIES`, tri-state `owner|operator|off`, default `off`) in the dev/mock webpack config.
- [ ] 1.2 Introduce a provider-selection module that exports either `FamiliesContextProvider` (real) or `MockFamiliesContextProvider` (mock) based on the compile-time flag, behind a `const` guard so the unused branch tree-shakes.
- [ ] 1.3 Have the Family route/entry consume the selection module instead of importing `FamiliesContextProvider` directly; in mock mode seed `buildOwnerFlowStore` / `buildOperatorFlowStore` per the persona flag (reuse `families.fixtures.ts`).
- [ ] 1.4 Ensure the Family page is reachable via normal in-app navigation in the mock build and renders inside the app shell (not a Storybook iframe), keeping the existing `data-testid`s.
- [ ] 1.5 Verify a default (flag `off`) production build excludes families mock-engine code (inspect bundle / add a guard); confirm the real provider still wires unchanged.

## 2. Mock-wired Tauri build

- [ ] 2.1 Add a mock webpack build script (and, if needed, a `tauri.conf` profile / `devUrl` wiring) that produces the app bundle with the mock define set.
- [ ] 2.2 Add a `tauri:dev`/`tauri:build` variant (npm scripts) that launches/produces the mock-wired binary for each persona configuration.
- [ ] 2.3 Manually confirm (on Linux, or via `tauri dev`) the mock-wired app opens to the correct persona entry state: owner → create-family entry; operator → seeded node invites.

## 3. WebdriverIO + tauri-driver harness

- [ ] 3.1 Add `webdriverio` (+ runner/test framework) to `nym-wallet` dev deps; document `cargo install tauri-driver --locked`.
- [ ] 3.2 Create the WebdriverIO config that starts `tauri-driver`, points `tauri:options.application` at the mock-wired binary, and sets sensible CI step timeouts.
- [ ] 3.3 Implement a skip-not-fail guard: detect macOS / missing `tauri-driver` / missing `WebKitWebDriver` and skip the suite with a clear message (design D5).
- [ ] 3.4 Add `test:e2e:tauri` npm script wiring the WebdriverIO run.

## 4. Journey specs (parity with Storybook)

- [ ] 4.1 Port the owner lifecycle journey (create → invite → accept → kick → disband) from `FamilyFlows.stories.tsx` / `e2e/families.spec.ts` to WebdriverIO, reusing the same `data-testid` selectors.
- [ ] 4.2 Port the operator lifecycle journey (accept → leave, then reject) to WebdriverIO, asserting the reject-node invite group ends empty.
- [ ] 4.3 Port the multi-node operator invite-states assertion (`node-invite-group-201` present, `node-invite-group-203-empty`).
- [ ] 4.4 Handle portalled confirmation dialogs by locating their global test ids (mirror the Storybook `screen`-scoped queries).
- [ ] 4.5 Factor shared selector/step constants so the Playwright and WebdriverIO suites stay observably equivalent (parity requirement).

## 5. CI integration

- [ ] 5.1 Add a Linux CI job that installs `WebKitWebDriver` (WebKitGTK) and `tauri-driver` (`cargo install --locked`, cached).
- [ ] 5.2 In the job, build the mock-wired binary (per persona) and run `test:e2e:tauri`; fail the job on any owner/operator journey failure.
- [ ] 5.3 Keep the existing Playwright/Storybook `test:e2e` job as the cross-platform check (unchanged); document the split in the e2e README/comment.
- [ ] 5.4 Optionally gate the native-webview job as non-blocking initially if flaky, with a note to promote it to required once stable.

## 6. Verification & docs

- [ ] 6.1 Run the WebdriverIO suite in CI (Linux) and confirm both owner and operator journeys pass against the native webview.
- [ ] 6.2 Confirm the macOS local invocation skips cleanly (no red failure).
- [ ] 6.3 Confirm `tsc` + eslint stay clean and the production build is unaffected (no mock code, no behavior change).
- [ ] 6.4 Document the two-suite setup (Playwright→Storybook for local/cross-platform; WebdriverIO→tauri-driver for native-webview CI) and the mock-flag usage in the wallet README / e2e comments.
