/* eslint-disable @typescript-eslint/naming-convention, no-restricted-syntax, no-param-reassign */
import {
  AcceptFamilyInvitationArgs,
  CreateFamilyArgs,
  FamilyConfig,
  FamilyCursor,
  FamilyError,
  FamilyEvent,
  FamilyPagedResponse,
  FAMILY_PAGE_DEFAULT_LIMIT,
  FAMILY_PAGE_MAX_LIMIT,
  FamilyInvitation,
  InviteToFamilyArgs,
  KickFromFamilyArgs,
  LeaveFamilyArgs,
  NodeFamily,
  NodeFamilyId,
  NodeFamilyMembershipResponse,
  NodeId,
  PastFamilyInvitation,
  PastFamilyMember,
  PastInvitationStatusKind,
  PendingFamilyInvitationDetails,
  RejectFamilyInvitationArgs,
  RevokeFamilyInvitationArgs,
  UpdateFamilyArgs,
} from 'src/types/families';

/**
 * Pure, framework-free in-memory model of the `node-families-contract`.
 *
 * Mutators throw `FamilyError` and return the emitted events; queries are
 * read-only. The React mock provider and Jest drive this engine, which keeps the
 * contract logic in one testable place.
 */

interface BondedNode {
  owner: string;
  isUnbonding: boolean;
}

interface ArchivedInvitation {
  invitation: FamilyInvitation;
  status: { kind: PastInvitationStatusKind; at: number };
  seq: number;
}

interface ArchivedMember {
  record: PastFamilyMember;
  seq: number;
}

export interface MockStore {
  config: FamilyConfig;
  /** Controllable clock (unix seconds) used for expiry + timestamps. */
  nowSecs: number;
  /** Monotonic, never recycled, starts at 1. */
  nextFamilyId: NodeFamilyId;
  families: Map<NodeFamilyId, NodeFamily>;
  /** node_id -> membership (one family per node). */
  members: Map<NodeId, { family_id: NodeFamilyId; joined_at: number }>;
  /** `${family_id}:${node_id}` -> pending invitation. */
  pending: Map<string, FamilyInvitation>;
  pastInvitations: ArchivedInvitation[];
  pastMembers: ArchivedMember[];
  /** insertion order for archive cursors. */
  seq: number;
  /** Simulated mixnet bond table for node existence/control checks. */
  bondedNodes: Map<NodeId, BondedNode>;
}

const pendingKey = (familyId: NodeFamilyId, nodeId: NodeId) => `${familyId}:${nodeId}`;

const byteLen = (s: string): number => new TextEncoder().encode(s).length;

/** ASCII-only normalisation: lowercase ASCII letters, keep digits, drop everything else. */
export const normaliseFamilyName = (name: string): string => {
  let out = '';
  for (const ch of name) {
    if (ch >= 'A' && ch <= 'Z') out += ch.toLowerCase();
    else if (ch >= 'a' && ch <= 'z') out += ch;
    else if (ch >= '0' && ch <= '9') out += ch;
  }
  return out;
};

const event = (ty: FamilyEvent['ty'], attributes: Record<string, string | number>): FamilyEvent => ({
  ty,
  attributes: Object.fromEntries(Object.entries(attributes).map(([k, v]) => [k, String(v)])),
});

// ---------------------------------------------------------------------------
// Internal lookups
// ---------------------------------------------------------------------------

const findFamilyByOwner = (store: MockStore, owner: string): NodeFamily | undefined => {
  for (const fam of store.families.values()) if (fam.owner === owner) return fam;
  return undefined;
};

const findFamilyByNormalisedName = (store: MockStore, normalised: string): NodeFamily | undefined => {
  for (const fam of store.families.values()) if (fam.normalised_name === normalised) return fam;
  return undefined;
};

const requireOwnedFamily = (store: MockStore, sender: string): NodeFamily => {
  const fam = findFamilyByOwner(store, sender);
  if (!fam) throw new FamilyError('SenderDoesntOwnAFamily', 'You do not own a family', { address: sender });
  return fam;
};

const controlsNode = (store: MockStore, sender: string, nodeId: NodeId): boolean => {
  const bond = store.bondedNodes.get(nodeId);
  return Boolean(bond) && bond!.owner === sender && !bond!.isUnbonding;
};

