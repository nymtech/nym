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
 * Pure status-derivation selector (design D4): four sections, each 1:1 with a
 * contract query. No cross-section dedup, no priority cascade. Only `Rejected`
 * past invitations populate the Rejected section — `Revoked` ones are owner-side
 * actions and are NOT surfaced. One row per record, so a node may legitimately
 * appear in more than one section.
 */
export const deriveMemberSections = ({
  pending,
  joined,
  pastInvitations,
  pastMembers,
}: MemberSectionInputs): FamilyMemberSections => ({
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
    .filter((p) => p.status.kind === 'Rejected')
    .map((p) => ({
      section: 'rejected',
      node_id: p.invitation.node_id,
      rejected_at: p.status.at,
    })),
  removed: pastMembers.map((m) => ({
    section: 'removed',
    node_id: m.node_id,
    removed_at: m.removed_at,
  })),
});
