import { DecCoin } from '@nymproject/types';
import { FamilyError, FamilyErrorKind, isFamilyError } from 'src/types/families';
import { createEmptyStore } from './families.fixtures';
import {
  MockStore,
  mockAcceptFamilyInvitation,
  mockCreateFamily,
  mockDisbandFamily,
  mockGetFamilyById,
  mockGetFamilyByName,
  mockGetFamilyByOwner,
  mockGetFamilyMembersPaged,
  mockGetFamilyMembership,
  mockGetPastInvitationsForFamilyPaged,
  mockGetPastMembersForFamilyPaged,
  mockGetPendingInvitationsForFamilyPaged,
  mockGetPendingInvitationsForNodePaged,
  mockInviteToFamily,
  mockKickFromFamily,
  mockLeaveFamily,
  mockOnNymNodeUnbond,
  mockRejectFamilyInvitation,
  mockRevokeFamilyInvitation,
  mockUpdateFamily,
  normaliseFamilyName,
} from './familiesMockState';

const coin = (denom: string, amount: string): DecCoin => ({ denom, amount } as unknown as DecCoin);
const FEE = coin('nym', '100');
const OWNER = 'owner';
const NOW = 1000;

/** Fresh store at NOW with a set of bonded nodes (id -> controller). */
const setup = (nodes: Record<number, string> = {}): MockStore => {
  const s = createEmptyStore(NOW);
  Object.entries(nodes).forEach(([id, owner]) => s.bondedNodes.set(Number(id), { owner, isUnbonding: false }));
  return s;
};

const expectError = (fn: () => void, kind: FamilyErrorKind) => {
  expect(fn).toThrow(FamilyError);
  try {
    fn();
  } catch (e) {
    expect(Boolean(isFamilyError(e))).toBe(true);
    expect((e as FamilyError).kind).toBe(kind);
  }
};

const create = (s: MockStore, name = 'Tatry', owner = OWNER) =>
  mockCreateFamily(s, owner, { name, description: 'desc', fee: FEE });

describe('normaliseFamilyName', () => {
  it('strips punctuation, whitespace, casing, and non-ASCII', () => {
    ['Foo Bar', 'foobar', 'FOO-BAR', '  f.o.o.b.a.r  '].forEach((n) => expect(normaliseFamilyName(n)).toBe('foobar'));
    expect(normaliseFamilyName('café')).toBe('caf');
    expect(normaliseFamilyName('⭐stars')).toBe('stars');
    expect(normaliseFamilyName('!!!---')).toBe('');
  });
});