const archiveInvitation = (store: MockStore, invitation: FamilyInvitation, kind: PastInvitationStatusKind) => {
  store.seq += 1;
  store.pastInvitations.push({ invitation, status: { kind, at: store.nowSecs }, seq: store.seq });
};

const archiveMember = (store: MockStore, familyId: NodeFamilyId, nodeId: NodeId) => {
  store.seq += 1;
  store.pastMembers.push({
    record: { family_id: familyId, node_id: nodeId, removed_at: store.nowSecs },
    seq: store.seq,
  });
};

const removeMembership = (store: MockStore, nodeId: NodeId, familyId: NodeFamilyId) => {
  store.members.delete(nodeId);
  const fam = store.families.get(familyId);
  if (fam && fam.members > 0) fam.members -= 1;
  archiveMember(store, familyId, nodeId);
};

// ---------------------------------------------------------------------------
// Execute mutators (throw FamilyError, return emitted events)
// ---------------------------------------------------------------------------

export function mockCreateFamily(store: MockStore, sender: string, args: CreateFamilyArgs): FamilyEvent[] {
  // fee check
  const fee = store.config.create_family_fee;
  if (args.fee.denom !== fee.denom) {
    throw new FamilyError('InvalidDeposit', `Expected fee in ${fee.denom}`, { expected: fee, received: args.fee });
  }
  if (args.fee.amount !== fee.amount) {
    throw new FamilyError('InvalidFamilyCreationFee', 'Incorrect creation fee', { expected: fee, received: args.fee });
  }
  // name length (bytes) + normalisation
  if (byteLen(args.name) > store.config.family_name_length_limit) {
    throw new FamilyError('FamilyNameTooLong', 'Family name too long', {
      length: byteLen(args.name),
      limit: store.config.family_name_length_limit,
    });
  }
  const normalised = normaliseFamilyName(args.name);
  if (normalised.length === 0) throw new FamilyError('EmptyFamilyName', 'Family name normalises to empty');
  // description length
  if (byteLen(args.description) > store.config.family_description_length_limit) {
    throw new FamilyError('FamilyDescriptionTooLong', 'Family description too long', {
      length: byteLen(args.description),
      limit: store.config.family_description_length_limit,
    });
  }
  // one family per owner
  const existing = findFamilyByOwner(store, sender);
  if (existing) {
    throw new FamilyError('SenderAlreadyOwnsAFamily', 'You already own a family', {
      address: sender,
      family_id: existing.id,
    });
  }
  // name uniqueness
  const taken = findFamilyByNormalisedName(store, normalised);
  if (taken) {
    throw new FamilyError('FamilyNameAlreadyTaken', 'Family name already taken', {
      name: normalised,
      family_id: taken.id,
    });
  }
  // sender's bonded node must not already be in a family
  for (const [nodeId, bond] of store.bondedNodes) {
    if (bond.owner === sender && store.members.has(nodeId)) {
      throw new FamilyError('AlreadyInFamily', 'Your node is already in a family', {
        address: sender,
        node_id: nodeId,
        family_id: store.members.get(nodeId)!.family_id,
      });
    }
  }

  const id = store.nextFamilyId;
  store.nextFamilyId += 1;
  const family: NodeFamily = {
    id,
    name: args.name,
    description: args.description,
    normalised_name: normalised,
    members: 0,
    created_at: store.nowSecs,
    paid_fee: args.fee,
    owner: sender,
  };
  store.families.set(id, family);
  return [
    event('family_creation', {
      family_name: args.name,
      owner_address: sender,
      family_id: id,
      paid_fee: `${args.fee.amount} ${args.fee.denom}`,
    }),
  ];
}

