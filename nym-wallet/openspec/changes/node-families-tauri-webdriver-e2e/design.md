## Context

The Node Families feature (parent change `node-families-wallet`) is complete and verified in Storybook. The page component is already cleanly decoupled from Tauri:

- `src/pages/families/FamilyPage.tsx` — pure page, consumes `useFamiliesContext()`, no Tauri imports.
- `src/context/FamiliesContextProvider.tsx` — the **real** provider; the only families file importing `./main` (Tauri runtime).
- `src/context/mocks/families.tsx` (`MockFamiliesContextProvider`) — drives the page from an in-memory contract engine (`familiesMockState.ts`) seeded by `families.fixtures.ts`.
- `src/pages/families/FamilyPageRoute.tsx` (`FamilyPageWithProvider`) — wraps `FamilyPage` in the real provider for the live app route.
- Storybook flow stories (`FamilyFlows.stories.tsx`) and `e2e/families.spec.ts` (Playwright → Storybook on :6006) already encode the journeys via `data-testid`s.

Constraints established up front:
- **Playwright cannot drive Tauri's WebDriver.** `tauri-driver` exposes the classic W3C WebDriver protocol; Playwright speaks CDP/BiDi. The Tauri-documented clients are Selenium and WebdriverIO.
- **`tauri-driver` has no macOS support** (no WKWebView driver). Native-webview e2e therefore runs only on Linux/Windows — in CI, not on the developer's Mac.

Decisions taken with the user: **WebdriverIO + `tauri-driver` in CI** for the native-webview suite, and a **build-time env flag** for mock-provider selection.

## Goals / Non-Goals

**Goals:**
- Render the existing Family page inside the real Tauri desktop shell, backed by the Storybook mock providers, with zero chain/IPC dependency.
- Keep all mock code out of the production bundle (compile-time elimination).
- Replay the owner and operator journeys against the native webview via WebdriverIO + `tauri-driver`, reusing the existing `data-testid` selectors and assertions.
- Run the native-webview suite in Linux CI; keep the existing Playwright/Storybook suite as the cross-platform/local check.

**Non-Goals:**
- No real IPC/chain wiring (remains parent-change tasks 9.4/9.5).
- No macOS native-webview e2e (unsupported by Tauri).
- No migration of the existing Playwright/Storybook suite to WebdriverIO — both coexist.
- No change to the Family feature's behavior or its specs.

## Decisions

**D1 — WebdriverIO + `tauri-driver` for native-webview e2e (not Playwright).**
Playwright cannot connect to `tauri-driver`'s W3C WebDriver endpoint. WebdriverIO is the Tauri-documented client and natively speaks WebDriver classic. The journeys are simple click/type/assert sequences, so the port from Playwright is mechanical. *Alternative considered:* Selenium — also supported, but WebdriverIO's config + assertion ergonomics are closer to the existing Playwright specs, lowering porting cost.

**D2 — Build-time env flag selects the provider (not runtime).**
A webpack `DefinePlugin` constant (e.g. `process.env.WALLET_MOCK_FAMILIES`) gates the import of `MockFamiliesContextProvider` vs `FamiliesContextProvider` behind a `const MOCK = ...` check. With the flag off (default), the dead branch and its transitive mock imports tree-shake out — no mock engine in production. *Alternatives considered:* runtime URL/query flag (ships mock code in prod unless guarded; rejected for prod-safety) and a fully separate entry point (heavier; the flag is enough since the page is already provider-agnostic).

**D3 — Reuse the existing fixtures and selectors verbatim.**
The mock build seeds `buildOwnerFlowStore` / `buildOperatorFlowStore` and the page already exposes the journey `data-testid`s. The WebdriverIO journeys mirror `FamilyFlows.stories.tsx` step-for-step so the two suites stay observably equivalent. Persona/seed selection at launch reuses the same flag mechanism (e.g. `WALLET_MOCK_FAMILIES=owner|operator`) so each journey gets a deterministic start state.