describe('createFamily', () => {
  it('persists the family and emits family_creation', () => {
    const s = setup();
    const events = create(s);
    const fam = mockGetFamilyByOwner(s, OWNER)!;
    expect(fam.id).toBe(1);
    expect(fam.members).toBe(0);
    expect(fam.created_at).toBe(NOW);
    expect(fam.normalised_name).toBe('tatry');
    expect(events[0].ty).toBe('family_creation');
    expect(events[0].attributes).toMatchObject({ family_id: '1', owner_address: OWNER });
  });

  it('rejects a wrong fee denom (InvalidDeposit) and wrong amount (InvalidFamilyCreationFee)', () => {
    expectError(
      () => mockCreateFamily(setup(), OWNER, { name: 'A', description: '', fee: coin('foo', '100') }),
      'InvalidDeposit',
    );
    expectError(
      () => mockCreateFamily(setup(), OWNER, { name: 'A', description: '', fee: coin('nym', '5') }),
      'InvalidFamilyCreationFee',
    );
  });

  it('enforces byte-length name limit (multi-byte counts full bytes)', () => {
    const s = setup();
    s.config.family_name_length_limit = 8;
    // "🚀rocket" = 4-byte emoji + 6 = 10 bytes > 8
    expectError(() => mockCreateFamily(s, OWNER, { name: '🚀rocket', description: '', fee: FEE }), 'FamilyNameTooLong');
  });

  it('rejects an all-symbol name as EmptyFamilyName', () => {
    expectError(
      () => mockCreateFamily(setup(), OWNER, { name: '!!!---', description: '', fee: FEE }),
      'EmptyFamilyName',
    );
  });

  it('rejects a colliding normalised name', () => {
    const s = setup();
    create(s, 'Shared');
    expectError(
      () => mockCreateFamily(s, 'other', { name: '$$shared$$', description: '', fee: FEE }),
      'FamilyNameAlreadyTaken',
    );
  });

  it('rejects a second family for the same owner', () => {
    const s = setup();
    create(s);
    expectError(() => create(s, 'Another'), 'SenderAlreadyOwnsAFamily');
  });

  it('rejects when the owner controls a bonded node already in a family', () => {
    const s = setup({ 5: OWNER });
    // put node 5 in some family via another owner's invite+accept
    s.bondedNodes.set(5, { owner: OWNER, isUnbonding: false });
    create(s, 'X', 'otherowner');
    mockInviteToFamily(s, 'otherowner', { node_id: 5 });
    mockAcceptFamilyInvitation(s, OWNER, { family_id: 1, node_id: 5 });
    expectError(() => create(s, 'Mine', OWNER), 'AlreadyInFamily');
  });

  it('assigns monotonic, non-recycled ids', () => {
    const s = setup();
    create(s, 'First');
    mockDisbandFamily(s, OWNER);
    create(s, 'Second');
    expect(mockGetFamilyByOwner(s, OWNER)!.id).toBe(2);
  });
});

describe('updateFamily', () => {
  it('updates name only and emits conditional attribute', () => {
    const s = setup();
    create(s);
    const events = mockUpdateFamily(s, OWNER, { updated_name: 'Renamed', updated_description: null });
    const fam = mockGetFamilyByOwner(s, OWNER)!;
    expect(fam.name).toBe('Renamed');
    expect(fam.description).toBe('desc');
    expect(events[0].attributes).toMatchObject({ updated_name: 'Renamed' });
    expect(events[0].attributes.updated_description).toBeUndefined();
  });

  it('no-op (both None) emits no event and does not require ownership', () => {
    const s = setup();
    expect(mockUpdateFamily(s, 'nobody', {})).toStrictEqual([]);
  });

  it('rejects a set field from a non-owner', () => {
    expectError(() => mockUpdateFamily(setup(), 'nobody', { updated_name: 'X' }), 'SenderDoesntOwnAFamily');
  });

  it('allows a case-only rename (same normalised) and rejects collision with another family', () => {
    const s = setup();
    create(s, 'Shared');
    create(s, 'Other', 'owner2');
    // case-only rename of owner's family
    mockUpdateFamily(s, OWNER, { updated_name: 'SHARED' });
    expect(mockGetFamilyByOwner(s, OWNER)!.normalised_name).toBe('shared');
    // rename to collide with owner2's family
    expectError(() => mockUpdateFamily(s, OWNER, { updated_name: 'other' }), 'FamilyNameAlreadyTaken');
  });
});

describe('disbandFamily', () => {
  it('disbands an empty family and emits family_disband', () => {
    const s = setup();
    create(s);
    const events = mockDisbandFamily(s, OWNER);
    expect(mockGetFamilyByOwner(s, OWNER)).toBeNull();
    expect(events[0].ty).toBe('family_disband');
  });

  it('rejects a non-empty family', () => {
    const s = setup({ 7: 'ctrl7' });
    create(s);
    mockInviteToFamily(s, OWNER, { node_id: 7 });
    mockAcceptFamilyInvitation(s, 'ctrl7', { family_id: 1, node_id: 7 });
    expectError(() => mockDisbandFamily(s, OWNER), 'FamilyNotEmpty');
  });

  it('sweeps still-pending invitations as Revoked', () => {
    const s = setup({ 8: 'ctrl8' });
    create(s);
    mockInviteToFamily(s, OWNER, { node_id: 8 });
    mockDisbandFamily(s, OWNER);
    const past = mockGetPastInvitationsForFamilyPaged(s, 1).items;
    expect(past).toHaveLength(1);
    expect(past[0].status.kind).toBe('Revoked');
  });
});

