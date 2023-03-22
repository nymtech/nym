import { JsonObject } from '@cosmjs/cosmwasm-stargate';
import { Code, CodeDetails, Contract, ContractCodeHistoryEntry } from '@cosmjs/cosmwasm-stargate/build/cosmwasmclient';
import {
  Account,
  Block,
  Coin,
  DeliverTxResponse,
  IndexedTx,
  SearchTxFilter,
  SearchTxQuery,
  SequenceResponse,
} from '@cosmjs/stargate';
import {
  MixNodeRewarding,
  PagedMixNodeBondResponse,
  PagedMixNodeDetailsResponse,
  StakeSaturationResponse,
  UnbondedMixnodeResponse,
} from '@nymproject/types';

export type MixnetContractVersion = {
  build_timestamp: string;
  build_version: string;
  commit_sha: string;
  commit_timestamp: string;
  commit_branch: string;
  rustc_version: string;
};

export type PagedMixnodeResponse = {
  nodes: MixNodeBond[];
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

export type LayerDistribution = {
  gateways: number;
  layer1: number;
  layer2: number;
  layer3: number;
};

export type Delegation = {
  owner: string;
  node_identity: string;
  amount: Coin;
  block_height: number;
  proxy?: string;
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

export enum Layer {
  Gateway,
  One,
  Two,
  Three,
}

export type MixNodeBond = {
  owner: string;
  mix_node: MixNode;
  layer: Layer;
  bond_amount: Coin;
  total_delegation: Coin;
};

export type MixNode = {
  host: string;
  mix_port: number;
  verloc_port: number;
  http_api_port: number;
  sphinx_key: string;
  identity_key: string;
  version: string;
  profit_margin_percent: number;
};

export type GatewayBond = {
  owner: string;
  gateway: Gateway;

  bond_amount: Coin;
  total_delegation: Coin;
};

export type Gateway = {
  host: string;
  mix_port: number;
  clients_port: number;
  location: string;
  sphinx_key: string;
  identity_key: string;
  version: string;
};

export type SendRequest = {
  senderAddress: string;
  recipientAddress: string;
  transferAmount: readonly Coin[];
};

export interface SmartContractQuery {
  queryContractSmart(address: string, queryMsg: Record<string, unknown>): Promise<JsonObject>;
}
export interface ICosmWasmQuery {
  // methods exposed by `CosmWasmClient`
  getChainId(): Promise<string>;
  getHeight(): Promise<number>;
  getAccount(searchAddress: string): Promise<Account | null>;
  getSequence(address: string): Promise<SequenceResponse>;
  getBlock(height?: number): Promise<Block>;
  getBalance(address: string, searchDenom: string): Promise<Coin>;
  getTx(id: string): Promise<IndexedTx | null>;
  searchTx(query: SearchTxQuery, filter?: SearchTxFilter): Promise<readonly IndexedTx[]>;
  disconnect(): void;
  broadcastTx(tx: Uint8Array, timeoutMs?: number, pollIntervalMs?: number): Promise<DeliverTxResponse>;
  getCodes(): Promise<readonly Code[]>;
  getCodeDetails(codeId: number): Promise<CodeDetails>;
  getContracts(codeId: number): Promise<readonly string[]>;
  getContract(address: string): Promise<Contract>;
  getContractCodeHistory(address: string): Promise<readonly ContractCodeHistoryEntry[]>;
  queryContractRaw(address: string, key: Uint8Array): Promise<Uint8Array | null>;
  queryContractSmart(address: string, queryMsg: Record<string, unknown>): Promise<JsonObject>;
}

export interface INymdQuery {
  // nym-specific implemented inside NymQuerier
  getContractVersion(mixnetContractAddress: string): Promise<MixnetContractVersion>;
  getMixNodeBonds(
    mixnetContractAddress: string,
    limit?: number,
    startAfter?: string,
  ): Promise<PagedMixNodeBondResponse>;
  getMixNodesDetailed(
    mixnetContractAddress: string,
    limit?: number,
    startAfter?: string,
  ): Promise<PagedMixNodeDetailsResponse>;
  getGatewaysPaged(mixnetContractAddress: string, limit?: number, startAfter?: string): Promise<PagedGatewayResponse>;
  getOwnedMixnode(mixnetContractAddress: string, address: string): Promise<MixOwnershipResponse>;
  ownsGateway(mixnetContractAddress: string, address: string): Promise<GatewayOwnershipResponse>;
  getStateParams(mixnetContractAddress: string): Promise<ContractState>;
  getAllNetworkDelegationsPaged(
    mixnetContractAddress: string,
    limit?: number,
    startAfter?: [string, string],
  ): Promise<PagedAllDelegationsResponse>;
  getMixNodeDelegationsPaged(
    mixnetContractAddress: string,
    mix_id: number,
    limit?: number,
    startAfter?: string,
  ): Promise<PagedMixDelegationsResponse>;
  getDelegatorDelegationsPaged(
    mixnetContractAddress: string,
    delegator: string,
    limit?: number,
    startAfter?: string,
  ): Promise<PagedDelegatorDelegationsResponse>;
  getDelegationDetails(mixnetContractAddress: string, mix_id: number, delegator: string): Promise<Delegation>;
  getLayerDistribution(mixnetContractAddress: string): Promise<LayerDistribution>;
  getStakeSaturation(mixnetContractAddress: string, mixId: number): Promise<StakeSaturationResponse>;
  getUnbondedMixNodeInformation(mixnetContractAddress: string, mixId: number): Promise<UnbondedMixnodeResponse>;
  getMixnodeRewardingDetails(mixnetContractAddress: string, mixId: number): Promise<MixNodeRewarding>;
}

export interface IVestingQuerier {
  getVestingContractVersion(mixnetContractAddress: string): Promise<MixnetContractVersion>;
}

export interface MappedCoin {
  readonly denom: string;
  readonly fractionalDigits: number;
}

export interface CoinMap {
  readonly [key: string]: MappedCoin;
}

export interface ContractState {
  owner: string;
  rewarding_validator_address: string;
  vesting_contract_address: string;
  rewarding_denom: string;
  params: ContractStateParams;
}
