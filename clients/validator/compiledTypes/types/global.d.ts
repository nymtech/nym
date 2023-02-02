import { MixNode, DecCoin, GatewayBond, MixNodeBond, MixNodeDetails, Delegation, Coin, UnbondedMixnode } from './rust';
export declare type TNodeType = 'mixnode' | 'gateway';
export declare type TNodeOwnership = {
  hasOwnership: boolean;
  nodeType?: TNodeType;
};
export declare type TDelegation = {
  owner: string;
  node_identity: string;
  amount: DecCoin;
  block_height: number;
  proxy: string;
};
export declare type TPagedDelegations = {
  delegations: TDelegation[];
  start_next_after: string;
};
export declare type TMixnodeBondDetails = {
  pledge_amount: DecCoin;
  total_delegation: DecCoin;
  owner: string;
  layer: string;
  block_height: number;
  mix_node: MixNode;
  proxy: any;
};
export declare type MixnetContractVersion = {
  build_timestamp: string;
  build_version: string;
  commit_sha: string;
  commit_timestamp: string;
  commit_branch: string;
  rustc_version: string;
};
export declare type PagedMixNodeBondResponse = {
  nodes: MixNodeBond[];
  per_page: number;
  start_next_after?: string;
};
export declare type PagedMixNodeDetailsResponse = {
  nodes: MixNodeDetails[];
  per_page: number;
  start_next_after?: string;
};
export declare type PagedGatewayResponse = {
  nodes: GatewayBond[];
  per_page: number;
  start_next_after?: string;
};
export declare type MixOwnershipResponse = {
  address: string;
  mixnode?: MixNodeBond;
};
export declare type GatewayOwnershipResponse = {
  address: string;
  gateway?: GatewayBond;
};
export declare type ContractStateParams = {
  minimum_mixnode_pledge: string;
  minimum_gateway_pledge: string;
  mixnode_rewarded_set_size: number;
  mixnode_active_set_size: number;
};
export declare type PagedMixDelegationsResponse = {
  delegations: Delegation[];
  start_next_after?: string;
};
export declare type PagedDelegatorDelegationsResponse = {
  delegations: Delegation[];
  start_next_after?: string;
};
export declare type PagedAllDelegationsResponse = {
  delegations: Delegation[];
  start_next_after?: [string, string];
};
export declare type RewardingResult = {
  operator_reward: string;
  total_delegator_reward: string;
};
export declare type NodeRewardParams = {
  period_reward_pool: string;
  k: string;
  reward_blockstamp: number;
  circulating_supply: string;
  uptime: string;
  sybil_resistance_percent: number;
};
export declare type DelegatorRewardParams = {
  node_reward_params: NodeRewardParams;
  sigma: number;
  profit_margin: number;
  node_profit: number;
};
export declare type PendingDelegatorRewarding = {
  running_results: RewardingResult;
  next_start: string;
  rewarding_params: DelegatorRewardParams;
};
export declare type RewardingStatus =
  | {
      Complete: RewardingResult;
    }
  | {
      PendingNextDelegatorPage: PendingDelegatorRewarding;
    };
export declare type MixnodeRewardingStatusResponse = {
  status?: RewardingStatus;
};
export declare type SendRequest = {
  senderAddress: string;
  recipientAddress: string;
  transferAmount: readonly Coin[];
};
export declare type UnbondedMixnodeResponse = [mix_id: number, unbonded_info?: UnbondedMixnode];
export declare type PagedUnbondedMixnodesResponse = {
  nodes: UnbondedMixnodeResponse[];
  per_page: number;
  start_next_after: string;
};
export declare type MappedCoin = {
  readonly denom: string;
  readonly fractionalDigits: number;
};
export declare type LayerDistribution = {
  gateways: number;
  layer1: number;
  layer2: number;
  layer3: number;
};

export declare type IntervalRewardParams = {
  reward_pool: string;
  staking_supply: string;
  staking_supply_scale_factor: string;
  epoch_reward_budget: string;
  stake_saturation_point: string;
  sybil_resistance: string;
  active_set_work_factor: string;
  interval_pool_emission: string;
}

export declare type RewardingParams = {
  interval: IntervalRewardParams;
  rewarded_set_size: number;
  active_set_size: number;
};
//# sourceMappingURL=global.d.ts.map
