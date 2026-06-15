import { AccountEntry } from '@nymproject/types';
import { mapAccountRemovalError } from './accountRemovalErrors';

/**
 * Orchestrates the local + remote effects of removing a stored account so the sequence is testable
 * outside React: remove via IPC, then reload the stored accounts and return the refreshed list.
 * Backend errors are surfaced as a user-facing message via `mapAccountRemovalError`.
 */
export async function performAccountRemoval({
  account,
  password,
  removeAccount,
  reloadStoredAccounts,
}: {
  account: AccountEntry;
  password: string;
  removeAccount: (args: { password: string; accountName: string }) => Promise<void>;
  reloadStoredAccounts: () => Promise<AccountEntry[]>;
}): Promise<AccountEntry[]> {
  try {
    await removeAccount({ password, accountName: account.id });
    return await reloadStoredAccounts();
  } catch (e) {
    throw new Error(mapAccountRemovalError(e));
  }
}
