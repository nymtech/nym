/* eslint-disable @typescript-eslint/naming-convention */
import { createContext, useContext, useMemo } from 'react';
import { useQueries, useQuery } from '@tanstack/react-query';
import {
  AcceptFamilyInvitationArgs,
  CreateFamilyArgs,
  FamilyConfig,
  FamilyCursor,
  FamilyMemberSections,
  FamilyPagedResponse,
  FamilyTxResult,
  FAMILY_PAGE_MAX_LIMIT,
  InviteToFamilyArgs,
  KickFromFamilyArgs,
  LeaveFamilyArgs,
  NodeFamily,
  NodeFamilyId,
  NodeFamilyMembershipResponse,
  NodeId,
  OperatorInviteView,
  PastFamilyInvitation,
  PastFamilyMember,
  PendingFamilyInvitationDetails,
  RejectFamilyInvitationArgs,
  RevokeFamilyInvitationArgs,
  UpdateFamilyArgs,
} from 'src/types/families';
import * as familyRequests from 'src/requests/families';
import { familyQueryKeys } from './familyQueryKeys';
import { deriveMemberSections } from './familyMemberSections';

/**
 * Read functions the context exposes. Swapping this object (real Tauri requests
 * vs. the mock) is the single seam that lets every read hook and the aggregator
 * work unchanged under Storybook/tests (design D3).
 */
export interface FamilyQueries {
  getFamilyConfig: () => Promise<FamilyConfig>;
  getFamilyById: (familyId: NodeFamilyId) => Promise<NodeFamily | null>;
  getFamilyByOwner: (owner: string) => Promise<NodeFamily | null>;
  getFamilyMembership: (nodeId: NodeId) => Promise<NodeFamilyMembershipResponse>;
  getFamilyMembersPaged: (
    familyId: NodeFamilyId,
    startAfter?: FamilyCursor,
    limit?: number,
  ) => Promise<FamilyPagedResponse<{ node_id: NodeId; joined_at: number }>>;
  getPendingInvitationsForFamilyPaged: (
    familyId: NodeFamilyId,
    startAfter?: FamilyCursor,
    limit?: number,
  ) => Promise<FamilyPagedResponse<PendingFamilyInvitationDetails>>;
  getPendingInvitationsForNodePaged: (
    nodeId: NodeId,
    startAfter?: FamilyCursor,
    limit?: number,
  ) => Promise<FamilyPagedResponse<PendingFamilyInvitationDetails>>;
  getPastInvitationsForFamilyPaged: (
    familyId: NodeFamilyId,
    startAfter?: FamilyCursor,
    limit?: number,
  ) => Promise<FamilyPagedResponse<PastFamilyInvitation>>;
  getPastMembersForFamilyPaged: (
    familyId: NodeFamilyId,
    startAfter?: FamilyCursor,
    limit?: number,
  ) => Promise<FamilyPagedResponse<PastFamilyMember>>;
}

export type FamilyExecutingAction =
  | 'create'
  | 'update'
  | 'disband'
  | 'invite'
  | 'revoke'
  | 'kick'
  | 'accept'
  | 'reject'
  | 'leave'
  | null;

export interface TFamiliesContext {
  /** Connected wallet address (the prospective/actual family owner). */
  ownerAddress?: string;
  /** Node ids this account controls (drives the operator invite view). */
  controlledNodeIds: NodeId[];
  /** Current chain time (unix seconds) used for TTL/expiry display. */
  nowSecs: number;
  /** Read seam, consumed by the read hooks below. */
  queries: FamilyQueries;
  /** True while an execute call is in flight. */
  isExecuting: boolean;
  /** Which execute call is in flight, so we can show per-button loading states. */
  executingAction: FamilyExecutingAction;
  /** Last execute error message (cleared via `clearError`). */
  error?: string;
  clearError: () => void;
  createFamily: (args: CreateFamilyArgs) => Promise<FamilyTxResult>;
  updateFamily: (args: UpdateFamilyArgs) => Promise<FamilyTxResult>;
  disbandFamily: () => Promise<FamilyTxResult>;
  inviteToFamily: (args: InviteToFamilyArgs) => Promise<FamilyTxResult>;
  revokeFamilyInvitation: (args: RevokeFamilyInvitationArgs) => Promise<FamilyTxResult>;
  kickFromFamily: (args: KickFromFamilyArgs) => Promise<FamilyTxResult>;
  acceptFamilyInvitation: (args: AcceptFamilyInvitationArgs) => Promise<FamilyTxResult>;
  rejectFamilyInvitation: (args: RejectFamilyInvitationArgs) => Promise<FamilyTxResult>;
  leaveFamily: (args: LeaveFamilyArgs) => Promise<FamilyTxResult>;
  /** Invalidate every families query (used after an execute call). */
  refreshAll: () => Promise<void>;
}

