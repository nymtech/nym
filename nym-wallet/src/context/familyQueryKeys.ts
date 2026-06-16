import type { NodeFamilyId, NodeId } from 'src/types/families';

const familyRoot = ['families'] as const;

/** TanStack Query key registry for node-families reads (mirrors `delegationQueryKeys`). */
export const familyQueryKeys = {
  all: familyRoot,
  config: [...familyRoot, 'config'] as const,
  /** Used when there is no owner address so React Query never caches `byOwner('')`. */
  byOwnerDisabled: [...familyRoot, 'byOwner', '__disabled__'] as const,
  byOwner: (owner: string) => [...familyRoot, 'byOwner', owner] as const,
  byId: (familyId: NodeFamilyId) => [...familyRoot, 'byId', familyId] as const,
  operatorInvites: (nodeId: NodeId) => [...familyRoot, 'operatorInvites', nodeId] as const,
  membership: (nodeId: NodeId) => [...familyRoot, 'membership', nodeId] as const,
  members: (familyId: NodeFamilyId) => [...familyRoot, 'members', familyId] as const,
  pendingForFamily: (familyId: NodeFamilyId) => [...familyRoot, 'pendingForFamily', familyId] as const,
  pendingForNode: (nodeId: NodeId) => [...familyRoot, 'pendingForNode', nodeId] as const,
  pastInvitationsForFamily: (familyId: NodeFamilyId) => [...familyRoot, 'pastInvitationsForFamily', familyId] as const,
  pastMembersForFamily: (familyId: NodeFamilyId) => [...familyRoot, 'pastMembersForFamily', familyId] as const,
};
