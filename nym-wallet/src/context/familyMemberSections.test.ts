import { PastFamilyInvitation, PastFamilyMember, PendingFamilyInvitationDetails } from 'src/types/families';
import { deriveMemberSections } from './familyMemberSections';

const pending: PendingFamilyInvitationDetails[] = [
  { invitation: { family_id: 1, node_id: 10, expires_at: 100 }, expired: false },
  { invitation: { family_id: 1, node_id: 11, expires_at: 50 }, expired: true },
];
const joined = [
  { node_id: 20, joined_at: 5 },
  { node_id: 21, joined_at: 6 },
];
const pastInvitations: PastFamilyInvitation[] = [
  { invitation: { family_id: 1, node_id: 30, expires_at: 0 }, status: { kind: 'Rejected', at: 7 } },
  { invitation: { family_id: 1, node_id: 31, expires_at: 0 }, status: { kind: 'Revoked', at: 8 } },
  { invitation: { family_id: 1, node_id: 32, expires_at: 0 }, status: { kind: 'Accepted', at: 9 } },
];
const pastMembers: PastFamilyMember[] = [{ family_id: 1, node_id: 40, removed_at: 12 }];

describe('deriveMemberSections', () => {
  const sections = deriveMemberSections({ pending, joined, pastInvitations, pastMembers });

  it('maps each section to its source query', () => {
    expect(sections.pending.map((r) => r.node_id)).toStrictEqual([10, 11]);
    expect(sections.joined.map((r) => r.node_id)).toStrictEqual([20, 21]);
    expect(sections.removed.map((r) => r.node_id)).toStrictEqual([40]);
  });

  it('surfaces only Rejected past invitations (not Revoked or Accepted)', () => {
    expect(sections.rejected.map((r) => r.node_id)).toStrictEqual([30]);
  });

  it('carries the expired flag and timestamps', () => {
    expect(sections.pending.find((r) => r.node_id === 11)?.expired).toBe(true);
    expect(sections.rejected[0].rejected_at).toBe(7);
    expect(sections.removed[0].removed_at).toBe(12);
  });

  it('drops a currently-joined node from the rejected/removed history', () => {
    // node 20 is currently Joined, but also has a past Rejected invite + Removed record
    const reJoined = deriveMemberSections({
      pending,
      joined,
      pastInvitations: [
        ...pastInvitations,
        { invitation: { family_id: 1, node_id: 20, expires_at: 0 }, status: { kind: 'Rejected', at: 4 } },
      ],
      pastMembers: [...pastMembers, { family_id: 1, node_id: 20, removed_at: 3 }],
    });
    expect(reJoined.joined.some((r) => r.node_id === 20)).toBe(true);
    expect(reJoined.rejected.some((r) => r.node_id === 20)).toBe(false);
    expect(reJoined.removed.some((r) => r.node_id === 20)).toBe(false);
  });
});
