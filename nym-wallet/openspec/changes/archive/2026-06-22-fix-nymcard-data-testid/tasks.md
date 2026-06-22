## 1. Fix NymCard

- [x] 1.1 In `src/components/NymCard.tsx`, accept a `'data-testid'?: string` prop alongside `dataTestid`; resolve `dataTestidAttr ?? dataTestid`.
- [x] 1.2 Apply `data-testid` to the root `<Card>` element unconditionally (header or not); rendered only when defined.
- [x] 1.3 Remove the `dataTestid || title`/`'nym-card'` fallback on `CardHeader` so no title-derived id is emitted and the id is not duplicated.

## 2. Verify

- [x] 2.1 `pnpm tsc` clean ("No errors found"); eslint clean on `NymCard.tsx`; mock dev build compiles.
- [x] 2.2 `pnpm test` (Jest) green — **85/85** (after removing 328 stray compiled `.js` artifacts that were shadowing the `.ts` sources and breaking Jest; unrelated to this fix).
- [x] 2.3 Node Families Playwright suite green (3/3). Spot-checked in the mock app: `node-invite-group-201` and `invite-card-2` now render (were dropped before), and the misleading title ids (`Node 201`, `Current family`) are gone.
- [ ] 2.4 (Bonus) Confirm the Storybook Node Families `play` functions resolve their original ids — not run (would need a Storybook run); the ids they target now render in the DOM.

## 3. Cleanup (optional)

- [ ] 3.1 Simplify `e2e/shared/families.ts` — **skipped intentionally**: the current selectors work and remain correct (invite buttons are family-id keyed via `ConfirmActionButton` regardless of this fix; `operator-node-<n>` is a valid scope). No change needed.
- [x] 3.2 Updated the `NymCard` caveat note in `e2e/README.md` to record that the prop is fixed.
