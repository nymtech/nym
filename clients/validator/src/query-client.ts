import { CosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import { Tendermint34Client } from '@cosmjs/tendermint-rpc';
import NymdQuerier from './nymd-querier';
import {
  ContractStateParams,
  Delegation,
  GatewayBond,
  GatewayOwnershipResponse,
  ICosmWasmQuery,
  INymdQuery,
  LayerDistribution,
  MixnetContractVersion,
  MixOwnershipResponse,
  PagedAllDelegationsResponse,
  PagedDelegatorDelegationsResponse,
  PagedGatewayResponse,
  PagedMixDelegationsResponse,
  PagedMixNodeBondResponse,
  PagedMixNodeDetailsResponse,
  PagedUnbondedMixnodesResponse,
  RewardingStatus,
  StakeSaturation,
  UnbondedMixnodeResponse,
} from './types';
import ValidatorApiQuerier, { IValidatorApiQuery } from './validator-api-querier';
import { MixNodeBond, MixNodeRewarding } from '@nymproject/types';

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
    mixIdentity: string,
    limit?: number,
    startAfter?: string,
  ): Promise<PagedMixDelegationsResponse>;
  getDelegatorDelegationsPaged(
    mixnetContractAddress: string,
    delegator: string,
    limit?: number,
    startAfter?: string,
  ): Promise<PagedDelegatorDelegationsResponse>;
  getDelegationDetails(mixnetContractAddress: string, mixIdentity: string, delegator: string): Promise<Delegation>;
  getLayerDistribution(mixnetContractAddress: string): Promise<LayerDistribution>;
  getRewardPool(mixnetContractAddress: string): Promise<string>;
  getCirculatingSupply(mixnetContractAddress: string): Promise<string>;
  getIntervalRewardPercent(mixnetContractAddress: string): Promise<number>;
  getSybilResistancePercent(mixnetContractAddress: string): Promise<number>;
  getRewardingStatus(
    mixnetContractAddress: string,
    mixIdentity: string,
    rewardingIntervalNonce: number,
  ): Promise<RewardingStatus>;
  getStakeSaturation(mixnetContractAddress: string, mixId: number): Promise<StakeSaturation>;
  getUnbondedMixNodeInformation(mixnetContractAddress: string, mixId: number): Promise<UnbondedMixnodeResponse>;
  getMixnodeRewardingDetails(mixnetContractAddress: string, mixId: number): Promise<MixNodeRewarding>;
}

export interface IQueryClient extends ICosmWasmQuery, INymdQuery, INymApiQuery {}

export default class QueryClient extends CosmWasmClient implements IQueryClient {
  private nymdQuerier: NymdQuerier;

  private nymApiQuerier: NymApiQuerier;

  private constructor(tmClient: Tendermint34Client, nymApiUrl: string) {
    super(tmClient);
    this.nymdQuerier = new NymdQuerier(this);
    this.nymApiQuerier = new NymApiQuerier(nymApiUrl);
  }

  public static async connectWithNym(nymdUrl: string, nymApiUrl: string): Promise<QueryClient> {
    const tmClient = await Tendermint34Client.connect(nymdUrl);
    return new QueryClient(tmClient, nymApiUrl);
  }

  getContractVersion(mixnetContractAddress: string): Promise<MixnetContractVersion> {
    return this.nymdQuerier.getContractVersion(mixnetContractAddress);
  }

  getMixNodeBonds(
    mixnetContractAddress: string,
    limit?: number,
    startAfter?: string,
  ): Promise<PagedMixNodeBondResponse> {
    return this.nymdQuerier.getMixNodeBonds(mixnetContractAddress, limit, startAfter);
  }

  getMixNodesDetailed(
    mixnetContractAddress: string,
    limit?: number,
    startAfter?: string,
  ): Promise<PagedMixNodeDetailsResponse> {
    return this.nymdQuerier.getMixNodesDetailed(mixnetContractAddress, limit, startAfter);
  }

  getStakeSaturation(mixnetContractAddress: string, mixId: number): Promise<StakeSaturation> {
    return this.nymdQuerier.getStakeSaturation(mixnetContractAddress, mixId);
  }

  getMixnodeRewardingDetails(mixnetContractAddress: string, mixId: number): Promise<MixNodeRewarding> {
    return this.nymdQuerier.getMixnodeRewardingDetails(mixnetContractAddress, mixId);
  }

