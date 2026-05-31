/* eslint-disable @typescript-eslint/naming-convention */
import Big from 'big.js';
import { DecCoin } from '@nymproject/types';
import { FamilyError, FamilyErrorKind, isFamilyError } from 'src/types/families';

/** Byte length (matches the contract's `String::len` limit checks). */
export const byteLength = (s: string): number => new TextEncoder().encode(s).length;

/** Strip control characters / angle-bracket tags before submission. React escapes on render, but we neutralise eagerly. */
export const sanitizeInput = (s: string): string =>
  // eslint-disable-next-line no-control-regex
  s.replace(/[\u0000-\u001F\u007F]/g, '').replace(/[<>]/g, '');

export const formatCoin = (coin?: DecCoin): string => (coin ? `${coin.amount} ${coin.denom.toUpperCase()}` : '—');

export const truncateAddress = (addr: string, head = 8, tail = 6): string =>
  addr.length <= head + tail + 1 ? addr : `${addr.slice(0, head)}…${addr.slice(-tail)}`;

/** Human-readable remaining TTL, or "Expired". */
export const formatExpiry = (expiresAt: number, nowSecs: number): string => {
  const remaining = expiresAt - nowSecs;
  if (remaining <= 0) return 'Expired';
  if (remaining < 60) return `in ${remaining}s`;
  if (remaining < 3600) return `in ${Math.floor(remaining / 60)} min`;
  if (remaining < 86400) return `in ${Math.floor(remaining / 3600)}h`;
  return `in ${Math.floor(remaining / 86400)}d`;
};

/** True when balance is below fee + a gas headroom (best-effort, pre-submit). */
export const isInsufficientBalance = (balance: DecCoin | undefined, fee: DecCoin, gasHeadroom = '0.1'): boolean => {
  if (!balance) return false;
  try {
    return Big(balance.amount).lt(Big(fee.amount).plus(gasHeadroom));
  } catch {
    return false;
  }
};

const ERROR_MESSAGES: Record<FamilyErrorKind, string> = {
  InvalidFamilyCreationFee: 'The attached creation fee is incorrect.',
  InvalidDeposit: 'The attached funds are invalid for this operation.',
  FamilyNameAlreadyTaken: 'That family name is already taken.',
  FamilyNameTooLong: 'The family name is too long.',
  EmptyFamilyName: 'The family name cannot be empty after normalisation.',
  FamilyDescriptionTooLong: 'The family description is too long.',
  SenderAlreadyOwnsAFamily: 'You already own a family.',
  SenderDoesntOwnAFamily: 'You do not own a family.',
  NodeAlreadyInFamily: 'That node is already in a family.',
  AlreadyInFamily: 'Your node is already a member of a family.',
  NodeDoesntExist: 'That node does not exist or is unbonding.',
  PendingInvitationAlreadyExists: 'There is already a pending invitation for that node.',
  ZeroInvitationValidity: 'Invitation validity must be greater than zero.',
  InvitationExpired: 'That invitation has expired.',
  InvitationNotFound: 'No pending invitation was found.',
  FamilyNotEmpty: 'The family must be empty before it can be deleted.',
  FamilyNotFound: 'That family no longer exists.',
  SenderDoesntControlNode: 'You do not control that node.',
  NodeNotMemberOfFamily: 'That node is not a member of your family.',
  NodeNotInFamily: 'That node is not in any family.',
  UnauthorisedMixnetCallback: 'Unauthorised callback.',
};

export const familyErrorMessage = (e: unknown): string => {
  if (isFamilyError(e)) return ERROR_MESSAGES[e.kind] ?? e.message;
  if (e instanceof Error) return e.message;
  return String(e);
};

export type InviteWarning = 'already-in-family' | 'non-existent' | 'duplicate-pending';

/** Map an invite-time contract error to the spec's three warning states. */
export const inviteWarningFromError = (e: unknown): InviteWarning | undefined => {
  const kind: FamilyErrorKind | undefined = isFamilyError(e) ? (e as FamilyError).kind : undefined;
  switch (kind) {
    case 'NodeAlreadyInFamily':
      return 'already-in-family';
    case 'NodeDoesntExist':
      return 'non-existent';
    case 'PendingInvitationAlreadyExists':
      return 'duplicate-pending';
    default:
      return undefined;
  }
};

export const INVITE_WARNING_MESSAGES: Record<InviteWarning, string> = {
  'already-in-family': 'This node is already in a family — the invite was not sent.',
  'non-existent': 'This node does not exist or is unbonding — the invite was not sent.',
  'duplicate-pending': 'This node already has a pending invite from your family.',
};
