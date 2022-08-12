import type { DecCoin } from "./DecCoin";
import type { VestingPeriod } from "./VestingPeriod";

export interface VestingAccountInfo { owner_address: string, staking_address: string | null, start_time: bigint, periods: Array<VestingPeriod>, amount: DecCoin, }