export function mockUpdateFamily(store: MockStore, sender: string, args: UpdateFamilyArgs): FamilyEvent[] {
  const setName = args.updated_name !== undefined && args.updated_name !== null;
  const setDesc = args.updated_description !== undefined && args.updated_description !== null;
  // no-op short-circuit BEFORE the ownership check
  if (!setName && !setDesc) return [];

  const fam = requireOwnedFamily(store, sender);

  if (setName) {
    const name = args.updated_name as string;
    if (byteLen(name) > store.config.family_name_length_limit) {
      throw new FamilyError('FamilyNameTooLong', 'Family name too long', {
        length: byteLen(name),
        limit: store.config.family_name_length_limit,
      });
    }
    const normalised = normaliseFamilyName(name);
    if (normalised.length === 0) throw new FamilyError('EmptyFamilyName', 'Family name normalises to empty');
    const clash = findFamilyByNormalisedName(store, normalised);
    if (clash && clash.id !== fam.id) {
      throw new FamilyError('FamilyNameAlreadyTaken', 'Family name already taken', {
        name: normalised,
        family_id: clash.id,
      });
    }
    fam.name = name;
    fam.normalised_name = normalised;
  }

  if (setDesc) {
    const desc = args.updated_description as string;
    if (byteLen(desc) > store.config.family_description_length_limit) {
      throw new FamilyError('FamilyDescriptionTooLong', 'Family description too long', {
        length: byteLen(desc),
        limit: store.config.family_description_length_limit,
      });
    }
    fam.description = desc;
  }

  const attributes: Record<string, string | number> = { family_id: fam.id, owner_address: sender };
  if (setName) attributes.updated_name = fam.name;
  if (setDesc) attributes.updated_description = fam.description;
  return [event('family_update', attributes)];
}

export function mockDisbandFamily(store: MockStore, sender: string): FamilyEvent[] {
  const fam = requireOwnedFamily(store, sender);
  if (fam.members > 0) {
    throw new FamilyError('FamilyNotEmpty', 'Family is not empty', { family_id: fam.id, members: fam.members });
  }
  // sweep still-pending invitations issued by this family -> Revoked
  for (const [key, invitation] of [...store.pending]) {
    if (invitation.family_id === fam.id) {
      archiveInvitation(store, invitation, 'Revoked');
      store.pending.delete(key);
    }
  }
  store.families.delete(fam.id);
  return [
    event('family_disband', {
      family_id: fam.id,
      owner_address: sender,
      refunded_fee: `${fam.paid_fee.amount} ${fam.paid_fee.denom}`,
    }),
  ];
}

export function mockInviteToFamily(store: MockStore, sender: string, args: InviteToFamilyArgs): FamilyEvent[] {
  const fam = requireOwnedFamily(store, sender);
  const validity = args.validity_secs ?? store.config.default_invitation_validity_secs;
  if (validity === 0) throw new FamilyError('ZeroInvitationValidity', 'Invitation validity must be positive');
  const bond = store.bondedNodes.get(args.node_id);
  if (!bond || bond.isUnbonding) {
    throw new FamilyError('NodeDoesntExist', 'Node does not exist or is unbonding', { node_id: args.node_id });
  }
  if (store.members.has(args.node_id)) {
    throw new FamilyError('NodeAlreadyInFamily', 'Node already in a family', {
      node_id: args.node_id,
      family_id: store.members.get(args.node_id)!.family_id,
    });
  }
  const key = pendingKey(fam.id, args.node_id);
  const existing = store.pending.get(key);
  if (existing) {
    if (store.nowSecs < existing.expires_at) {
      throw new FamilyError('PendingInvitationAlreadyExists', 'A pending invitation already exists', {
        family_id: fam.id,
        node_id: args.node_id,
      });
    }
    archiveInvitation(store, existing, 'Expired');
    store.pending.delete(key);
  }
  const expires_at = store.nowSecs + validity;
  store.pending.set(key, { family_id: fam.id, node_id: args.node_id, expires_at });
  return [event('family_invitation', { family_id: fam.id, node_id: args.node_id, expires_at })];
}

export function mockRevokeFamilyInvitation(
  store: MockStore,
  sender: string,
  args: RevokeFamilyInvitationArgs,
): FamilyEvent[] {
  const fam = requireOwnedFamily(store, sender);
  const key = pendingKey(fam.id, args.node_id);
  const invitation = store.pending.get(key);
  if (!invitation) {
    throw new FamilyError('InvitationNotFound', 'No pending invitation', { family_id: fam.id, node_id: args.node_id });
  }
  archiveInvitation(store, invitation, 'Revoked');
  store.pending.delete(key);
  return [event('family_invitation_revoked', { family_id: fam.id, node_id: args.node_id })];
}

