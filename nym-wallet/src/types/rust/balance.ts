import type { Coin } from './coin';

export interface Balance {
  coin: Coin;
  printable_balance: string;
}
