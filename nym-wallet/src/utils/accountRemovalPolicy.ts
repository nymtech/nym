import { AccountEntry } from '@nymproject/types';

/** Advisory UI pre-check only; backend removal rules in `remove_account_for_password` are authoritative. */
export type AccountRemovalBlockReason = 'active_account' | 'last_account';

export function getAccountRemovalBlockReason(
  accounts: AccountEntry[],
  selectedAccountId: string | undefined,
  targetAccountId: string,
): AccountRemovalBlockReason | null {
  if (accounts.length <= 1) {
    return 'last_account';
  }
  if (selectedAccountId === targetAccountId) {
    return 'active_account';
  }
  return null;
}

export function canRemoveAccount(
  accounts: AccountEntry[],
  selectedAccountId: string | undefined,
  targetAccountId: string,
): boolean {
  return getAccountRemovalBlockReason(accounts, selectedAccountId, targetAccountId) === null;
}

export function getAccountRemovalBlockMessage(reason: AccountRemovalBlockReason): string {
  if (reason === 'last_account') {
    return 'You cannot remove your only stored account here. To reset the wallet entirely, sign out and remove your saved wallet file after backing it up.';
  }
  return 'Switch to another account before removing this one.';
}
