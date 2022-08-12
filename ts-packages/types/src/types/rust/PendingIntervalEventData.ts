import type { IntervalRewardingParamsUpdate } from "./IntervalRewardingParamsUpdate";
import type { MixNodeCostParams } from "./MixNodeCostParams";

export type PendingIntervalEventData = { ChangeMixCostParams: { mix_id: number, new_costs: MixNodeCostParams, } } | { UpdateRewardingParams: { update: IntervalRewardingParamsUpdate, } } | { UpdateIntervalConfig: { epochs_in_interval: number, epoch_duration_secs: bigint, } };