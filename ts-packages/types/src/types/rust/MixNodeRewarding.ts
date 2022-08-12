import type { MixNodeCostParams } from "./MixNodeCostParams";

export interface MixNodeRewarding { cost_params: MixNodeCostParams, operator: string, delegates: string, total_unit_reward: string, unit_delegation: string, last_rewarded_epoch: number, unique_delegations: number, }