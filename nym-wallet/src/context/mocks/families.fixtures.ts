/* eslint-disable @typescript-eslint/naming-convention, no-restricted-syntax, no-param-reassign */
import type { FamilyConfig, NodeId } from 'src/types/families';
import {
  MockStore,
  mockAcceptFamilyInvitation,
  mockCreateFamily,
  mockInviteToFamily,
  mockKickFromFamily,
  mockLeaveFamily,
  mockRejectFamilyInvitation,
  mockRevokeFamilyInvitation,
} from './familiesMockState';

/** Stable clock for deterministic fixtures (unix seconds). */
export const MOCK_NOW_SECS = 1_700_000_000;

export const FAMILY_FIXTURE_CONFIG: FamilyConfig = {
  create_family_fee: { denom: 'nym', amount: '100' },
  family_name_length_limit: 30,
  family_description_length_limit: 120,
  default_invitation_validity_secs: 3600,
};

// Personas / addresses ------------------------------------------------------
export const MOCK_OWNER_ADDRESS = 'n1owner000000000000000000000000000000owner';
export const MOCK_OPERATOR_ADDRESS = 'n1operator00000000000000000000000operator';
export const MOCK_OTHER_OWNER_ADDRESS = 'n1alpine0000000000000000000000000000alpine';

export const MOCK_OWNER_FAMILY_NAME = 'Tatry Operators';
export const MOCK_OTHER_FAMILY_NAME = 'Alpine Routers';

// Node ids ------------------------------------------------------------------
/** Operator's controlled nodes: active invite / expired invite / no invite. */
export const MOCK_OPERATOR_NODE_ACTIVE: NodeId = 201;
export const MOCK_OPERATOR_NODE_EXPIRED: NodeId = 202;
export const MOCK_OPERATOR_NODE_NONE: NodeId = 203;
export const MOCK_OPERATOR_NODE_IDS = [MOCK_OPERATOR_NODE_ACTIVE, MOCK_OPERATOR_NODE_EXPIRED, MOCK_OPERATOR_NODE_NONE];

export const createEmptyStore = (now: number = MOCK_NOW_SECS): MockStore => ({
  config: { ...FAMILY_FIXTURE_CONFIG },
  nowSecs: now,
  nextFamilyId: 1,
  families: new Map(),
  members: new Map(),
  pending: new Map(),
  pastInvitations: [],
  pastMembers: [],
  seq: 0,
  bondedNodes: new Map(),
});

const bond = (store: MockStore, nodeId: NodeId, owner: string) =>
  store.bondedNodes.set(nodeId, { owner, isUnbonding: false });

/**
 * Build a richly-seeded store exercising the full owner + operator surface:
 * - an owned family (#1) with Joined members, Removed (kicked + left), past
 *   Rejected and Revoked invitations, and pending invites (one active, one expired);
 * - a second family (#2, other owner) inviting the operator's nodes so the
 *   operator persona sees active / expired / no-invite states across its nodes.
 */
