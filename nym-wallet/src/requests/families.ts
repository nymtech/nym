/* eslint-disable @typescript-eslint/naming-convention */
import {
  AcceptFamilyInvitationArgs,
  CreateFamilyArgs,
  FamilyConfig,
  FamilyCursor,
  FamilyPagedResponse,
  FamilyTxResult,
  InviteToFamilyArgs,
  KickFromFamilyArgs,
  LeaveFamilyArgs,
  NodeFamily,
  NodeFamilyId,
  NodeFamilyMembershipResponse,
  NodeId,
  PastFamilyInvitation,
  PastFamilyMember,
  PendingFamilyInvitationDetails,
  RejectFamilyInvitationArgs,
  RevokeFamilyInvitationArgs,
  UpdateFamilyArgs,
} from 'src/types/families';
import { invokeWrapper } from './wrapper';

/**
 * Tauri IPC bindings for the node-families contract.
 *
 * Command names are the assumed Rust handler names; the Rust side lands with the
 * wiring task (tasks.md §9) and is verified on rebase (§9.5). Until then these
 * are exercised exclusively through the mock provider.
 */

// --- Execute messages -------------------------------------------------------

export const createFamily = async (args: CreateFamilyArgs) => invokeWrapper<FamilyTxResult>('create_family', args);

export const updateFamily = async (args: UpdateFamilyArgs) => invokeWrapper<FamilyTxResult>('update_family', args);

export const disbandFamily = async () => invokeWrapper<FamilyTxResult>('disband_family');

export const inviteToFamily = async (args: InviteToFamilyArgs) =>
  invokeWrapper<FamilyTxResult>('invite_to_family', args);

export const revokeFamilyInvitation = async (args: RevokeFamilyInvitationArgs) =>
  invokeWrapper<FamilyTxResult>('revoke_family_invitation', args);

export const kickFromFamily = async (args: KickFromFamilyArgs) =>
  invokeWrapper<FamilyTxResult>('kick_from_family', args);

export const acceptFamilyInvitation = async (args: AcceptFamilyInvitationArgs) =>
  invokeWrapper<FamilyTxResult>('accept_family_invitation', args);

export const rejectFamilyInvitation = async (args: RejectFamilyInvitationArgs) =>
  invokeWrapper<FamilyTxResult>('reject_family_invitation', args);

export const leaveFamily = async (args: LeaveFamilyArgs) => invokeWrapper<FamilyTxResult>('leave_family', args);

// --- Single-entity queries --------------------------------------------------

export const getFamilyById = async (familyId: NodeFamilyId) =>
  invokeWrapper<NodeFamily | null>('get_family_by_id', { familyId });

export const getFamilyByOwner = async (owner: string) =>
  invokeWrapper<NodeFamily | null>('get_family_by_owner', { owner });

export const getFamilyMembership = async (nodeId: NodeId) =>
  invokeWrapper<NodeFamilyMembershipResponse>('get_family_membership', { nodeId });

export const getFamilyConfig = async () => invokeWrapper<FamilyConfig>('get_family_config');

// --- Paginated queries ------------------------------------------------------

export const getFamilyMembersPaged = async (familyId: NodeFamilyId, startAfter?: FamilyCursor, limit?: number) =>
  invokeWrapper<FamilyPagedResponse<{ node_id: NodeId; joined_at: number }>>('get_family_members_paged', {
    familyId,
    startAfter,
    limit,
  });

export const getPendingInvitationsForFamilyPaged = async (
  familyId: NodeFamilyId,
  startAfter?: FamilyCursor,
  limit?: number,
) =>
  invokeWrapper<FamilyPagedResponse<PendingFamilyInvitationDetails>>('get_pending_invitations_for_family_paged', {
    familyId,
    startAfter,
    limit,
  });

export const getPendingInvitationsForNodePaged = async (nodeId: NodeId, startAfter?: FamilyCursor, limit?: number) =>
  invokeWrapper<FamilyPagedResponse<PendingFamilyInvitationDetails>>('get_pending_invitations_for_node_paged', {
    nodeId,
    startAfter,
    limit,
  });

export const getPastInvitationsForFamilyPaged = async (
  familyId: NodeFamilyId,
  startAfter?: FamilyCursor,
  limit?: number,
) =>
  invokeWrapper<FamilyPagedResponse<PastFamilyInvitation>>('get_past_invitations_for_family_paged', {
    familyId,
    startAfter,
    limit,
  });

export const getPastMembersForFamilyPaged = async (familyId: NodeFamilyId, startAfter?: FamilyCursor, limit?: number) =>
  invokeWrapper<FamilyPagedResponse<PastFamilyMember>>('get_past_members_for_family_paged', {
    familyId,
    startAfter,
    limit,
  });
