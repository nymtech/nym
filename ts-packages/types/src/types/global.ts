import { MixNode, DecCoin, GatewayBond, MixNodeBond, MixNodeDetails, Delegation, Coin, UnbondedMixnode } from './rust';

export type TNodeType = 'mixnode' | 'gateway';

export type TNodeOwnership = {
  hasOwnership: boolean;
  nodeType?: TNodeType;
};

export type TDelegation = {
  owner: string;
  node_identity: string;
  amount: DecCoin;
  block_height: number;
  proxy: string; // proxy address used to delegate the funds on behalf of anouther address
};

export type TPagedDelegations = {
  delegations: TDelegation[];
  start_next_after: string;
};

export type TMixnodeBondDetails = {
  pledge_amount: DecCoin;
  total_delegation: DecCoin;
  owner: string;
  layer: string;
  block_height: number;
  mix_node: MixNode;
  proxy: any;
};

export type MixnetContractVersion = {
  build_timestamp: string;
  build_version: string;
  commit_sha: string;
  commit_timestamp: string;
  commit_branch: string;
  rustc_version: string;
};

export type PagedMixNodeBondResponse = {
  nodes: MixNodeBond[];
  per_page: number;
  start_next_after?: string;
};

export type PagedMixNodeDetailsResponse = {
  nodes: MixNodeDetails[];
  per_page: number;
  start_next_after?: string;
};

export type PagedGatewayResponse = {
  nodes: GatewayBond[];
  per_page: number;
  start_next_after?: string;
};

export type MixOwnershipResponse = {
  address: string;
  mixnode?: MixNodeBond;
};

export type GatewayOwnershipResponse = {
  address: string;
  gateway?: GatewayBond;
};

export type ContractStateParams = {
  // ideally I'd want to define those as `number` rather than `string`, but
  // rust-side they are defined as Uint128 and that don't have
  // native javascript representations and therefore are interpreted as strings after deserialization
  minimum_mixnode_pledge: string;
  minimum_gateway_pledge: string;
  mixnode_rewarded_set_size: number;
  mixnode_active_set_size: number;
};

export type PagedMixDelegationsResponse = {
  delegations: Delegation[];
  start_next_after?: string;
};

export type PagedDelegatorDelegationsResponse = {
  delegations: Delegation[];
  start_next_after?: string;
};

export type PagedAllDelegationsResponse = {
  delegations: Delegation[];
  start_next_after?: [string, string];
};

export type RewardingResult = {
  operator_reward: string;
  total_delegator_reward: string;
};

export type NodeRewardParams = {
  period_reward_pool: string;
  k: string;
  reward_blockstamp: number;
  circulating_supply: string;
  uptime: string;
  sybil_resistance_percent: number;
};

export type DelegatorRewardParams = {
  node_reward_params: NodeRewardParams;
  sigma: number;
  profit_margin: number;
  node_profit: number;
};

export type PendingDelegatorRewarding = {
  running_results: RewardingResult;
  next_start: string;
  rewarding_params: DelegatorRewardParams;
};

export type RewardingStatus = { Complete: RewardingResult } | { PendingNextDelegatorPage: PendingDelegatorRewarding };

export type MixnodeRewardingStatusResponse = {
  status?: RewardingStatus;
};

export type SendRequest = {
  senderAddress: string;
  recipientAddress: string;
  transferAmount: readonly Coin[];
};

export type UnbondedMixnodeResponse = [mix_id: number, unbonded_info?: UnbondedMixnode];

export type PagedUnbondedMixnodesResponse = {
  nodes: UnbondedMixnodeResponse[];
  per_page: number;
  start_next_after: string;
};

export type MappedCoin = {
  readonly denom: string;
  readonly fractionalDigits: number;
};

export type LayerDistribution = {
  gateways: number;
  layer1: number;
  layer2: number;
  layer3: number;
};

export interface ContractState {
  owner: string;
  rewarding_validator_address: string;
  vesting_contract_address: string;
  rewarding_denom: string;
  params: ContractStateParams;
}

export type VestingAccountNode = {
  amount: Coin;
  block_time: string;
};

export interface VestAccounts {
  account_id: string;
  owner: string;
}

export interface VestingAccountsPaged {
  accounts: VestAccounts[];
  start_next_after: string;
}

export interface VestingAccountsCoinPaged {
  account_id: string;
  owner: string;
  still_vesting: Coin;
}

export interface DelegationTimes {
  account_id: number;
  delegation_timestamps: [];
  mix_id: number;
  owner: string;
}

export interface DelegationBlock {
  account_id: number;
  amount: string;
  block_timestamp: number;
  mix_id: number;
}

export interface Delegations {
  delegations: DelegationBlock[];
  start_next_after: string | null;
}
