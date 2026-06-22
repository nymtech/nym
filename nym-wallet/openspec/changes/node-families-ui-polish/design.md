## Context

The Node Families feature (NYM-1199) was implemented on a feature branch and is approaching ship readiness. QA testing against Figma wireframes and three Jira acceptance criteria tickets (NYM-1558, NYM-1559, NYM-1560) surfaced a cluster of bugs and structural issues. All affected files are within `nym-wallet/src/` under the `families/` pages and `Families/` components directories.

Key constraints:
- Must not break the existing Tauri IPC layer (`familyRequests.*`)
- Tests at `src/components/Families/*.test.tsx` and `src/pages/families/*.test.tsx` must pass
- No new external dependencies; all fixes use existing React/MUI/TanStack Query patterns already in the codebase
- The `FamiliesContext` shape is consumed by Storybook/mock providers — any additions need defaults

## Goals / Non-Goals

**Goals:**
- Countdown timers tick live (1-second interval, cleaned up on unmount)
- Each form button reflects only its own in-flight operation
- Operator membership ("Current family" + Leave) surfaces in My family tab, not Node invites
- Pending invitations appear exactly once (in MemberList, not twice)
- Create form blocks with a clear warning when the owner's node is already in a family
- Page horizontal padding matches the wallet's standard layout
- Owner's nym-node is auto-joined at family creation (NYM-1558)
- Freshly-invited node appears in Pending immediately, not in Joined/Rejected (NYM-1559)
- Removed/Rejected member sections are collapsed to 3 entries with "See all (N)" (NYM-1560)
- Family nav item shows a badge/dot when the user's controlled node has pending invitations

**Non-Goals:**
- Pagination or server-side truncation of member lists
- Design changes beyond what is specified in the Figma wireframe (node 1859-981)
- Changes to Tauri backend / CosmWasm contract behaviour
- Changes to the Bonding or Delegation pages

## Decisions

### D1 — Live `nowSecs` via `useEffect` interval, not `useMemo`
**Decision:** Replace `const nowSecs = useMemo(() => Math.floor(Date.now() / 1000), [])` in `FamiliesContextProvider` with a `useState` initialized to `Math.floor(Date.now() / 1000)` and a `useEffect` that runs `setInterval(() => setNowSecs(Math.floor(Date.now() / 1000)), 1000)` with cleanup.

**Why over alternatives:**
- *Prop drilling `Date.now()` from each display site* — duplicates timers, no single source of truth
- *`useReducer` tick* — unnecessarily complex for a scalar value
- The existing pattern already distributes `nowSecs` through context to all consumers; changing just the source is the minimal diff

### D2 — Per-action loading via two local `useState` flags in `OwnerManagementPage`
**Decision:** Add `const [editBusy, setEditBusy] = useState(false)` and `const [inviteBusy, setInviteBusy] = useState(false)` in `OwnerManagementPage`. Wrap `handleEdit` / `handleInvite` to set/clear their respective flag, independent of `ctx.isExecuting`. Pass `editBusy` to `EditFamilyForm.isSubmitting` and `inviteBusy` to `InviteNodeForm.isSubmitting`.

**Why not split `isExecuting` in the context:** The context's `isExecuting` is also used by delete, kick, revoke, leave — all of which correctly want to disable their own button. The dual-button bleed is only in `OwnerManagementPage` where two forms sit side-by-side. Local state is the minimal, correct fix.

**Why not a single `executingOp: string | null` discriminant:** Unnecessary generality — only two forms share a screen.

### D3 — Operator membership card moved into `OwnerManagementPage` / `FamilyPage` owner tab
**Decision:** In `OperatorInvitesPage`, remove the "Current family" `NymCard` block. In `FamilyPage` (or `OwnerManagementPage`), add an `OperatorMembershipCard` that renders when `controlledNodeIds` has a node that is already a member of a family. This requires reading `useFamilyMembership(nodeId)` from within the My family tab.

**Placement:** Rendered between the family summary card and the Edit/Invite grid, so it is visible immediately when the owner lands on the tab.

**Leave button:** Render as a standard MUI `Button` without `fullWidth`, wrapped in a `Box` (left-aligned), matching the pattern used by "Save changes" and "Send invite".

### D4 — Remove `PendingInvitesList`; pending rows stay in `MemberList`
**Decision:** Remove the `<PendingInvitesList>` from `OwnerManagementPage`. The `MemberList` component already renders a "Pending" subsection with node ID and expiry. Add a Withdraw action to the pending row in `MemberList` (currently missing) so the owner can still revoke from there. The `PendingInvitesList` component file can remain for Storybook but is no longer rendered.

**Why not remove `MemberList`'s pending section:** `MemberList` is the single source of truth for all member states; it already receives the pending rows. Removing it from `MemberList` would split state representation.

### D5 — Pre-validate node membership in `CreateFamilyEntry`
**Decision:** In `CreateFamilyEntry`, call `useFamilyMembership(nodeId)` for each `controlledNodeIds[0]`. If the membership query returns a `family_id`, render an `Alert severity="warning"` above the form ("Node {id} is already a member of family {name}. Leave that family before creating a new one.") and set `disabled` on the Create button.

Show the alert only when the query has settled (not loading) to avoid flash. This is a UI-only guard; the backend will still reject if somehow submitted.

