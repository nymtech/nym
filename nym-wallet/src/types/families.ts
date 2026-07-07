/* eslint-disable @typescript-eslint/naming-convention */
import { DecCoin, TransactionExecuteResult } from '@nymproject/types';

/**
 * Wallet-facing types for the `node-families-contract` capability.
 *
 * These mirror the contract data types defined in
 * `openspec/specs/node-families-contract/spec.md`. Field names are kept in the
 * contract's snake_case so request/IPC payloads map 1:1. Status unions and the
 * error set are flattened into wallet-friendly discriminated unions; the request
 * layer is responsible for translating the contract's serde envelope into these
 * shapes (verified on rebase per tasks.md 9.5).
 */

/** u64 on-chain; safe as a JS number for wallet-scale ids. `0` is the "no family" sentinel. */
export type NodeFamilyId = number;
/** Mixnet node id (u32 on-chain). */
export type NodeId = number;
/** Unix timestamp in seconds (block time). */
export type UnixSeconds = number;

/** Runtime config, read from chain — never hardcode the fee or limits. */
export interface FamilyConfig {
  create_family_fee: DecCoin;
  /** Byte-length limit (String::len) on the family name. */
  family_name_length_limit: number;
  /** Byte-length limit on the family description. */
  family_description_length_limit: number;
  default_invitation_validity_secs: number;
}

export interface NodeFamily {
  id: NodeFamilyId;
  name: string;
  description: string;
  /** ASCII-normalised canonical name (globally unique among live families). */
  normalised_name: string;
  members: number;
  created_at: UnixSeconds;
  paid_fee: DecCoin;
  owner: string;
}

export interface FamilyMembership {
  family_id: NodeFamilyId;
  joined_at: UnixSeconds;
}

export interface FamilyInvitation {
  family_id: NodeFamilyId;
  node_id: NodeId;
  expires_at: UnixSeconds;
}

/** A pending invitation stamped with the contract's live `expired` flag (`now >= expires_at`). */
export interface PendingFamilyInvitationDetails {
  invitation: FamilyInvitation;
  expired: boolean;
}

export type PastInvitationStatusKind = 'Accepted' | 'Rejected' | 'Revoked' | 'Expired';

/** Terminal status of an archived invitation. `Revoked` is owner-side only; `Expired` is set when a stale invite is superseded by re-invite. */
export interface PastInvitationStatus {
  kind: PastInvitationStatusKind;
  at: UnixSeconds;
}

export interface PastFamilyInvitation {
  invitation: FamilyInvitation;
  status: PastInvitationStatus;
}

export interface PastFamilyMember {
  family_id: NodeFamilyId;
  node_id: NodeId;
  removed_at: UnixSeconds;
}

// ---------------------------------------------------------------------------
// Execute message args
// ---------------------------------------------------------------------------

export interface CreateFamilyArgs {
  name: string;
  description: string;
  /** The configured `create_family_fee`, attached as funds. */
  fee: DecCoin;
}

/** `None`/`undefined` means "leave unchanged"; a string sets the field. */
export interface UpdateFamilyArgs {
  updated_name?: string | null;
  updated_description?: string | null;
}

export interface InviteToFamilyArgs {
  node_id: NodeId;
  /** Falls back to `Config::default_invitation_validity_secs` when omitted. */
  validity_secs?: number | null;
}

export interface RevokeFamilyInvitationArgs {
  node_id: NodeId;
}

export interface KickFromFamilyArgs {
  node_id: NodeId;
}

export interface AcceptFamilyInvitationArgs {
  family_id: NodeFamilyId;
  node_id: NodeId;
}

export interface RejectFamilyInvitationArgs {
  family_id: NodeFamilyId;
  node_id: NodeId;
}

export interface LeaveFamilyArgs {
  node_id: NodeId;
}

// ---------------------------------------------------------------------------
// Query responses
// ---------------------------------------------------------------------------

/** Cursor for the contract's exclusive `start_after` pagination. */
export type FamilyCursor = number | [number, number] | null;

