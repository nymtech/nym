## Why

`NymCard`'s test-id prop is `dataTestid` (camelCase), but **17** call sites across the wallet pass `data-testid` (kebab-case), which `NymCard` silently ignores. Those intended test ids (`member-list`, `create-family-form`, `invite-card-<id>`, `operator-node-<id>-family`, `dissolve-family-card`, `family-summary`, `balance-usd-approx`, error/gateway cards, …) never reach the DOM. Worse, when a `data-testid` is dropped `NymCard` stamps the header with the **title text** as a test id (e.g. `Node 201`, `Current family`), producing misleading ids.

This silently breaks UI test selectors: the Node Families Storybook `play` functions and the first Playwright pass both targeted ids that don't render (discovered during the `node-families-tauri-webdriver-e2e` work, which had to scope around it). Fixing the component restores the intended, consistent test-id contract for the whole app.

## What Changes

- `NymCard` SHALL accept a standard `data-testid` prop (in addition to the existing `dataTestid`) and apply the resolved id to the card **root** element, so it wraps the card's content and is usable as a scope container.
- Remove the title-as-test-id fallback (`data-testid={dataTestid || title}`) so cards no longer emit misleading ids derived from their title; emit a test id only when one is explicitly provided.
- The resolved id MUST appear on exactly one element (no duplicate root/header ids that would break strict locators).
- Follow-up cleanup: revert the scope-around workarounds in the Node Families e2e selectors (`e2e/shared/families.ts`) back to the now-rendering intended ids where it simplifies them.

## Capabilities

### New Capabilities
- `nymcard-testid-contract`: The `NymCard` component's contract for exposing a DOM test id (accept `data-testid`, single deterministic element, no title-derived ids).

### Modified Capabilities
<!-- None — no existing capability spec covers NymCard. -->

## Impact

- **Component**: `src/components/NymCard.tsx` (the fix). ~27 `NymCard` call sites; **17** currently-dropped `data-testid`s will start rendering (families, balance, gateway, error cards). No call-site edits required.
- **Tests**: `nym-wallet` Jest is node-env (no DOM render), so unit tests are unaffected; verify the wallet builds, `tsc`/eslint stay clean, the Node Families Playwright suite stays green, and (bonus) the Storybook `play` functions now resolve their original ids.
- **Risk**: low — no test currently queries by the title-fallback id (verified); the change only adds/relocates ids. Snapshot-style DOM tests (none today) would see new `data-testid` attributes appear.
- **Out of scope**: broad re-write of call sites to switch `dataTestid` → `data-testid` (both remain supported); behavioural/visual changes to `NymCard`.
