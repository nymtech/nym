import type { Coin } from './coin';

export interface DelegationResult {
  source_address: string;
  target_address: string;
  amount: Coin | null;
}
