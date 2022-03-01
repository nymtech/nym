import { Coin } from "./coin";
import { VestingPeriod } from "./vestingperiod";

export interface VestingAccountInfo {
  owner_address: string;
  staking_address: string | null;
  start_time: bigint;
  periods: Array<VestingPeriod>;
  coin: Coin;
}