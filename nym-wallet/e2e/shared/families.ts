/**
 * Shared journey constants for the Node Families e2e suites (parity requirement,
 * design D3). Both the primary Playwright suite (`e2e/families.spec.ts`) and the
 * optional WebdriverIO native leg (`e2e-tauri/families.tauri.ts`) import these.
 *
 * These are the test ids that render reliably in the DOM. We scope by the `operator-node-<n>`
 * Stack wrapper / table rows and key invite buttons by family id (the ids emitted by
 * `ConfirmActionButton`, e.g. `invite-card-<familyId>-accept`). Historically `NymCard` dropped
 * `data-testid` (its prop was `dataTestid`), so `node-invite-group-<n>` / `invite-card-<fid>`
 * didn't render and the original Storybook play-function selectors were invalid; that was fixed
 * in the `fix-nymcard-data-testid` change. These selectors remain valid and are kept as-is.
 */

export type FamilyPersona = 'owner' | 'operator' | 'operator-seeded';

/** Fixture node ids (mirror src/context/mocks/families.fixtures.ts). */
export const FAMILY_NODES = {
  ownerFlow: 301, // MOCK_OWNER_FLOW_NODE
  operatorAccept: 201, // MOCK_OPERATOR_FLOW_ACCEPT_NODE / MOCK_OPERATOR_NODE_ACTIVE
  operatorReject: 204, // MOCK_OPERATOR_FLOW_REJECT_NODE
  operatorNone: 203, // MOCK_OPERATOR_NODE_NONE
} as const;

/**
 * The family_id that issued each persona's invites — invite-card test ids are keyed by
 * family_id, not node id. Owner-flow + operator-flow each have a single family (id 1);
 * the seeded store's operator invites come from the second family (id 2).
 */
export const FAMILY_IDS = {
  ownerFlow: 1,
  operatorFlow: 1,
  seeded: 2,
} as const;

/** Route into the mock-wired app shell for a given persona (see PERSONAS in src/main.mock.tsx). */
export const familyMockUrl = (persona: FamilyPersona) => `/main.mock.html?persona=${persona}#/family`;

/** Test ids that actually render (parameterised by node id or family id where noted). */
export const TID = {
  // owner: create + manage
  createFamilyName: 'create-family-name',
  createFamilyDescription: 'create-family-description',
  createFamilySubmit: 'create-family-submit',
  ownerManagementPage: 'owner-management-page',
  familySettingsButton: 'family-settings-button',
  familySettingsPage: 'family-settings-page',
  inviteNodeId: 'invite-node-id',
  inviteNodeSubmit: 'invite-node-submit',
  inviteNodeConfirm: 'invite-node-confirm',
  deleteButton: 'delete-family-button',
  deleteConfirm: 'delete-family-button-confirm',
  pendingInvite: (node: number) => `pending-invite-${node}`,
  memberJoined: (node: number) => `member-joined-${node}`,
  memberJoinedKick: (node: number) => `member-joined-${node}-kick`,
  memberJoinedKickConfirm: (node: number) => `member-joined-${node}-kick-confirm`,
  myNodeFamily: (node: number) => `my-node-family-${node}`,
  // tabs
  tabOwner: 'family-tab-owner',
  tabOperator: 'family-tab-operator',
  // operator: per-node section wrapper (Stack — renders) used for scoping
  operatorNodeSection: (node: number) => `operator-node-${node}`,
  inviteGroupEmpty: (node: number) => `node-invite-group-${node}-empty`,
  inviteGroupMember: (node: number) => `node-invite-group-${node}-member`,
  leaveButton: 'leave-family-button',
  leaveConfirm: 'leave-family-button-confirm',
  // invite cards: keyed by FAMILY id (ConfirmActionButton dataTestid — renders)
  acceptCard: (familyId: number) => `invite-card-${familyId}-accept`,
  acceptConfirm: (familyId: number) => `invite-card-${familyId}-accept-confirm`,
  rejectCard: (familyId: number) => `invite-card-${familyId}-reject`,
  rejectConfirm: (familyId: number) => `invite-card-${familyId}-reject-confirm`,
} as const;