**D4 — Mock build artifact for the harness.**
`tauri-driver` launches a built binary, so the harness needs a mock-wired build. Options: (a) `tauri dev` with the mock webpack config behind `devUrl`, or (b) a one-off `tauri build` with the mock define set, then point WebdriverIO at the produced binary. CI uses (b) for a stable artifact; local debugging (on Linux) can use (a). The Tauri `frontendDist`/`devUrl` already point at the webpack output, so only the define + entry wiring changes.

**D5 — Skip-not-fail on unsupported platforms.**
The WebdriverIO suite detects macOS (or a missing `tauri-driver`/`WebKitWebDriver`) and skips with a clear message, so `pnpm test:e2e:tauri` on a Mac is a no-op rather than a red failure. CI is the source of truth.

**D6 — Separate CI job, not a step in `build` (reconciled with recent merge).**
The merged `ci-nym-wallet-frontend.yml` has one `build` job (ubuntu-22.04) doing install → tsc → lint → unit tests → build-storybook → upload. The native-webview suite is added as a **separate job** rather than appended to `build`: it additionally needs a Rust/Tauri compile, `WebKitWebDriver`, and `tauri-driver`, which would slow every `build` run and couple unrelated failures. It can `needs: build` (reuse nothing) or run independently. *Also reconciled:* the existing Playwright→Storybook suite (`test:e2e`) is **not yet in CI** — only unit tests + Storybook build are — so the "keep Playwright as the cross-platform check" intent means optionally wiring `test:e2e` into CI too, not assuming it already runs there.

## Risks / Trade-offs

- **Native-webview e2e can't run on the developer's Mac** → Keep the Playwright/Storybook suite as the local check (works everywhere); rely on Linux CI for native-webview coverage; D5 makes the local invocation a clean skip.
- **WebKitGTK rendering/timing differs from Chromium** → Journeys use explicit waits on `data-testid`s (as the Storybook flows already do) and zero/low mock latency for the auto-run paths; generous step timeouts in CI.
- **Build-flag branch could accidentally ship mock code** → Default flag off; add a check (bundle assertion or the existing `check:singletons`-style guard) and the spec scenario "Production build excludes mock code" to lock it in.
- **`tauri-driver` + `WebKitWebDriver` version drift in CI** → Pin `tauri-driver` (`cargo install --locked`) and install a known WebKitGTK driver in the CI image; cache cargo bin.
- **Duplicated journey logic across two suites drifts over time** → Both target identical `data-testid`s and assert identical outcomes (parity requirement); factor shared selector/step constants if drift appears.
- **Provider seam breaks the merged Figma Code Connect mapping** (`FamilyPage.figma.tsx` does `example: () => <FamilyPage />`) → Keep the mock/real selection in a *separate* module (D2); never make `FamilyPage`'s own module depend on the flag or on Tauri, so it stays importable in isolation. The `src/**/*.figma.tsx` include is unaffected.
- **Theme swap (Nym 2.0) changing appearance under test** → Confirmed color-only (no DOM/`data-testid` change, families components untouched by the merge); journeys assert visibility/test ids, not pixels, so parity holds across the palette change.

## Migration Plan

Additive only — no rollback of existing behavior needed. Rollout: (1) add the build-time flag + provider seam; (2) add the WebdriverIO config/spec; (3) add the Linux CI job. If the native-webview suite proves flaky in CI, it can be gated/`continue-on-error` while the Playwright suite continues to gate merges, with no impact on the app itself.

## Open Questions

- Build vs dev-server launch for the CI harness (D4 (a) vs (b)) — default to a built binary; revisit if build time in CI is prohibitive.
- Whether to also add a Windows CI leg (supported by `tauri-driver`) or keep Linux-only initially — start Linux-only.
- Exact flag surface for persona seeding (single tri-state `WALLET_MOCK_FAMILIES=owner|operator|off` vs a separate seed var) — resolve during tasks.
