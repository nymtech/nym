## Context

The Node Families feature (NYM-1199) was implemented on a feature branch and is approaching ship readiness. QA testing against Figma wireframes and three Jira acceptance criteria tickets (NYM-1558, NYM-1559, NYM-1560) surfaced a cluster of bugs and structural issues. All affected files are within `nym-wallet/src/` under the `families/` pages and `Families/` components directories.

Key constraints:
- Must not break the existing Tauri IPC layer (`familyRequests.*`)
- Tests at `src/components/Families/*.test.tsx` and `src/pages/families/*.test.tsx` must pass
- No new external dependencies; all fixes use existing React/MUI/TanStack Query patterns already in the codebase
- The `FamiliesContext` shape is consumed by Storybook/mock providers — any additions need defaults

## Goals / Non-Goals

**Goals:**
- Countdown timers tick live (1-second interval, cleaned up on unmount), via a shared `useNowSecs` hook
- Each family action's button reflects only its own in-flight operation (single `executingAction` discriminant)
- Operator membership (current family + Leave) surfaces in the My family tab, not the Invites tab
- Pending invitations appear exactly once (in the unified members table, not twice)
- Create form is hidden (replaced by the membership panel) when the owner's node is already in a family
- Page layout aligns with the wallet's standard `PageLayout`
- Owner's nym-node is auto-joined at family creation, handled atomically by the contract (NYM-1558)
- Freshly-invited node appears in Pending immediately, not in Joined/Rejected (NYM-1559)
- Members, invites and history are shown in one delegations-style table with joined/history de-duplication (supersedes NYM-1560 "See all")
- Family nav item and the Invites sub-tab show a count badge when the controlled node has active pending invitations

**Non-Goals:**
- Pagination or server-side truncation of member lists
- Design changes beyond what is specified in the Figma wireframe (node 1859-981)
- Changes to Tauri backend / CosmWasm contract behaviour
- Changes to the Bonding or Delegation pages

## Decisions

### D1 — Live `nowSecs` via a shared `useNowSecs` hook
**Decision (as built):** Extract the ticking clock into `useNowSecs` (`src/hooks/useNowSecs.ts`): a `useState` initialized to `Math.floor(Date.now() / 1000)` plus a `useEffect` running `setInterval(..., 1000)` with cleanup. The provider feeds its value out as `ctx.nowSecs`, so every countdown consumer shares one ticking source rather than the old `useMemo(() => ..., [])` that only computed once on mount.

**Why over alternatives:**
- *Prop drilling `Date.now()` from each display site* — duplicates timers, no single source of truth
- *`useReducer` tick* — unnecessarily complex for a scalar value
- The existing pattern already distributes `nowSecs` through context to all consumers; changing just the source is the minimal diff

### D2 — Per-action loading via a single `executingAction` discriminant on the context
**Decision (as built):** Replace the global `isExecuting` boolean with `executingAction: FamilyExecutingAction | null` on `FamiliesContext` (values like `create`, `invite`, `kick`, `revoke`, `leave`, `accept`, `reject`). Each button shows its loading label only when `ctx.executingAction` equals its own action, and is otherwise disabled (greyed) while a different action is in flight. The provider's `run(action, fn)` helper sets/clears the discriminant around each IPC call.

**Why a discriminant rather than two local `useState` flags (the earlier plan):** More than two actions share the owner surface (invite, kick, revoke, leave, plus create on the empty state). A single discriminant keeps them all consistent and avoids scattering busy flags across components. Note also that "Edit family" / "Save changes" was moved to the separate `FamilySettingsPage`, so the original "two forms side-by-side" framing no longer applies.

### D3 — Membership surfaced via `MyNodeFamilySection` in the My family tab
**Decision (as built):** The "Current family" block was removed from the Invites surface and replaced by `MyNodeFamilySection`, rendered through `ControlledNodeSections`. On the owner surface it sits in its own bordered `FamilyContentPanel` above the owned-family management panel; on the empty/create surface it stands in for the create form when the controlled node belongs to another wallet's family. This keeps an owned family visually distinct from membership in someone else's family.

**Leave button:** Rendered compact (not `fullWidth`) and labelled with the family name (e.g. "Leave {family}") so the user knows exactly which family they are leaving.

### D4 — Single `FamilyMembersTable` replaces `MemberList` + `PendingInvitesList`
**Decision (as built):** Both `MemberList` and the standalone `PendingInvitesList` were replaced by one delegations-style `FamilyMembersTable` (columns: Node, Status, Actions). It receives joined/rejected/removed rows from the member-list query and pending rows from the live invitations query, merges them in order (joined → pending → rejected → removed), and renders the row-appropriate action (Remove / Withdraw / Clear / none). Pending invitations therefore appear exactly once, with a Withdraw (or Clear, if expired) action inline.