/** Generic shape for a paginated contract query response. */
export interface FamilyPagedResponse<T> {
  items: T[];
  /** Cursor of the last entry; `null` ends the list. */
  start_next_after: FamilyCursor;
}

export interface NodeFamilyMembershipResponse {
  node_id: NodeId;
  family_id: NodeFamilyId | null;
}

export const FAMILY_PAGE_DEFAULT_LIMIT = 50;
export const FAMILY_PAGE_MAX_LIMIT = 100;

// ---------------------------------------------------------------------------
// Member-list sections (D4: each maps 1:1 to a contract query, one row per record)
// ---------------------------------------------------------------------------

export type MemberListSectionKey = 'pending' | 'joined' | 'rejected' | 'removed';

export interface PendingMemberRow {
  section: 'pending';
  node_id: NodeId;
  expires_at: UnixSeconds;
  expired: boolean;
}

export interface JoinedMemberRow {
  section: 'joined';
  node_id: NodeId;
  joined_at: UnixSeconds;
}

export interface RejectedMemberRow {
  section: 'rejected';
  node_id: NodeId;
  rejected_at: UnixSeconds;
}

export interface RemovedMemberRow {
  section: 'removed';
  node_id: NodeId;
  removed_at: UnixSeconds;
}

export type MemberRow = PendingMemberRow | JoinedMemberRow | RejectedMemberRow | RemovedMemberRow;

/** A pending invitation addressed to one of the operator's nodes, resolved with its family details. */
export interface OperatorInviteView {
  family_id: NodeFamilyId;
  family_name: string;
  owner_address: string;
  expires_at: UnixSeconds;
  expired: boolean;
}

export interface FamilyMemberSections {
  pending: PendingMemberRow[];
  joined: JoinedMemberRow[];
  rejected: RejectedMemberRow[];
  removed: RemovedMemberRow[];
}

// ---------------------------------------------------------------------------
// Typed error set (mirrors NodeFamiliesContractError)
// ---------------------------------------------------------------------------

export type FamilyErrorKind =
  | 'InvalidFamilyCreationFee'
  | 'InvalidDeposit'
  | 'FamilyNameAlreadyTaken'
  | 'FamilyNameTooLong'
  | 'EmptyFamilyName'
  | 'FamilyDescriptionTooLong'
  | 'SenderAlreadyOwnsAFamily'
  | 'SenderDoesntOwnAFamily'
  | 'NodeAlreadyInFamily'
  | 'AlreadyInFamily'
  | 'NodeDoesntExist'
  | 'PendingInvitationAlreadyExists'
  | 'ZeroInvitationValidity'
  | 'InvitationExpired'
  | 'InvitationNotFound'
  | 'FamilyNotEmpty'
  | 'FamilyNotFound'
  | 'SenderDoesntControlNode'
  | 'NodeNotMemberOfFamily'
  | 'NodeNotInFamily'
  | 'UnauthorisedMixnetCallback';

/** Error thrown by the mock (and surfaced from the real IPC error string) so the UI can branch on `kind`. */
export class FamilyError extends Error {
  constructor(public kind: FamilyErrorKind, message?: string, public context?: Record<string, unknown>) {
    super(message ?? kind);
    this.name = 'FamilyError';
  }
}

export const isFamilyError = (e: unknown): e is FamilyError => e instanceof FamilyError;

// ---------------------------------------------------------------------------
// Events (stable public surface; mock execute returns carry these)
// ---------------------------------------------------------------------------

export type FamilyEventName =
  | 'family_creation'
  | 'family_update'
  | 'family_disband'
  | 'family_invitation'
  | 'family_invitation_revoked'
  | 'family_invitation_accepted'
  | 'family_invitation_rejected'
  | 'family_member_left'
  | 'family_member_kicked'
  | 'family_node_unbond_cleanup';

export interface FamilyEvent {
  ty: FamilyEventName;
  attributes: Record<string, string>;
}

/** TransactionExecuteResult augmented with the family event(s) the call emitted. */
export type FamilyTxResult = TransactionExecuteResult & { family_events?: FamilyEvent[] };