export const buildSeededStore = (): MockStore => {
  const s = createEmptyStore(MOCK_NOW_SECS);
  const fee = s.config.create_family_fee;

  // bonded nodes the owner family will act on (each controlled by its own address)
  const ctrl = (n: NodeId) => `n1ctrl${n}000000000000000000000000000ctrl`;
  for (const n of [101, 102, 103, 104, 105, 106, 107, 108]) bond(s, n, ctrl(n));
  // operator's three nodes
  for (const n of MOCK_OPERATOR_NODE_IDS) bond(s, n, MOCK_OPERATOR_ADDRESS);

  // --- Family #1, owned by MOCK_OWNER_ADDRESS ---
  mockCreateFamily(s, MOCK_OWNER_ADDRESS, {
    name: MOCK_OWNER_FAMILY_NAME,
    description: 'Operators coordinating routing in the Tatra mountains.',
    fee,
  });

  // Joined members (101, 102)
  mockInviteToFamily(s, MOCK_OWNER_ADDRESS, { node_id: 101 });
  mockAcceptFamilyInvitation(s, ctrl(101), { family_id: 1, node_id: 101 });
  mockInviteToFamily(s, MOCK_OWNER_ADDRESS, { node_id: 102 });
  mockAcceptFamilyInvitation(s, ctrl(102), { family_id: 1, node_id: 102 });

  // Removed, kicked (103)
  mockInviteToFamily(s, MOCK_OWNER_ADDRESS, { node_id: 103 });
  mockAcceptFamilyInvitation(s, ctrl(103), { family_id: 1, node_id: 103 });
  mockKickFromFamily(s, MOCK_OWNER_ADDRESS, { node_id: 103 });

  // Removed, left (104)
  mockInviteToFamily(s, MOCK_OWNER_ADDRESS, { node_id: 104 });
  mockAcceptFamilyInvitation(s, ctrl(104), { family_id: 1, node_id: 104 });
  mockLeaveFamily(s, ctrl(104), { node_id: 104 });

  // Past invitation, Rejected (105)
  mockInviteToFamily(s, MOCK_OWNER_ADDRESS, { node_id: 105 });
  mockRejectFamilyInvitation(s, ctrl(105), { family_id: 1, node_id: 105 });

  // Past invitation, Revoked (106)
  mockInviteToFamily(s, MOCK_OWNER_ADDRESS, { node_id: 106 });
  mockRevokeFamilyInvitation(s, MOCK_OWNER_ADDRESS, { node_id: 106 });

  // Pending, active (107) and expired (108)
  mockInviteToFamily(s, MOCK_OWNER_ADDRESS, { node_id: 107 }); // expires NOW + 3600 (active)
  mockInviteToFamily(s, MOCK_OWNER_ADDRESS, { node_id: 108 });
  s.pending.get('1:108')!.expires_at = s.nowSecs - 1; // force expired

  // --- Family #2, owned by MOCK_OTHER_OWNER_ADDRESS, invites the operator's nodes ---
  mockCreateFamily(s, MOCK_OTHER_OWNER_ADDRESS, {
    name: MOCK_OTHER_FAMILY_NAME,
    description: 'Alpine routing collective.',
    fee,
  });
  mockInviteToFamily(s, MOCK_OTHER_OWNER_ADDRESS, { node_id: MOCK_OPERATOR_NODE_ACTIVE }); // active
  mockInviteToFamily(s, MOCK_OTHER_OWNER_ADDRESS, { node_id: MOCK_OPERATOR_NODE_EXPIRED });
  s.pending.get(`2:${MOCK_OPERATOR_NODE_EXPIRED}`)!.expires_at = s.nowSecs - 1; // force expired
  // MOCK_OPERATOR_NODE_NONE: intentionally no invitation

  return s;
};

/** Node the owner-flow account both owns (the family) and controls (so one sender can drive create→invite→accept→kick→disband). */
export const MOCK_OWNER_FLOW_NODE: NodeId = 301;

/** Empty store for the owner flow story: the owner account also controls node 301. */
export const buildOwnerFlowStore = (): MockStore => {
  const s = createEmptyStore(MOCK_NOW_SECS);
  bond(s, MOCK_OWNER_FLOW_NODE, MOCK_OWNER_ADDRESS);
  return s;
};

/** Operator-flow nodes: one to accept-then-leave, one to reject. */
export const MOCK_OPERATOR_FLOW_ACCEPT_NODE: NodeId = 201;
export const MOCK_OPERATOR_FLOW_REJECT_NODE: NodeId = 204;

/** Store for the operator flow story: two active invites addressed to the operator's nodes. */
export const buildOperatorFlowStore = (): MockStore => {
  const s = createEmptyStore(MOCK_NOW_SECS);
  bond(s, MOCK_OPERATOR_FLOW_ACCEPT_NODE, MOCK_OPERATOR_ADDRESS);
  bond(s, MOCK_OPERATOR_FLOW_REJECT_NODE, MOCK_OPERATOR_ADDRESS);
  mockCreateFamily(s, MOCK_OTHER_OWNER_ADDRESS, {
    name: MOCK_OTHER_FAMILY_NAME,
    description: 'Alpine routing collective.',
    fee: s.config.create_family_fee,
  });
  mockInviteToFamily(s, MOCK_OTHER_OWNER_ADDRESS, { node_id: MOCK_OPERATOR_FLOW_ACCEPT_NODE });
  mockInviteToFamily(s, MOCK_OTHER_OWNER_ADDRESS, { node_id: MOCK_OPERATOR_FLOW_REJECT_NODE });
  return s;
};
