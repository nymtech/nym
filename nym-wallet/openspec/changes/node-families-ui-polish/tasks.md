## 1. Foundation — verify page padding baseline

- [ ] 1.1 Check `src/pages/balance/`, `src/pages/bonding/`, and `src/pages/delegation/` for the `px` value they use on their top-level page Stack/Box to confirm the wallet-wide standard
- [ ] 1.2 In `FamilyPage.tsx`, change `sx={{ p: 4 }}` to match the wallet standard (likely `sx={{ pt: 3, pb: 4 }}` or remove horizontal padding), aligning cards with the Figma wireframe (node 1859-981)

## 2. Fix countdown timer (D1)

- [ ] 2.1 In `FamiliesContextProvider.tsx`, replace the `useMemo` for `nowSecs` with a `useState` initialized to `Math.floor(Date.now() / 1000)`
- [ ] 2.2 Add a `useEffect` that sets a 1-second `setInterval` updating `nowSecs` and returns a cleanup function that clears the interval on unmount
- [ ] 2.3 Verify that pending invite countdowns in `PendingInvitesList`, `MemberList`, and `InviteCard` all tick live in the running app

## 3. Fix per-action loading state (D2)

- [ ] 3.1 In `OwnerManagementPage.tsx`, add `const [editBusy, setEditBusy] = useState(false)` and `const [inviteBusy, setInviteBusy] = useState(false)`
- [ ] 3.2 Wrap `handleEdit` to set `editBusy = true` before the await and `false` in a finally block; same for `handleInvite` with `inviteBusy`
- [ ] 3.3 Pass `isSubmitting={editBusy}` to `EditFamilyForm` and `isSubmitting={inviteBusy}` to `InviteNodeForm` (remove `ctx.isExecuting` from these two)
- [ ] 3.4 Verify that clicking "Save changes" only shows "Saving…" on that button; "Send invite" stays idle, and vice versa

## 4. Move operator membership card to My family tab (D3)

- [ ] 4.1 In `OperatorInvitesPage.tsx`, remove the "Current family" `NymCard` block (lines 47–55) and the `LeaveFamilyButton` / `handleLeave` handler from `OperatorNodeSection`
- [ ] 4.2 Create an `OperatorMembershipCard` component (or inline in `OwnerManagementPage`) that calls `useFamilyMembership(nodeId)` for each controlled node; renders only when `family_id` is defined and the family data is loaded
- [ ] 4.3 Place the card between the family summary card and the Edit/Invite grid in `OwnerManagementPage`; wire `handleLeave` to `ctx.leaveFamily`
- [ ] 4.4 Fix `LeaveFamilyButton` (or its usage): remove full-width behavior — render the button inside a `Box` without `fullWidth`, matching the compact style of "Save changes" / "Send invite"

## 5. Remove duplicate pending invitations (D4)

- [ ] 5.1 In `OwnerManagementPage.tsx`, remove the `<PendingInvitesList>` render (lines 134–140); keep the `handleRevoke` handler
- [ ] 5.2 Add a Withdraw action to the Pending rows in `MemberList` — pass `onRevoke` prop and render a compact "Withdraw" button next to pending entries
- [ ] 5.3 Verify the Members card Pending section is the only place pending invites appear and still supports withdrawing

## 6. Pre-validate node-in-family before create (D5)

- [ ] 6.1 In `CreateFamilyEntry`, call `useFamilyMembership` for `controlledNodeIds[0]` (if it exists)
- [ ] 6.2 When the membership query has settled and returns a `family_id`, render an `Alert severity="warning"` above the form: "Node {id} is already a member of a family. Leave that family before creating a new one."
- [ ] 6.3 Disable the Create button when the blocking membership is detected; do not block while the query is still loading

## 7. NYM-1559 — Fix invited node appearing in wrong Members section

- [ ] 7.1 In dev mode, log the raw API response after `inviteToFamily` + `refreshAll()` completes; inspect the `status` field of the newly-invited node
- [ ] 7.2 Find the status-to-section mapping in `useFamilyMemberList` and cross-check it against the contract's member status enum values
- [ ] 7.3 Fix the mismatched mapping (or remove any optimistic update that writes a wrong status); confirm the invited node appears in Pending immediately after invite
- [ ] 7.4 Add a regression test: send invite, assert node is in `pending` section, assert it is not in `joined`, `rejected`, or `removed`

## 8. NYM-1558 — Auto-add owner's node at family creation

- [ ] 8.1 Verify that `FamilyTxResult` from `createFamily` includes the new family's ID; if not, add a query to fetch it by owner address after creation
- [ ] 8.2 In `CreateFamilyEntry.handleCreate`, after `createFamily` resolves, if `controlledNodeIds[0]` exists: call `ctx.inviteToFamily({ node_id })` then `ctx.acceptFamilyInvitation({ family_id: newFamilyId, node_id })` in sequence
- [ ] 8.3 If `inviteToFamily` or `acceptFamilyInvitation` fails, surface a snackbar error ("Family created but your node could not be added automatically — accept the invite from the Node invites tab") and do not swallow the error
- [ ] 8.4 In `CreateFamilyForm`, accept an optional `ownerNodeId` prop; when provided, render helper text: "Your node {id} will be added automatically."
- [ ] 8.5 Verify that after creation the owner's node appears in the Joined section without a manual invite step

## 9. NYM-1560 — "See all" truncation in MemberList

- [ ] 9.1 Add `const COLLAPSED_LIMIT = 3` constant in `MemberList.tsx`
- [ ] 9.2 Add `const [rejectedExpanded, setRejectedExpanded] = useState(false)` and `const [removedExpanded, setRemovedExpanded] = useState(false)` local state
- [ ] 9.3 For the Rejected section: render only the first 3 entries when `!rejectedExpanded`; show a MUI `Button` (variant="text", size="small") reading "See all ({n})" when there are more than 3; clicking it sets `rejectedExpanded = true`
- [ ] 9.4 Apply the same pattern for the Removed section
- [ ] 9.5 Pending and Joined sections are NOT truncated
- [ ] 9.6 Verify that expanding shows all entries and that the list never silently hides entries without the toggle

## 10. Tests and QA

- [ ] 10.1 Run `pnpm test --filter nym-wallet` and fix any failures introduced by the above changes
- [ ] 10.2 Update Storybook stories for `MemberList`, `EditFamilyForm`, `InviteNodeForm` if their props changed
- [ ] 10.3 Manually verify alignment against Figma wireframe (node 1859-981): cards should align to the wallet content edge with no extra horizontal gaps
- [ ] 10.4 Manually test the dual-persona edge case: owner account that controls a node already in their own family — "Current family" card renders in My family tab correctly