export function mockKickFromFamily(store: MockStore, sender: string, args: KickFromFamilyArgs): FamilyEvent[] {
  const fam = requireOwnedFamily(store, sender);
  const membership = store.members.get(args.node_id);
  if (!membership) throw new FamilyError('NodeNotInFamily', 'Node is not in any family', { node_id: args.node_id });
  if (membership.family_id !== fam.id) {
    throw new FamilyError('NodeNotMemberOfFamily', 'Node is not a member of your family', {
      node_id: args.node_id,
      family_id: fam.id,
    });
  }
  removeMembership(store, args.node_id, fam.id);
  return [event('family_member_kicked', { family_id: fam.id, node_id: args.node_id })];
}

export function mockAcceptFamilyInvitation(
  store: MockStore,
  sender: string,
  args: AcceptFamilyInvitationArgs,
): FamilyEvent[] {
  if (!controlsNode(store, sender, args.node_id)) {
    throw new FamilyError('SenderDoesntControlNode', 'You do not control this node', {
      address: sender,
      node_id: args.node_id,
    });
  }
  const existing = store.members.get(args.node_id);
  if (existing) {
    throw new FamilyError('NodeAlreadyInFamily', 'Node already in a family', {
      node_id: args.node_id,
      family_id: existing.family_id,
    });
  }
  const key = pendingKey(args.family_id, args.node_id);
  const invitation = store.pending.get(key);
  if (!invitation) {
    throw new FamilyError('InvitationNotFound', 'No pending invitation', {
      family_id: args.family_id,
      node_id: args.node_id,
    });
  }
  if (store.nowSecs >= invitation.expires_at) {
    throw new FamilyError('InvitationExpired', 'Invitation has expired', {
      family_id: args.family_id,
      node_id: args.node_id,
      expires_at: invitation.expires_at,
      now: store.nowSecs,
    });
  }
  const fam = store.families.get(args.family_id);
  if (!fam) throw new FamilyError('FamilyNotFound', 'Family no longer exists', { family_id: args.family_id });

  store.pending.delete(key);
  store.members.set(args.node_id, { family_id: args.family_id, joined_at: store.nowSecs });
  fam.members += 1;
  archiveInvitation(store, invitation, 'Accepted');
  return [event('family_invitation_accepted', { family_id: args.family_id, node_id: args.node_id })];
}

export function mockRejectFamilyInvitation(
  store: MockStore,
  sender: string,
  args: RejectFamilyInvitationArgs,
): FamilyEvent[] {
  if (!controlsNode(store, sender, args.node_id)) {
    throw new FamilyError('SenderDoesntControlNode', 'You do not control this node', {
      address: sender,
      node_id: args.node_id,
    });
  }
  const key = pendingKey(args.family_id, args.node_id);
  const invitation = store.pending.get(key);
  if (!invitation) {
    throw new FamilyError('InvitationNotFound', 'No pending invitation', {
      family_id: args.family_id,
      node_id: args.node_id,
    });
  }
  archiveInvitation(store, invitation, 'Rejected');
  store.pending.delete(key);
  return [event('family_invitation_rejected', { family_id: args.family_id, node_id: args.node_id })];
}

export function mockLeaveFamily(store: MockStore, sender: string, args: LeaveFamilyArgs): FamilyEvent[] {
  if (!controlsNode(store, sender, args.node_id)) {
    throw new FamilyError('SenderDoesntControlNode', 'You do not control this node', {
      address: sender,
      node_id: args.node_id,
    });
  }
  const membership = store.members.get(args.node_id);
  if (!membership) throw new FamilyError('NodeNotInFamily', 'Node is not in any family', { node_id: args.node_id });
  removeMembership(store, args.node_id, membership.family_id);
  return [event('family_member_left', { family_id: membership.family_id, node_id: args.node_id })];
}

/** Test helper simulating the mixnet's unbond callback (no auth in the mock). */
export function mockOnNymNodeUnbond(store: MockStore, nodeId: NodeId): FamilyEvent[] {
  const membership = store.members.get(nodeId);
  if (membership) removeMembership(store, nodeId, membership.family_id);
  for (const [key, invitation] of [...store.pending]) {
    if (invitation.node_id === nodeId) {
      archiveInvitation(store, invitation, 'Rejected');
      store.pending.delete(key);
    }
  }
  return [event('family_node_unbond_cleanup', { node_id: nodeId })];
}