  getGatewaysPaged(mixnetContractAddress: string, limit?: number, startAfter?: string): Promise<PagedGatewayResponse> {
    return this.nymdQuerier.getGatewaysPaged(mixnetContractAddress, limit, startAfter);
  }

  getStateParams(mixnetContractAddress: string): Promise<ContractStateParams> {
    return this.nymdQuerier.getStateParams(mixnetContractAddress);
  }

  getOwnedMixnode(mixnetContractAddress: string, address: string): Promise<MixOwnershipResponse> {
    return this.nymdQuerier.getOwnedMixnode(mixnetContractAddress, address);
  }

  ownsGateway(mixnetContractAddress: string, address: string): Promise<GatewayOwnershipResponse> {
    return this.nymdQuerier.ownsGateway(mixnetContractAddress, address);
  }

  getUnbondedMixNodes(
    mixnetContractAddress: string,
    limit?: number,
    startAfter?: string,
  ): Promise<PagedUnbondedMixnodesResponse> {
    return this.nymdQuerier.getUnbondedMixNodes(mixnetContractAddress, limit, startAfter);
  }

  getUnbondedMixNodeInformation(mixnetContractAddress: string, mixId: number): Promise<UnbondedMixnodeResponse> {
    return this.nymdQuerier.getUnbondedMixNodeInformation(mixnetContractAddress, mixId);
  }

  getAllNetworkDelegationsPaged(
    mixnetContractAddress: string,
    limit?: number,
    startAfter?: [string, string],
  ): Promise<PagedAllDelegationsResponse> {
    return this.nymdQuerier.getAllNetworkDelegationsPaged(mixnetContractAddress, limit, startAfter);
  }

  getMixNodeDelegationsPaged(
    mixnetContractAddress: string,
    mixIdentity: string,
    limit?: number,
    startAfter?: string,
  ): Promise<PagedMixDelegationsResponse> {
    return this.nymdQuerier.getMixNodeDelegationsPaged(mixnetContractAddress, mixIdentity, limit, startAfter);
  }

  getDelegatorDelegationsPaged(
    mixnetContractAddress: string,
    delegator: string,
    limit?: number,
    startAfter?: string,
  ): Promise<PagedDelegatorDelegationsResponse> {
    return this.nymdQuerier.getDelegatorDelegationsPaged(mixnetContractAddress, delegator, limit, startAfter);
  }

  getDelegationDetails(mixnetContractAddress: string, mixIdentity: string, delegator: string): Promise<Delegation> {
    return this.nymdQuerier.getDelegationDetails(mixnetContractAddress, mixIdentity, delegator);
  }

  getLayerDistribution(mixnetContractAddress: string): Promise<LayerDistribution> {
    return this.nymdQuerier.getLayerDistribution(mixnetContractAddress);
  }

  getRewardPool(mixnetContractAddress: string): Promise<string> {
    return this.nymdQuerier.getRewardPool(mixnetContractAddress);
  }

  getCirculatingSupply(mixnetContractAddress: string): Promise<string> {
    return this.nymdQuerier.getCirculatingSupply(mixnetContractAddress);
  }

  getIntervalRewardPercent(mixnetContractAddress: string): Promise<number> {
    return this.nymdQuerier.getIntervalRewardPercent(mixnetContractAddress);
  }

  getSybilResistancePercent(mixnetContractAddress: string): Promise<number> {
    return this.nymdQuerier.getSybilResistancePercent(mixnetContractAddress);
  }

  getRewardingStatus(
    mixnetContractAddress: string,
    mixIdentity: string,
    rewardingIntervalNonce: number,
  ): Promise<RewardingStatus> {
    return this.nymdQuerier.getRewardingStatus(mixnetContractAddress, mixIdentity, rewardingIntervalNonce);
  }

  getCachedGateways(): Promise<GatewayBond[]> {
    return this.nymApiQuerier.getCachedGateways();
  }

  getCachedMixnodes(): Promise<MixNodeBond[]> {
    return this.nymApiQuerier.getCachedMixnodes();
  }

  getActiveMixnodes(): Promise<MixNodeBond[]> {
    return this.nymApiQuerier.getActiveMixnodes();
  }

  getRewardedMixnodes(): Promise<MixNodeBond[]> {
    return this.nymApiQuerier.getRewardedMixnodes();
  }
}