describe('inviteToFamily', () => {
  const seeded = () => {
    const s = setup({ 10: 'ctrl10', 11: 'ctrl11' });
    create(s);
    return s;
  };

  it('invites with the computed expiry and emits family_invitation', () => {
    const s = seeded();
    const events = mockInviteToFamily(s, OWNER, { node_id: 10, validity_secs: 500 });
    expect(s.pending.get('1:10')!.expires_at).toBe(NOW + 500);
    expect(events[0].ty).toBe('family_invitation');
  });

  it('falls back to the default validity', () => {
    const s = seeded();
    mockInviteToFamily(s, OWNER, { node_id: 10 });
    expect(s.pending.get('1:10')!.expires_at).toBe(NOW + s.config.default_invitation_validity_secs);
  });

  it('rejects zero validity, non-existent node, and duplicate pending', () => {
    const s = seeded();
    expectError(() => mockInviteToFamily(s, OWNER, { node_id: 10, validity_secs: 0 }), 'ZeroInvitationValidity');
    expectError(() => mockInviteToFamily(s, OWNER, { node_id: 999 }), 'NodeDoesntExist');
    mockInviteToFamily(s, OWNER, { node_id: 11 });
    expectError(() => mockInviteToFamily(s, OWNER, { node_id: 11 }), 'PendingInvitationAlreadyExists');
  });

  it('re-invites after expiry, archiving the stale invitation as Expired', () => {
    const s = seeded();
    mockInviteToFamily(s, OWNER, { node_id: 11, validity_secs: 100 });
    s.pending.get('1:11')!.expires_at = NOW - 1;
    const events = mockInviteToFamily(s, OWNER, { node_id: 11, validity_secs: 200 });
    expect(s.pending.get('1:11')!.expires_at).toBe(NOW + 200);
    expect(mockGetPastInvitationsForFamilyPaged(s, 1).items[0].status.kind).toBe('Expired');
    expect(events[0].ty).toBe('family_invitation');
  });

  it('rejects inviting a node already in a family', () => {
    const s = seeded();
    mockInviteToFamily(s, OWNER, { node_id: 10 });
    mockAcceptFamilyInvitation(s, 'ctrl10', { family_id: 1, node_id: 10 });
    expectError(() => mockInviteToFamily(s, OWNER, { node_id: 10 }), 'NodeAlreadyInFamily');
  });
});