// ---------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------

function paginate<T>(
  all: { cursor: number; value: T }[],
  startAfter: FamilyCursor | undefined,
  limit?: number,
): FamilyPagedResponse<T> {
  const lim = Math.min(limit ?? FAMILY_PAGE_DEFAULT_LIMIT, FAMILY_PAGE_MAX_LIMIT);
  const after = typeof startAfter === 'number' ? startAfter : null;
  const sorted = [...all].sort((a, b) => a.cursor - b.cursor);
  const filtered = after == null ? sorted : sorted.filter((x) => x.cursor > after);
  const page = filtered.slice(0, lim);
  return {
    items: page.map((p) => p.value),
    start_next_after: page.length > 0 ? page[page.length - 1].cursor : null,
  };
}

export const mockGetFamilyConfig = (store: MockStore): FamilyConfig => store.config;

export const mockGetFamilyById = (store: MockStore, familyId: NodeFamilyId): NodeFamily | null =>
  store.families.get(familyId) ?? null;

export const mockGetFamilyByName = (store: MockStore, name: string): NodeFamily | null =>
  findFamilyByNormalisedName(store, normaliseFamilyName(name)) ?? null;

export const mockGetFamilyByOwner = (store: MockStore, owner: string): NodeFamily | null =>
  findFamilyByOwner(store, owner) ?? null;

export const mockGetFamilyMembership = (store: MockStore, nodeId: NodeId): NodeFamilyMembershipResponse => ({
  node_id: nodeId,
  family_id: store.members.get(nodeId)?.family_id ?? null,
});

export const mockGetFamilyMembersPaged = (
  store: MockStore,
  familyId: NodeFamilyId,
  startAfter?: FamilyCursor,
  limit?: number,
): FamilyPagedResponse<{ node_id: NodeId; joined_at: number }> => {
  const all = [...store.members.entries()]
    .filter(([, m]) => m.family_id === familyId)
    .map(([node_id, m]) => ({ cursor: node_id, value: { node_id, joined_at: m.joined_at } }));
  return paginate(all, startAfter, limit);
};

const withExpiry = (store: MockStore, invitation: FamilyInvitation): PendingFamilyInvitationDetails => ({
  invitation,
  expired: store.nowSecs >= invitation.expires_at,
});

export const mockGetPendingInvitationsForFamilyPaged = (
  store: MockStore,
  familyId: NodeFamilyId,
  startAfter?: FamilyCursor,
  limit?: number,
): FamilyPagedResponse<PendingFamilyInvitationDetails> => {
  const all = [...store.pending.values()]
    .filter((inv) => inv.family_id === familyId)
    .map((inv) => ({ cursor: inv.node_id, value: withExpiry(store, inv) }));
  return paginate(all, startAfter, limit);
};

export const mockGetPendingInvitationsForNodePaged = (
  store: MockStore,
  nodeId: NodeId,
  startAfter?: FamilyCursor,
  limit?: number,
): FamilyPagedResponse<PendingFamilyInvitationDetails> => {
  const all = [...store.pending.values()]
    .filter((inv) => inv.node_id === nodeId)
    .map((inv) => ({ cursor: inv.family_id, value: withExpiry(store, inv) }));
  return paginate(all, startAfter, limit);
};

export const mockGetPastInvitationsForFamilyPaged = (
  store: MockStore,
  familyId: NodeFamilyId,
  startAfter?: FamilyCursor,
  limit?: number,
): FamilyPagedResponse<PastFamilyInvitation> => {
  const all = store.pastInvitations
    .filter((a) => a.invitation.family_id === familyId)
    .map((a) => ({ cursor: a.seq, value: { invitation: a.invitation, status: a.status } }));
  return paginate(all, startAfter, limit);
};

export const mockGetPastMembersForFamilyPaged = (
  store: MockStore,
  familyId: NodeFamilyId,
  startAfter?: FamilyCursor,
  limit?: number,
): FamilyPagedResponse<PastFamilyMember> => {
  const all = store.pastMembers
    .filter((a) => a.record.family_id === familyId)
    .map((a) => ({ cursor: a.seq, value: a.record }));
  return paginate(all, startAfter, limit);
};
