## Context

`src/components/NymCard.tsx` currently:

```tsx
<Card data-testid={hideHeader ? dataTestid : undefined} ...>
  {!hideHeader && title !== undefined && (
    <CardHeader data-testid={dataTestid || (typeof title === 'string' ? title : 'nym-card')} ... />
  )}
  ...children...
</Card>
```

Problems: (1) only reads `dataTestid` (camelCase) — the 17 call sites passing `data-testid` (kebab) are ignored; (2) when a header is shown the id lands on the `CardHeader`, not the content-wrapping root, so it can't scope children; (3) the `|| title` fallback emits misleading ids like `Node 201` / `Current family`.

Discovered while building `node-families-tauri-webdriver-e2e`: the e2e had to scope around this (using `operator-node-<n>` wrappers and family-id-keyed button ids) because `node-invite-group-<n>` / `invite-card-<n>` never rendered.

## Goals / Non-Goals

**Goals:** accept `data-testid`; put the resolved id on the card root (content-wrapping, scope-able); single element only; drop title-derived ids; keep `dataTestid` working.

**Non-Goals:** changing call sites from `dataTestid` to `data-testid` en masse; any visual/behavioural change; touching `ConfirmActionButton` (its `dataTestid` already renders correctly).

## Decisions

**D1 — Resolve `data-testid ?? dataTestid` and place it on the `<Card>` root, always.**
Destructure both props (`'data-testid': dataTestidAttr`, `dataTestid`) and compute `const testId = dataTestidAttr ?? dataTestid`. Apply `data-testid={testId}` to the outer `<Card>` unconditionally (header or not), so it wraps content and scoping works. *Alternative considered:* keep it on the header when a header exists — rejected (can't scope children, and is the current broken behaviour).

**D2 — Remove the `|| title` / `'nym-card'` fallback; render no attribute when unset.**
Emit `data-testid` only when `testId` is defined. Verified no test queries by a title-derived id, so this only removes noise. Avoids duplicate ids (root + header) that would break Playwright strict locators.

**D3 — Revert the e2e scope-arounds where the intended ids now render.**
With the fix, `node-invite-group-<n>` and `invite-card-<familyId>` render on real elements. Optionally simplify `e2e/shared/families.ts` back toward the intended ids. Keep the suite green either way — this is cleanup, not required for correctness.

## Risks / Trade-offs

- **17 previously-absent `data-testid`s start appearing** → could affect DOM snapshot tests. There are none today (Jest is node-env); mitigation: run the wallet build + the Node Families Playwright suite + `tsc`/eslint after the change.
- **A consumer relied on the title-derived id** → verified none do; if one surfaces, it can pass an explicit `data-testid`.
- **Type prop name** → React/TS allows `'data-testid'` as a prop key; type it explicitly in `NymCard`'s props so call sites keep type-checking.

## Migration Plan

Single-component change; additive (new ids appear, none removed except the unused title fallback). No rollback concerns. Verify by: build wallet, run `pnpm test`, `pnpm tsc`, and the Node Families Playwright suite (`pnpm test:e2e`).