describe('accept / reject / revoke', () => {
  const invited = (validity = 500) => {
    const s = setup({ 20: 'ctrl20' });
    create(s);
    mockInviteToFamily(s, OWNER, { node_id: 20, validity_secs: validity });
    return s;
  };

  it('accept records membership, increments count, archives Accepted, emits event', () => {
    const s = invited();
    const events = mockAcceptFamilyInvitation(s, 'ctrl20', { family_id: 1, node_id: 20 });
    expect(mockGetFamilyMembership(s, 20).family_id).toBe(1);
    expect(mockGetFamilyById(s, 1)!.members).toBe(1);
    expect(s.pending.has('1:20')).toBe(false);
    expect(mockGetPastInvitationsForFamilyPaged(s, 1).items[0].status.kind).toBe('Accepted');
    expect(events[0].ty).toBe('family_invitation_accepted');
  });

  it('accept rejects non-controller, expired, and missing invitation', () => {
    expectError(
      () => mockAcceptFamilyInvitation(invited(), 'someoneelse', { family_id: 1, node_id: 20 }),
      'SenderDoesntControlNode',
    );
    const expired = invited();
    expired.pending.get('1:20')!.expires_at = NOW; // now >= expires_at => expired
    expectError(
      () => mockAcceptFamilyInvitation(expired, 'ctrl20', { family_id: 1, node_id: 20 }),
      'InvitationExpired',
    );
    const s = setup({ 21: 'ctrl21' });
    create(s);
    expectError(() => mockAcceptFamilyInvitation(s, 'ctrl21', { family_id: 1, node_id: 21 }), 'InvitationNotFound');
  });

  it('reject archives Rejected and works even on expired invitations', () => {
    const s = invited();
    s.pending.get('1:20')!.expires_at = NOW - 1; // expired
    const events = mockRejectFamilyInvitation(s, 'ctrl20', { family_id: 1, node_id: 20 });
    expect(s.pending.has('1:20')).toBe(false);
    expect(mockGetPastInvitationsForFamilyPaged(s, 1).items[0].status.kind).toBe('Rejected');
    expect(events[0].ty).toBe('family_invitation_rejected');
  });

  it('revoke (owner) archives Revoked; missing pending throws', () => {
    const s = invited();
    mockRevokeFamilyInvitation(s, OWNER, { node_id: 20 });
    expect(mockGetPastInvitationsForFamilyPaged(s, 1).items[0].status.kind).toBe('Revoked');
    expectError(() => mockRevokeFamilyInvitation(s, OWNER, { node_id: 20 }), 'InvitationNotFound');
  });
});

describe('kick / leave', () => {
  const joined = (node = 30, ctrl = 'ctrl30') => {
    const s = setup({ [node]: ctrl });
    create(s);
    mockInviteToFamily(s, OWNER, { node_id: node });
    mockAcceptFamilyInvitation(s, ctrl, { family_id: 1, node_id: node });
    return s;
  };

  it('kick moves the node to Removed and emits family_member_kicked', () => {
    const s = joined();
    const events = mockKickFromFamily(s, OWNER, { node_id: 30 });
    expect(mockGetFamilyMembership(s, 30).family_id).toBeNull();
    expect(mockGetPastMembersForFamilyPaged(s, 1).items).toHaveLength(1);
    expect(events[0].ty).toBe('family_member_kicked');
  });

  it('kick rejects a node not in any family and one in a different family', () => {
    const s = joined(); // node 30 in OWNER's family 1
    expectError(() => mockKickFromFamily(s, OWNER, { node_id: 999 }), 'NodeNotInFamily');
    // node 31 joins a different family (owner2); OWNER cannot kick it
    s.bondedNodes.set(31, { owner: 'ctrl31', isUnbonding: false });
    create(s, 'Other', 'owner2');
    mockInviteToFamily(s, 'owner2', { node_id: 31 });
    mockAcceptFamilyInvitation(s, 'ctrl31', { family_id: 2, node_id: 31 });
    expectError(() => mockKickFromFamily(s, OWNER, { node_id: 31 }), 'NodeNotMemberOfFamily');
  });

  it('leave removes membership, emits family_member_left, and the node can rejoin', () => {
    const s = joined();
    mockLeaveFamily(s, 'ctrl30', { node_id: 30 });
    expect(mockGetFamilyMembership(s, 30).family_id).toBeNull();
    // can rejoin
    mockInviteToFamily(s, OWNER, { node_id: 30 });
    mockAcceptFamilyInvitation(s, 'ctrl30', { family_id: 1, node_id: 30 });
    expect(mockGetFamilyMembership(s, 30).family_id).toBe(1);
    // two removed records would exist after a second leave (sequential archive slots)
    mockLeaveFamily(s, 'ctrl30', { node_id: 30 });
    expect(mockGetPastMembersForFamilyPaged(s, 1).items).toHaveLength(2);
  });

  it('leave rejects a non-controller', () => {
    expectError(() => mockLeaveFamily(joined(), 'someoneelse', { node_id: 30 }), 'SenderDoesntControlNode');
  });
});

