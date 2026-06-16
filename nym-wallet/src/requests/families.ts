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
//
// Tauri maps JS camelCase argument keys to the Rust commands' snake_case
// parameters, so any multi-word arg must be passed camelCased here even though
// the `*Args` types keep the contract's snake_case field names (the mock and UI
// build them in that shape). Single-word args (`name`, `description`, `fee`,
// `owner`) need no remapping.

export const createFamily = async (args: CreateFamilyArgs) => invokeWrapper<FamilyTxResult>('create_family', args);

export const updateFamily = async (args: UpdateFamilyArgs) =>
  invokeWrapper<FamilyTxResult>('update_family', {
    updatedName: args.updated_name,
    updatedDescription: args.updated_description,
  });

export const disbandFamily = async () => invokeWrapper<FamilyTxResult>('disband_family');

export const inviteToFamily = async (args: InviteToFamilyArgs) =>
  invokeWrapper<FamilyTxResult>('invite_to_family', { nodeId: args.node_id, validitySecs: args.validity_secs });

export const revokeFamilyInvitation = async (args: RevokeFamilyInvitationArgs) =>
  invokeWrapper<FamilyTxResult>('revoke_family_invitation', { nodeId: args.node_id });

export const kickFromFamily = async (args: KickFromFamilyArgs) =>
  invokeWrapper<FamilyTxResult>('kick_from_family', { nodeId: args.node_id });

export const acceptFamilyInvitation = async (args: AcceptFamilyInvitationArgs) =>
  invokeWrapper<FamilyTxResult>('accept_family_invitation', { familyId: args.family_id, nodeId: args.node_id });

export const rejectFamilyInvitation = async (args: RejectFamilyInvitationArgs) =>
  invokeWrapper<FamilyTxResult>('reject_family_invitation', { familyId: args.family_id, nodeId: args.node_id });

export const leaveFamily = async (args: LeaveFamilyArgs) =>
  invokeWrapper<FamilyTxResult>('leave_family', { nodeId: args.node_id });

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