const notImplemented = async (): Promise<never> => {
  throw new Error('FamiliesContext not implemented');
};

/** Real Tauri-backed read functions; the mock provider swaps in its own. */
export const defaultQueries: FamilyQueries = {
  getFamilyConfig: familyRequests.getFamilyConfig,
  getFamilyById: familyRequests.getFamilyById,
  getFamilyByOwner: familyRequests.getFamilyByOwner,
  getFamilyMembership: familyRequests.getFamilyMembership,
  getFamilyMembersPaged: familyRequests.getFamilyMembersPaged,
  getPendingInvitationsForFamilyPaged: familyRequests.getPendingInvitationsForFamilyPaged,
  getPendingInvitationsForNodePaged: familyRequests.getPendingInvitationsForNodePaged,
  getPastInvitationsForFamilyPaged: familyRequests.getPastInvitationsForFamilyPaged,
  getPastMembersForFamilyPaged: familyRequests.getPastMembersForFamilyPaged,
};

export const FamiliesContext = createContext<TFamiliesContext>({
  controlledNodeIds: [],
  nowSecs: 0,
  queries: defaultQueries,
  isExecuting: false,
  executingAction: null,
  clearError: () => undefined,
  createFamily: notImplemented,
  updateFamily: notImplemented,
  disbandFamily: notImplemented,
  inviteToFamily: notImplemented,
  revokeFamilyInvitation: notImplemented,
  kickFromFamily: notImplemented,
  acceptFamilyInvitation: notImplemented,
  rejectFamilyInvitation: notImplemented,
  leaveFamily: notImplemented,
  refreshAll: async () => undefined,
});

export const useFamiliesContext = () => useContext<TFamiliesContext>(FamiliesContext);

// ---------------------------------------------------------------------------
// Pagination helper that walks the contract's exclusive `start_after` cursor to
// the end of a section, exercising start_after/start_next_after page-by-page.
// ---------------------------------------------------------------------------

const PAGE_SAFETY_BOUND = 1000;

async function fetchAllPages<T>(
  fetchPage: (startAfter?: FamilyCursor, limit?: number) => Promise<FamilyPagedResponse<T>>,
): Promise<T[]> {
  const out: T[] = [];
  let cursor: FamilyCursor = null;
  for (let i = 0; i < PAGE_SAFETY_BOUND; i += 1) {
    // eslint-disable-next-line no-await-in-loop
    const page = await fetchPage(cursor ?? undefined, FAMILY_PAGE_MAX_LIMIT);
    out.push(...page.items);
    if (!page.start_next_after || page.items.length === 0) break;
    cursor = page.start_next_after;
  }
  return out;
}

const READ_STALE_TIME = 60 * 1000;
const PENDING_STALE_TIME = 0;

// ---------------------------------------------------------------------------
// Read hooks (TanStack Query)
// ---------------------------------------------------------------------------

export const useFamilyConfig = () => {
  const { queries } = useFamiliesContext();
  return useQuery({
    queryKey: familyQueryKeys.config,
    queryFn: () => queries.getFamilyConfig(),
    staleTime: READ_STALE_TIME,
  });
};

export const useFamilyByOwner = (owner?: string) => {
  const { queries, ownerAddress } = useFamiliesContext();
  const addr = owner ?? ownerAddress;
  return useQuery({
    queryKey: addr ? familyQueryKeys.byOwner(addr) : familyQueryKeys.byOwnerDisabled,
    queryFn: () => queries.getFamilyByOwner(addr as string),
    enabled: Boolean(addr),
    staleTime: READ_STALE_TIME,
  });
};