### D5 — Pre-validate node membership in `CreateFamilyEntry` by hiding the form
**Decision (as built):** In `CreateFamilyEntry`, call `useFamilyMembership(controlledNodeIds[0])`. When it resolves with a `family_id`, do not render the create form at all — render `ControlledNodeSections` (`MyNodeFamilySection`, with its Leave action) instead, so the screen stays clean and the next step (leave the existing family) is obvious. `handleCreate` also short-circuits if `nodeInFamily`, as defence-in-depth. The check never blocks while the query is still loading.

**Why hide rather than warn+disable (the earlier plan):** A disabled form with an inline alert still shows create fields the user can't use. Hiding the fields and showing the existing-membership panel is cleaner and matches the "screens should always be clean" feedback.

### D6 — Page layout aligned with the wallet standard
**Decision (as built):** `FamilyPage` uses the shared `PageLayout` wrapper instead of an ad-hoc `p: 4`, so its cards align to the same content gutter as Balance/Bonding/Delegation and vertical spacing is preserved.

### D7 — NYM-1558: Auto-add owner node — done atomically in the contract
**Decision (as built):** The owner's bonded node enrolment was implemented in the node-families contract's `CreateFamily` handler (PR #6891), not in the wallet. When the sender controls a bonded, not-unbonding node that isn't already in a family, the contract writes its `FamilyMembership` and persists the family with `members = 1`. See the contract spec at `openspec/specs/node-families-contract/spec.md`.

**Why over the earlier FE invite+accept plan:** Doing it in the contract is atomic — there is no intermediate "stuck in Pending" state and no extra Tauri round-trips. The wallet just calls `createFamily`; the owner's node shows up in the Joined rows after the data refreshes. Consequently `CreateFamilyForm` has no "your node will be added automatically" hint and `handleCreate` does no follow-up invite/accept.

### D8 — NYM-1559: invited node routed to Pending (resolved)
**Decision (as built):** Pending rows are sourced from the live invitations query (`usePendingInvitationsForFamily`) rather than inferred from the member-list, and `deriveMemberSections` (`familyMemberSections.ts`) owns the status-to-section mapping for joined/rejected/removed. A freshly-invited node therefore appears in Pending immediately, and a node that is currently joined is dropped from the rejected/removed buckets so it is never duplicated in history.

### D9 — NYM-1560: superseded by the unified table + history de-duplication
**Decision (as built):** The planned "See all (N)" truncation of Removed/Rejected was dropped. Instead all records live in the single `FamilyMembersTable`, and `deriveMemberSections` removes any node from the rejected/removed buckets if it is currently joined. The clutter NYM-1560 targeted (stale history entries for nodes that came back) is handled by de-duplication rather than collapsing, so no expand/collapse state is needed.

### D10 — Nav + Invites sub-tab count badge via shared query, not FamiliesContext
**Problem:** `Nav` renders above the `/family` route, *outside* the `FamiliesContextProvider` hierarchy, so it can't call `useFamiliesContext()`.

**Decision (as built):**
1. `useControlledNodeIds()` (`src/hooks/useControlledNodeIds.ts`) resolves the controlled node ids app-wide without the Bonding/Families providers — it reads `AppContext` + `useGetNodeDetails` and returns `[]` until the bonded node resolves (so it degrades cleanly, including in the no-Tauri mock harness).
2. `usePendingInviteCountForNodes(nodeIds)` (`src/context/families.tsx`) sums **non-expired** pending invites across those nodes using `useQueries`, sharing the `pendingForNode` query cache so the badge and the invites view refresh together.
3. `InviteNotificationBadge` (`src/components/Families/InviteNotificationBadge.tsx`) is a styled mint MUI `Badge` that shows the **count** and hides itself at 0.

**Placements:** the sidebar "Family" entry in `Nav.tsx`, and the "Invites" sub-tab label in `FamilyPage.tsx`. We show the count (not a bare dot) because the user asked to see how many invites need addressing; expired invites are excluded since they can't be acted on.

**Why not lift FamiliesContextProvider to app level:** it wraps Tauri-specific IPC and would initialise on every page. The standalone hook is the minimal change.

## Risks / Trade-offs

- **D2 (`executingAction` discriminant)** — A single in-flight discriminant means starting one action greys out the others; this is intentional (avoids overlapping family transactions). Risk: a future developer reintroduces a global `isExecuting` boolean. Mitigation: the discriminant is documented on the context type.

- **D5 (pre-validation by hiding the form)** — Relies on `useFamilyMembership` having resolved; while loading, the create form remains visible. `handleCreate` short-circuits as defence-in-depth and the contract rejects anyway, so the worst case is a harmless no-op rather than a bad transaction.

- **D7 (contract-side auto-add)** — The wallet depends on the contract enrolling the owner's node; the owner's node only shows as Joined after the create tx settles and the queries refresh, so there may be a brief gap before it appears. No partial-failure window though, since enrolment is atomic in the create transaction.

## Open Questions

_None outstanding — D6 (layout), D7 (auto-add ownership of the change) and D3 (dual persona) were resolved during implementation: layout uses the shared `PageLayout`, auto-add is owned by the contract, and the dual-persona case renders the membership panel above the owned-family panel._
