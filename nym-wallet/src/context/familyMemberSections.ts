import {
  FamilyMemberSections,
  NodeId,
  PastFamilyInvitation,
  PastFamilyMember,
  PendingFamilyInvitationDetails,
} from 'src/types/families';

export interface MemberSectionInputs {
  pending: PendingFamilyInvitationDetails[];
  joined: { node_id: NodeId; joined_at: number }[];
  pastInvitations: PastFamilyInvitation[];
  pastMembers: PastFamilyMember[];
}

/**
 * Pure status-derivation selector: four sections, each derived from a contract
 * query. Only `Rejected` past invitations populate the Rejected section;
 * `Revoked` ones are owner-side actions and aren't surfaced.
 *
 * Nodes currently in the `joined` set are filtered out of the `rejected` and
 * `removed` history: a node that rejected or was removed and later re-joined is
 * an active member, so its stale historical rows would only confuse the owner.
 */
export const deriveMemberSections = ({
  pending,
  joined,
  pastInvitations,
  pastMembers,
}: MemberSectionInputs): FamilyMemberSections => {
  const joinedIds = new Set(joined.map((m) => m.node_id));

  return {
    pending: pending.map((d) => ({
      section: 'pending',
      node_id: d.invitation.node_id,
      expires_at: d.invitation.expires_at,
      expired: d.expired,
    })),
    joined: joined.map((m) => ({
      section: 'joined',
      node_id: m.node_id,
      joined_at: m.joined_at,
    })),
    rejected: pastInvitations
      .filter((p) => p.status.kind === 'Rejected' && !joinedIds.has(p.invitation.node_id))
      .map((p) => ({
        section: 'rejected',
        node_id: p.invitation.node_id,
        rejected_at: p.status.at,
      })),
    removed: pastMembers
      .filter((m) => !joinedIds.has(m.node_id))
      .map((m) => ({
        section: 'removed',
        node_id: m.node_id,
        removed_at: m.removed_at,
      })),
  };
};