const sameOwner = (a?: string, b?: string): boolean =>
  a !== undefined && b !== undefined && a.toLowerCase() === b.toLowerCase();

export const useFamilyById = (familyId?: NodeFamilyId) => {
  const { queries } = useFamiliesContext();
  return useQuery({
    queryKey: familyQueryKeys.byId(familyId ?? -1),
    queryFn: () => queries.getFamilyById(familyId as NodeFamilyId),
    enabled: familyId !== undefined,
    staleTime: READ_STALE_TIME,
  });
};

/**
 * Operator view: pending invitations addressed to a node, each resolved with its
 * family's name + owner so the invite card can render the full detail.
 */
export const useOperatorNodeInvites = (nodeId?: NodeId) => {
  const { queries } = useFamiliesContext();
  return useQuery<OperatorInviteView[]>({
    queryKey: familyQueryKeys.operatorInvites(nodeId ?? -1),
    enabled: nodeId !== undefined,
    staleTime: READ_STALE_TIME,
    queryFn: async () => {
      const pending = await fetchAllPages((startAfter, limit) =>
        queries.getPendingInvitationsForNodePaged(nodeId as NodeId, startAfter, limit),
      );
      const views = await Promise.all(
        pending.map(async (d) => {
          const family = await queries.getFamilyById(d.invitation.family_id);
          return {
            family_id: d.invitation.family_id,
            family_name: family?.name ?? `Family #${d.invitation.family_id}`,
            owner_address: family?.owner ?? '',
            expires_at: d.invitation.expires_at,
            expired: d.expired,
          } satisfies OperatorInviteView;
        }),
      );
      return views;
    },
  });
};

export const useFamilyMembership = (nodeId?: NodeId) => {
  const { queries } = useFamiliesContext();
  return useQuery({
    queryKey: familyQueryKeys.membership(nodeId ?? -1),
    queryFn: () => queries.getFamilyMembership(nodeId as NodeId),
    enabled: nodeId !== undefined,
    staleTime: READ_STALE_TIME,
  });
};

/**
 * The family this wallet owns, from `getFamilyByOwner`, with a fallback via the
 * bonded node's membership when the node belongs to a family this wallet created.
 */
export const useOwnedFamily = () => {
  const { ownerAddress, controlledNodeIds } = useFamiliesContext();
  const byOwner = useFamilyByOwner();
  const nodeId = controlledNodeIds[0];
  const membership = useFamilyMembership(nodeId);
  const membershipFamilyId = membership.data?.family_id ?? undefined;
  const needsMembershipLookup = byOwner.data === undefined && membershipFamilyId !== undefined;
  const byMembership = useFamilyById(needsMembershipLookup ? membershipFamilyId : undefined);

  const family = useMemo((): NodeFamily | null => {
    if (byOwner.data) return byOwner.data;
    const candidate = byMembership.data;
    if (candidate && sameOwner(candidate.owner, ownerAddress)) return candidate;
    return null;
  }, [byOwner.data, byMembership.data, ownerAddress]);

  const isPending =
    byOwner.isPending || (needsMembershipLookup && byMembership.isPending && byOwner.data === undefined);

  return {
    family,
    isPending,
    isError: byOwner.isError || byMembership.isError,
  };
};

export const useFamilyMembers = (familyId?: NodeFamilyId) => {
  const { queries } = useFamiliesContext();
  return useQuery({
    queryKey: familyQueryKeys.members(familyId ?? -1),
    queryFn: () =>
      fetchAllPages((startAfter, limit) => queries.getFamilyMembersPaged(familyId as NodeFamilyId, startAfter, limit)),
    enabled: familyId !== undefined,
    staleTime: READ_STALE_TIME,
  });
};

export const usePendingInvitationsForFamily = (familyId?: NodeFamilyId) => {
  const { queries } = useFamiliesContext();
  return useQuery({
    queryKey: familyQueryKeys.pendingForFamily(familyId ?? -1),
    queryFn: () =>
      fetchAllPages((startAfter, limit) =>
        queries.getPendingInvitationsForFamilyPaged(familyId as NodeFamilyId, startAfter, limit),
      ),
    enabled: familyId !== undefined,
    staleTime: PENDING_STALE_TIME,
  });
};