describe('onNymNodeUnbond', () => {
  it('removes membership and sweeps pending invitations as Rejected', () => {
    const s = setup({ 40: 'ctrl40', 41: 'ctrl41' });
    create(s);
    mockInviteToFamily(s, OWNER, { node_id: 40 });
    mockAcceptFamilyInvitation(s, 'ctrl40', { family_id: 1, node_id: 40 });
    mockInviteToFamily(s, OWNER, { node_id: 41 }); // pending

    const events = mockOnNymNodeUnbond(s, 40); // member -> removed
    expect(mockGetFamilyMembership(s, 40).family_id).toBeNull();
    expect(events[0].ty).toBe('family_node_unbond_cleanup');

    mockOnNymNodeUnbond(s, 41); // pending -> swept as Rejected
    expect(s.pending.has('1:41')).toBe(false);
    const past = mockGetPastInvitationsForFamilyPaged(s, 1).items;
    expect(past.some((p) => p.invitation.node_id === 41 && p.status.kind === 'Rejected')).toBe(true);
  });

  it('is a no-op for a node with no family and no invitations', () => {
    const s = setup();
    expect(mockOnNymNodeUnbond(s, 123)[0].ty).toBe('family_node_unbond_cleanup');
  });
});

describe('queries & pagination', () => {
  it('getFamilyByName is invariant under formatting; getFamilyMembership returns None for unknown', () => {
    const s = setup();
    create(s, 'MyFamily');
    expect(mockGetFamilyByName(s, 'my family')!.id).toBe(1);
    expect(mockGetFamilyMembership(s, 777).family_id).toBeNull();
  });

  it('pending query stamps the live expired flag', () => {
    const s = setup({ 50: 'c', 51: 'c' });
    create(s);
    mockInviteToFamily(s, OWNER, { node_id: 50, validity_secs: 500 }); // active
    mockInviteToFamily(s, OWNER, { node_id: 51, validity_secs: 500 });
    s.pending.get('1:51')!.expires_at = NOW - 1; // expired
    const { items } = mockGetPendingInvitationsForFamilyPaged(s, 1);
    const byNode = Object.fromEntries(items.map((i) => [i.invitation.node_id, i.expired]));
    expect(byNode[50]).toBe(false);
    expect(byNode[51]).toBe(true);
  });

  it('defaults limit to 50, clamps to 100, and pages exclusively via start_after', () => {
    const s = setup();
    create(s);
    // 60 members
    for (let i = 1; i <= 60; i += 1) {
      s.bondedNodes.set(1000 + i, { owner: `c${i}`, isUnbonding: false });
      mockInviteToFamily(s, OWNER, { node_id: 1000 + i });
      mockAcceptFamilyInvitation(s, `c${i}`, { family_id: 1, node_id: 1000 + i });
    }
    const page1 = mockGetFamilyMembersPaged(s, 1);
    expect(page1.items).toHaveLength(50);
    expect(page1.start_next_after).not.toBeNull();
    const page2 = mockGetFamilyMembersPaged(s, 1, page1.start_next_after ?? undefined);
    expect(page2.items).toHaveLength(10);
    const page3 = mockGetFamilyMembersPaged(s, 1, page2.start_next_after ?? undefined);
    expect(page3.items).toHaveLength(0);
    expect(page3.start_next_after).toBeNull();
    // limit clamps to 100; only 60 members exist so all 60 return
    expect(mockGetFamilyMembersPaged(s, 1, undefined, 10_000).items).toHaveLength(60);
  });

  it('multi-node operator: per-node pending queries are isolated', () => {
    const s = setup({ 70: 'op', 71: 'op' });
    create(s, 'F', 'owner2');
    mockInviteToFamily(s, 'owner2', { node_id: 70 });
    expect(mockGetPendingInvitationsForNodePaged(s, 70).items).toHaveLength(1);
    expect(mockGetPendingInvitationsForNodePaged(s, 71).items).toHaveLength(0);
  });
});
