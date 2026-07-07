> Status: implemented. Checkboxes reflect what actually shipped; notes call out where the build deviated from the original plan.

## 1. Page alignment

- [x] 1.1 Confirm the wallet-wide layout standard used by Balance/Bonding/Delegation pages
- [x] 1.2 `FamilyPage` uses the shared `PageLayout` wrapper (instead of an ad-hoc `p: 4`) so cards align with the rest of the wallet

## 2. Countdown timer (D1)

- [x] 2.1 Extract the ticking clock into `useNowSecs` (`src/hooks/useNowSecs.ts`): `useState` seeded with `Math.floor(Date.now()/1000)`
- [x] 2.2 1-second `setInterval` with cleanup on unmount inside the hook
- [x] 2.3 `nowSecs` flows out via `ctx.nowSecs`; consumed by `FamilyMembersTable` and `InviteCard`

## 3. Per-action loading (D2)

- [x] 3.1 Replace global `isExecuting` with an `executingAction` discriminant on `FamiliesContext` (set/cleared by the provider's `run(action, fn)` helper)
- [x] 3.2 Each button shows its loading label only when `ctx.executingAction` equals its own action; others are disabled but not "loading"
- [x] 3.3 ~~Pass per-form `editBusy`/`inviteBusy` flags~~ — superseded by the discriminant; also "Edit family" moved to `FamilySettingsPage`, so it no longer sits next to "Send invite"

## 4. Membership surface in My family tab (D3)

- [x] 4.1 Remove the "Current family" block from the Invites surface (`OperatorInvitesPage`)
- [x] 4.2 Surface membership via `MyNodeFamilySection` (through `ControlledNodeSections`) in the My family tab, in its own `FamilyContentPanel`
- [x] 4.3 Own family vs another wallet's family rendered as separate, visually distinct panels
- [x] 4.4 `LeaveFamilyButton` is compact (not full-width) and names the family being left

## 5. Pending invitations shown once (D4)

- [x] 5.1 Remove the standalone `PendingInvitesList` render
- [x] 5.2 Pending invites become rows in the unified `FamilyMembersTable` with an inline Withdraw action (and Clear when expired)
- [x] 5.3 Verify the table is the only place pending invites appear and withdrawing still works

## 6. Create pre-validation (D5)

- [x] 6.1 `CreateFamilyEntry` calls `useFamilyMembership(controlledNodeIds[0])`
- [x] 6.2 When the node is already in a family, hide the create form and render the membership panel (`MyNodeFamilySection`) instead — cleaner than an inline alert + disabled button
- [x] 6.3 `handleCreate` short-circuits if the node is in a family (defence-in-depth); never blocks while the query is loading

## 7. NYM-1559 — invited node routed to Pending

- [x] 7.1 Pending rows derived from the live invitations query (`usePendingInvitationsForFamily`); joined/rejected/removed from the member-list query
- [x] 7.2 `deriveMemberSections` (`familyMemberSections.ts`) maps statuses to sections
- [x] 7.3 A freshly-invited node shows in Pending immediately and not in Joined/Rejected/Removed
- [x] 7.4 A node currently joined is removed from the rejected/removed buckets (no duplication)

## 8. NYM-1558 — auto-add owner's node at family creation

- [x] 8.1 Implemented in the node-families contract's `CreateFamily` handler (PR #6891), not the wallet — atomic, no FE invite/accept dance
- [x] 8.2 Contract enrols the owner's bonded, not-unbonding node as the founding member (`members = 1`); skips unbonding / absent nodes
- [x] 8.3 Contract spec updated at `openspec/specs/node-families-contract/spec.md`
- [x] 8.4 ~~`CreateFamilyForm` "your node will be added automatically" hint~~ — not needed; enrolment is transparent
- [x] 8.5 Owner's node appears in the Joined rows after the create tx settles, with no manual invite step

## 9. NYM-1560 — clean member history (superseded by the unified table)

- [x] 9.1 ~~`COLLAPSED_LIMIT = 3` "See all" truncation~~ — dropped in favour of the single `FamilyMembersTable`
- [x] 9.2 History kept clean by de-duplication: a currently-joined node is not also shown in rejected/removed
- [x] 9.3 Pending and Joined rows are always fully visible

## 10. Invite notification badge (D10)

- [x] 10.1 `useControlledNodeIds` (`src/hooks/useControlledNodeIds.ts`) resolves controlled node ids without the Bonding/Families providers (for the always-on nav)
- [x] 10.2 `usePendingInviteCountForNodes` (`src/context/families.tsx`) sums non-expired pending invites via `useQueries`, sharing the `pendingForNode` cache
- [x] 10.3 `InviteNotificationBadge` styled mint count badge (hides at 0)
- [x] 10.4 Wired into the sidebar "Family" entry (`Nav.tsx`) — shows the count, not a bare dot
- [x] 10.5 Wired into the Family page "Invites" sub-tab label (`FamilyPage.tsx`), mirroring the count
- [x] 10.6 Expired invites excluded; badge clears after the last active invite is resolved

## 11. Tests and QA

- [x] 11.1 Member-section derivation covered (`familyMemberSections` / helpers tests)
- [x] 11.2 Storybook + mocks updated for the new components (`FamilyFlows.stories.tsx`, mock state/fixtures)
- [x] 11.3 e2e specs updated (`e2e/families.spec.ts`, `e2e/shared/families.ts`)
- [ ] 11.4 Manual verification against the Figma wireframe (node 1859-981) and the QA edge cases
