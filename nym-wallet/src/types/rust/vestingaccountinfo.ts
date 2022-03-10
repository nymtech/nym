import type { Coin } from './coin';
import type { VestingPeriod } from './vestingperiod';

export interface VestingAccountInfo {
  owner_address: string;
  staking_address: string | null;
  start_time: bigint;
  periods: Array<VestingPeriod>;
  coin: Coin;
}