### D6 — Page padding: remove horizontal padding from `FamilyPage`, keep vertical
**Decision:** Change `sx={{ p: 4 }}` to `sx={{ pt: 3, pb: 4 }}` (or match whatever `px` value the Balance/Bonding pages use — verify during implementation). The wallet's main content area already provides its own left/right gutter; the inner `p: 4` double-pads horizontally.

**Verification step:** During implementation, compare with `src/pages/balance/` or `src/pages/bonding/` to confirm the standard.

### D7 — NYM-1558: Auto-add owner node — sequential invite+accept after create
**Decision:** After `createFamily` resolves successfully, if `controlledNodeIds[0]` exists, immediately call `ctx.inviteToFamily({ node_id })` then `ctx.acceptFamilyInvitation({ family_id: newFamily.id, node_id })` in sequence. This uses the existing IPC calls; no backend changes required.

**UI change:** In `CreateFamilyForm`, if `ownerNodeId` prop is provided, add a helper line: "Your node {id} will be added automatically."

**Risk:** The invite+accept is two extra transactions. If `inviteToFamily` succeeds but `acceptFamilyInvitation` fails, the node will be in Pending, not Joined. Surface this to the user (snackbar error) and leave recovery to the user (they can accept from Node invites tab). Do not silently ignore.

### D8 — NYM-1559: Investigate + fix wrong-tab routing of invited node
**Decision:** The root cause is most likely that `useFamilyMemberList` maps the member status field incorrectly, OR that `refreshAll()` causes an optimistic cache write that puts the node in the wrong bucket before the server response arrives.

**Investigation path during implementation:**
1. Log the raw API response after `inviteToFamily` + `refreshAll()` in dev
2. Check the status value returned for a freshly-invited node in `useFamilyMemberList`
3. Verify the bucket mapping in the hook against the contract enum

Fix will be in the status mapping or by ensuring no optimistic update writes a wrong status.

### D9 — NYM-1560: Client-side truncation with expand toggle in `MemberList`
**Decision:** Add a `COLLAPSED_LIMIT = 3` constant. For the Removed and Rejected sections only, render the first 3 entries when collapsed and show a "See all ({n})" MUI `Button` (variant="text", size="small") when there are more. Pending and Joined sections are not truncated (they are actionable, so full visibility matters).

State: `const [rejectedExpanded, setRejectedExpanded] = useState(false)` and equivalent for Removed, local to `MemberList`. No persistence needed.

### D10 — Nav invite badge via standalone query hook, not FamiliesContext
**Problem:** `Nav` renders at `ApplicationLayout` level, which is *outside* the `FamiliesContextProvider` hierarchy. The families context provider only activates on the `/family` route. Calling `useFamiliesContext()` from Nav would throw.

**Decision:** Create a new hook `usePendingFamilyInviteCount()` in `src/hooks/` that:
1. Calls `useBondingContext()` to get the controlled node ID (bonding context is already available app-wide)
2. Directly calls `useOperatorNodeInvites(nodeId)` — this is a pure TanStack Query hook that does NOT require `FamiliesContext`, only the node ID
3. Returns `count: number` (number of non-expired pending invites for that node, 0 if no bonded node)

**Nav change:** In `Nav.tsx`, call `usePendingFamilyInviteCount()`. Wrap the `Family` nav item's `Icon` in a MUI `Badge` component with `variant="dot"` when `count > 0`. A dot (not a number) is sufficient — the exact count is visible inside the page.

**Why not lift FamiliesContextProvider to app level:** It wraps Tauri-specific IPC code. Moving it above the route boundary would cause it to initialise on every page load, including pages that have no families logic. The standalone hook is the minimal change.

**Why not use AppContext:** AppContext currently holds account/client details only. Adding families data there would couple unrelated concerns.

## Risks / Trade-offs

- **D2 (per-action loading) vs. context refactor** — Local state in `OwnerManagementPage` is correct but means the context `isExecuting` still gates delete/kick/revoke/leave buttons globally. That is the desired behaviour for those actions (they are destructive and should lock the whole page). The risk is that a future developer re-uses `ctx.isExecuting` for the edit/invite forms again. Mitigation: leave a comment in context explaining the distinction.

- **D7 (two-step invite+accept)** — Adds latency at family creation (two extra Tauri calls). Acceptable since creation is infrequent. The failure-in-the-middle state (node stuck in Pending) is visible and recoverable via the Node invites tab.

- **D5 (pre-validation)** — Relies on `useFamilyMembership` having resolved. If the query is slow or fails, the create form may be accessible when it shouldn't be. Mitigation: treat query loading as "unknown" and leave the button enabled (do not block on a loading indicator); the backend will reject anyway.

- **D8 (NYM-1559 investigation)** — Root cause is unknown until dev logging is added. If the bug is in the backend (contract returns wrong status), the fix falls outside FE scope and must be escalated.

## Open Questions

1. **D6 — exact page padding values**: Confirm what `px` the Balance/Bonding/Delegation pages use before writing the code.
2. **D7 — does `createFamily` return the new family's ID?** Verify the `FamilyTxResult` type includes the created family ID so we can pass it to `acceptFamilyInvitation`. If not, an extra query is needed.
3. **D3 — dual persona (owner + operator on same account)**: If the owner's account also controls a node that is in *their own* family, should "Current family" still render in My family tab? Likely yes — they might want to leave their own node. This edge case should be tested.
