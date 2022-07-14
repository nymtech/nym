import type { Account } from './Account';

export interface AccountWithMnemonic {
  account: Account;
  mnemonic: string;
}