export const usePastInvitationsForFamily = (familyId?: NodeFamilyId) => {
  const { queries } = useFamiliesContext();
  return useQuery({
    queryKey: familyQueryKeys.pastInvitationsForFamily(familyId ?? -1),
    queryFn: () =>
      fetchAllPages((startAfter, limit) =>
        queries.getPastInvitationsForFamilyPaged(familyId as NodeFamilyId, startAfter, limit),
      ),
    enabled: familyId !== undefined,
    staleTime: READ_STALE_TIME,
  });
};

export const usePastMembersForFamily = (familyId?: NodeFamilyId) => {
  const { queries } = useFamiliesContext();
  return useQuery({
    queryKey: familyQueryKeys.pastMembersForFamily(familyId ?? -1),
    queryFn: () =>
      fetchAllPages((startAfter, limit) =>
        queries.getPastMembersForFamilyPaged(familyId as NodeFamilyId, startAfter, limit),
      ),
    enabled: familyId !== undefined,
    staleTime: READ_STALE_TIME,
  });
};

export const usePendingInvitationsForNode = (nodeId?: NodeId) => {
  const { queries } = useFamiliesContext();
  return useQuery({
    queryKey: familyQueryKeys.pendingForNode(nodeId ?? -1),
    queryFn: () =>
      fetchAllPages((startAfter, limit) =>
        queries.getPendingInvitationsForNodePaged(nodeId as NodeId, startAfter, limit),
      ),
    enabled: nodeId !== undefined,
    staleTime: READ_STALE_TIME,
  });
};

/**
 * How many invites are still waiting on a decision across the given nodes. We only
 * count live ones (expired invites can't be accepted, so they don't "need
 * addressing"). Shares the `pendingForNode` query cache so the notification badge
 * stays in lockstep with the invites view and refreshes on the same invalidation.
 */
export const usePendingInviteCountForNodes = (nodeIds: NodeId[]): number => {
  const { queries } = useFamiliesContext();
  const results = useQueries({
    queries: nodeIds.map((nodeId) => ({
      queryKey: familyQueryKeys.pendingForNode(nodeId),
      queryFn: () =>
        fetchAllPages((startAfter, limit) => queries.getPendingInvitationsForNodePaged(nodeId, startAfter, limit)),
      staleTime: PENDING_STALE_TIME,
    })),
  });

  return results.reduce((total, result) => total + (result.data?.filter((d) => !d.expired).length ?? 0), 0);
};

// ---------------------------------------------------------------------------
// Member-list aggregator: one section per contract query. Revoked past
// invitations are not surfaced (only Rejected ones populate the Rejected
// section), and any node that's currently joined is dropped from the
// rejected/removed history so stale rows don't stick around.
// ---------------------------------------------------------------------------

export interface UseFamilyMemberListResult {
  sections: FamilyMemberSections;
  isLoading: boolean;
  isError: boolean;
  refetch: () => void;
}

export const useFamilyMemberList = (familyId?: NodeFamilyId): UseFamilyMemberListResult => {
  const joined = useFamilyMembers(familyId);
  const pastInvitations = usePastInvitationsForFamily(familyId);
  const pastMembers = usePastMembersForFamily(familyId);

  const sections = useMemo<FamilyMemberSections>(
    () =>
      deriveMemberSections({
        pending: [],
        joined: joined.data ?? [],
        pastInvitations: pastInvitations.data ?? [],
        pastMembers: pastMembers.data ?? [],
      }),
    [joined.data, pastInvitations.data, pastMembers.data],
  );

  // A refetch (e.g. the invalidate-all after sending an invite) can transiently
  // fail without clearing the last good `data`. Only surface the hard error state
  // when there is genuinely nothing cached to show, so the list doesn't flash
  // "Failed to load" over data that is still present.
  const hasAnyData = joined.data !== undefined || pastInvitations.data !== undefined || pastMembers.data !== undefined;
  const anyError = joined.isError || pastInvitations.isError || pastMembers.isError;

  return {
    sections,
    isLoading: joined.isPending || pastInvitations.isPending || pastMembers.isPending,
    isError: anyError && !hasAnyData,
    refetch: () => {
      joined.refetch();
      pastInvitations.refetch();
      pastMembers.refetch();
    },
  };
};
