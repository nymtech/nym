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
  ContractStateParams,
  Delegation,
  GatewayOwnershipResponse,
  LayerDistribution,
  MappedCoin,
  MixnetContractVersion,
  MixNodeRewarding,
  MixOwnershipResponse,
  PagedAllDelegationsResponse,
  PagedDelegatorDelegationsResponse,
  PagedGatewayResponse,
  PagedMixDelegationsResponse,
  PagedMixNodeBondResponse,
  PagedMixNodeDetailsResponse,
  StakeSaturationResponse,
  UnbondedMixnodeResponse,
} from '@nymproject/types';

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
  getStateParams(mixnetContractAddress: string): Promise<ContractStateParams>;
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

export type CoinMap = {
  readonly [key: string]: MappedCoin;
};
