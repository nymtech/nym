import type { DecCoin } from "./DecCoin";
import type { DelegationRecord } from "./DelegationRecord";
import type { MixNodeCostParams } from "./MixNodeCostParams";

export interface DelegationWithEverything { owner: string, mix_id: number, node_identity: string, amount: DecCoin, accumulated_by_delegates: DecCoin | null, accumulated_by_operator: DecCoin | null, block_height: bigint, delegated_on_iso_datetime: string, cost_params: MixNodeCostParams | null, avg_uptime_percent: number | null, stake_saturation: string | null, uses_vesting_contract_tokens: boolean, unclaimed_rewards: DecCoin | null, history: Array<DelegationRecord>, }